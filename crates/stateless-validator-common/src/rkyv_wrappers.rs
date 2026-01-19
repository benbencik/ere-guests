//! rkyv wrappers for ssz_types (VariableList, FixedVector).
//!
//! ssz_types doesn't support rkyv natively, so we provide wrappers that
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
use ssz_types::{FixedVector, VariableList};
use typenum::Unsigned;

/// Simple error wrapper for ssz_types errors which don't implement std::error::Error.
#[derive(Debug)]
struct SszError(String);

impl fmt::Display for SszError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::error::Error for SszError {}

/// Wrapper to serialize `VariableList<T, N>` as `Vec<T>`.
///
/// On serialization, the inner slice is serialized as a Vec.
/// On deserialization, the Vec is converted back to VariableList.
#[derive(Debug)]
pub struct AsVariableList;

impl<T, N> ArchiveWith<VariableList<T, N>> for AsVariableList
where
    T: Archive,
    N: Unsigned,
{
    type Archived = ArchivedVec<T::Archived>;
    type Resolver = VecResolver;

    fn resolve_with(
        field: &VariableList<T, N>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        // VariableList implements Deref<Target = [T]>
        ArchivedVec::resolve_from_slice(field.deref(), resolver, out);
    }
}

impl<T, N, S> SerializeWith<VariableList<T, N>, S> for AsVariableList
where
    T: Serialize<S>,
    N: Unsigned,
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize_with(
        field: &VariableList<T, N>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::serialize_from_slice(field.deref(), serializer)
    }
}

impl<T, N, D> DeserializeWith<ArchivedVec<T::Archived>, VariableList<T, N>, D> for AsVariableList
where
    T: Archive,
    T::Archived: Deserialize<T, D>,
    N: Unsigned,
    D: Fallible + ?Sized,
    D::Error: Source,
{
    fn deserialize_with(
        archived: &ArchivedVec<T::Archived>,
        deserializer: &mut D,
    ) -> Result<VariableList<T, N>, D::Error> {
        let vec: Vec<T> = Deserialize::<Vec<T>, D>::deserialize(archived, deserializer)?;
        // VariableList::new returns Err if vec.len() > N::to_usize()
        // This shouldn't happen if data was serialized correctly
        VariableList::new(vec).map_err(|e| <D::Error as Source>::new(SszError(format!("{e:?}"))))
    }
}

/// Wrapper to serialize `FixedVector<T, N>` as `Vec<T>`.
///
/// On serialization, the inner slice is serialized as a Vec.
/// On deserialization, the Vec is converted back to FixedVector.
#[derive(Debug)]
pub struct AsFixedVector;

impl<T, N> ArchiveWith<FixedVector<T, N>> for AsFixedVector
where
    T: Archive,
    N: Unsigned,
{
    type Archived = ArchivedVec<T::Archived>;
    type Resolver = VecResolver;

    fn resolve_with(
        field: &FixedVector<T, N>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        // FixedVector implements Deref<Target = [T]>
        ArchivedVec::resolve_from_slice(field.deref(), resolver, out);
    }
}

impl<T, N, S> SerializeWith<FixedVector<T, N>, S> for AsFixedVector
where
    T: Serialize<S>,
    N: Unsigned,
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize_with(
        field: &FixedVector<T, N>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::serialize_from_slice(field.deref(), serializer)
    }
}

impl<T, N, D> DeserializeWith<ArchivedVec<T::Archived>, FixedVector<T, N>, D> for AsFixedVector
where
    T: Archive + Clone + Default,
    T::Archived: Deserialize<T, D>,
    N: Unsigned,
    D: Fallible + ?Sized,
    D::Error: Source,
{
    fn deserialize_with(
        archived: &ArchivedVec<T::Archived>,
        deserializer: &mut D,
    ) -> Result<FixedVector<T, N>, D::Error> {
        let vec: Vec<T> = Deserialize::<Vec<T>, D>::deserialize(archived, deserializer)?;
        // FixedVector::new returns Err if vec.len() != N::to_usize()
        FixedVector::new(vec).map_err(|e| <D::Error as Source>::new(SszError(format!("{e:?}"))))
    }
}

/// Wrapper for nested VariableList types like `VariableList<VariableList<u8, M>, N>`.
///
/// This is used for fields like `Transactions = VariableList<Transaction, _>`
/// where `Transaction = VariableList<u8, _>`.
///
/// Serializes as `Vec<Vec<T>>` and reconstructs on deserialization.
#[derive(Debug)]
pub struct AsNestedVariableList;

impl<T, M, N> ArchiveWith<VariableList<VariableList<T, M>, N>> for AsNestedVariableList
where
    T: Archive,
    M: Unsigned,
    N: Unsigned,
{
    type Archived = ArchivedVec<ArchivedVec<T::Archived>>;
    type Resolver = VecResolver;

    fn resolve_with(
        field: &VariableList<VariableList<T, M>, N>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedVec::resolve_from_len(field.len(), resolver, out);
    }
}

impl<T, M, N, S> SerializeWith<VariableList<VariableList<T, M>, N>, S> for AsNestedVariableList
where
    T: Serialize<S>,
    M: Unsigned,
    N: Unsigned,
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize_with(
        field: &VariableList<VariableList<T, M>, N>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        // Convert to Vec<Vec<T>> and serialize that
        let vecs: Vec<&[T]> = field.iter().map(|inner| inner.deref()).collect();

        // We need to serialize as Vec<Vec<T>> which archives to ArchivedVec<ArchivedVec<T::Archived>>
        // But we have Vec<&[T]>, so we need to serialize each slice individually
        ArchivedVec::serialize_from_iter(
            vecs.iter().map(|slice| {
                // Each slice needs to be wrapped to serialize as ArchivedVec
                SliceAsVec(slice)
            }),
            serializer,
        )
    }
}

/// Helper wrapper to serialize a slice as ArchivedVec
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

impl<T, M, N, D>
    DeserializeWith<ArchivedVec<ArchivedVec<T::Archived>>, VariableList<VariableList<T, M>, N>, D>
    for AsNestedVariableList
where
    T: Archive,
    T::Archived: Deserialize<T, D>,
    M: Unsigned,
    N: Unsigned,
    D: Fallible + ?Sized,
    D::Error: Source,
{
    fn deserialize_with(
        archived: &ArchivedVec<ArchivedVec<T::Archived>>,
        deserializer: &mut D,
    ) -> Result<VariableList<VariableList<T, M>, N>, D::Error> {
        let mut outer = Vec::with_capacity(archived.len());
        for inner_archived in archived.iter() {
            let inner_vec: Vec<T> =
                Deserialize::<Vec<T>, D>::deserialize(inner_archived, deserializer)?;
            let inner = VariableList::new(inner_vec)
                .map_err(|e| <D::Error as Source>::new(SszError(format!("{e:?}"))))?;
            outer.push(inner);
        }
        VariableList::new(outer).map_err(|e| <D::Error as Source>::new(SszError(format!("{e:?}"))))
    }
}
