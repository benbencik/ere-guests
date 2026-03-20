//! rkyv wrappers for `libssz_types`.
//!
//! `libssz_types` doesn't support rkyv natively, so we provide wrappers that
//! serialize them as `Vec<T>` and reconstruct on deserialization.

use alloc::{format, string::String, vec::Vec};
use core::{fmt, ops::Deref};

use rkyv::{
    Archive, Deserialize, Place, Serialize,
    rancor::{Fallible, Source},
    ser::{Allocator, Writer},
    vec::{ArchivedVec, VecResolver},
    with::{ArchiveWith, DeserializeWith, SerializeWith},
};
use libssz_types::SszList;

/// Simple error wrapper for `libssz_types` errors which don't implement `std::error::Error`.
#[derive(Debug)]
struct SszError(String);

impl fmt::Display for SszError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::error::Error for SszError {}

/// Wrapper to serialize `SszList<T, N>` as `Vec<T>`.
///
/// On serialization, the inner slice is serialized as a Vec.
/// On deserialization, the Vec is converted back to `SszList`.
#[derive(Debug)]
pub struct AsVariableList;

impl<T, const N: usize> ArchiveWith<SszList<T, N>> for AsVariableList
where
    T: Archive,
{
    type Archived = ArchivedVec<T::Archived>;
    type Resolver = VecResolver;

    fn resolve_with(
        field: &SszList<T, N>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedVec::resolve_from_slice(field.deref(), resolver, out);
    }
}

impl<T, const N: usize, S> SerializeWith<SszList<T, N>, S> for AsVariableList
where
    T: Serialize<S>,
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize_with(
        field: &SszList<T, N>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::serialize_from_slice(field.deref(), serializer)
    }
}

impl<T, const N: usize, D> DeserializeWith<ArchivedVec<T::Archived>, SszList<T, N>, D>
    for AsVariableList
where
    T: Archive,
    T::Archived: Deserialize<T, D>,
    D: Fallible + ?Sized,
    D::Error: Source,
{
    fn deserialize_with(
        archived: &ArchivedVec<T::Archived>,
        deserializer: &mut D,
    ) -> Result<SszList<T, N>, D::Error> {
        let vec: Vec<T> = Deserialize::<Vec<T>, D>::deserialize(archived, deserializer)?;
        SszList::try_from(vec).map_err(|e| <D::Error as Source>::new(SszError(format!("{e:?}"))))
    }
}

/// Wrapper for nested `SszList` types like `SszList<SszList<u8, M>, N>`.
///
/// This is used for fields like `Transactions = SszList<Transaction, _>`
/// where `Transaction = SszList<u8, _>`.
///
/// Serializes as `Vec<Vec<T>>` and reconstructs on deserialization.
#[derive(Debug)]
pub struct AsNestedVariableList;

impl<T, const M: usize, const N: usize> ArchiveWith<SszList<SszList<T, M>, N>>
    for AsNestedVariableList
where
    T: Archive,
{
    type Archived = ArchivedVec<ArchivedVec<T::Archived>>;
    type Resolver = VecResolver;

    fn resolve_with(
        field: &SszList<SszList<T, M>, N>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedVec::resolve_from_len(field.len(), resolver, out);
    }
}

impl<T, const M: usize, const N: usize, S> SerializeWith<SszList<SszList<T, M>, N>, S>
    for AsNestedVariableList
where
    T: Serialize<S>,
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize_with(
        field: &SszList<SszList<T, M>, N>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        let vecs: Vec<&[T]> = field.iter().map(|inner| inner.deref()).collect();
        ArchivedVec::serialize_from_iter(
            vecs.iter().map(|slice| SliceAsVec(slice)),
            serializer,
        )
    }
}

/// Helper wrapper to serialize a slice as `ArchivedVec`.
struct SliceAsVec<'a, T>(&'a [T]);

impl<T: Archive> Archive for SliceAsVec<'_, T> {
    type Archived = ArchivedVec<T::Archived>;
    type Resolver = VecResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedVec::resolve_from_slice(self.0, resolver, out);
    }
}

impl<T: Serialize<S>, S: Fallible + Allocator + Writer + ?Sized> Serialize<S>
    for SliceAsVec<'_, T>
{
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::serialize_from_slice(self.0, serializer)
    }
}

impl<T, const M: usize, const N: usize, D>
    DeserializeWith<ArchivedVec<ArchivedVec<T::Archived>>, SszList<SszList<T, M>, N>, D>
    for AsNestedVariableList
where
    T: Archive,
    T::Archived: Deserialize<T, D>,
    D: Fallible + ?Sized,
    D::Error: Source,
{
    fn deserialize_with(
        archived: &ArchivedVec<ArchivedVec<T::Archived>>,
        deserializer: &mut D,
    ) -> Result<SszList<SszList<T, M>, N>, D::Error> {
        let mut outer = Vec::with_capacity(archived.len());
        for inner_archived in archived.iter() {
            let inner_vec: Vec<T> =
                Deserialize::<Vec<T>, D>::deserialize(inner_archived, deserializer)?;
            let inner = SszList::try_from(inner_vec)
                .map_err(|e| <D::Error as Source>::new(SszError(format!("{e:?}"))))?;
            outer.push(inner);
        }
        SszList::try_from(outer).map_err(|e| <D::Error as Source>::new(SszError(format!("{e:?}"))))
    }
}
