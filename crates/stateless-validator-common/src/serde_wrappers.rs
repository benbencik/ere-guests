//! Serde wrappers for byte arrays and `libssz_types` containers.
//!
//! Provides helpers for serializing fixed-size byte arrays using serde_with::Bytes,
//! which is efficient for binary formats and uses byte sequences for human-readable formats.

use alloc::{format, vec::Vec};
use core::ops::Deref;

use libssz_types::SszList;
use serde::{Deserializer, Serializer};
use serde_with::{DeserializeAs, SerializeAs};

/// Serialize a byte array using serde_with::Bytes.
pub mod bytes_array {
    use super::*;

    /// Serialize a fixed-size byte array.
    pub fn serialize<const N: usize, S: Serializer>(
        bytes: &[u8; N],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serde_with::Bytes::serialize_as(bytes, serializer)
    }

    /// Deserialize a fixed-size byte array.
    pub fn deserialize<'de, const N: usize, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[u8; N], D::Error> {
        serde_with::Bytes::deserialize_as(deserializer)
    }
}

/// Serialize an `SszList<T, N>` as `Vec<T>`.
pub mod ssz_list {
    use serde::{Deserialize, Serialize, de::Error};

    use super::*;

    /// Serialize an `SszList<T, N>` as a sequence.
    pub fn serialize<T, S: Serializer, const N: usize>(
        list: &SszList<T, N>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
    {
        list.deref().serialize(serializer)
    }

    /// Deserialize a sequence into `SszList<T, N>`.
    pub fn deserialize<'de, T, D: Deserializer<'de>, const N: usize>(
        deserializer: D,
    ) -> Result<SszList<T, N>, D::Error>
    where
        T: Deserialize<'de>,
    {
        let vec = Vec::<T>::deserialize(deserializer)?;
        SszList::try_from(vec).map_err(|err| D::Error::custom(format!("{err:?}")))
    }
}

/// Serialize an `SszList<SszList<T, M>, N>` as `Vec<Vec<T>>`.
pub mod nested_ssz_list {
    use serde::{Deserialize, Serialize, de::Error};

    use super::*;

    /// Serialize a nested `SszList` as a nested sequence.
    pub fn serialize<T, S: Serializer, const M: usize, const N: usize>(
        list: &SszList<SszList<T, M>, N>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
    {
        let nested: Vec<&[T]> = list.iter().map(|inner| inner.deref()).collect();
        nested.serialize(serializer)
    }

    /// Deserialize a nested sequence into `SszList<SszList<T, M>, N>`.
    pub fn deserialize<'de, T, D: Deserializer<'de>, const M: usize, const N: usize>(
        deserializer: D,
    ) -> Result<SszList<SszList<T, M>, N>, D::Error>
    where
        T: Deserialize<'de>,
    {
        let nested = Vec::<Vec<T>>::deserialize(deserializer)?;
        let mut outer = Vec::with_capacity(nested.len());
        for inner in nested {
            let inner =
                SszList::try_from(inner).map_err(|err| D::Error::custom(format!("{err:?}")))?;
            outer.push(inner);
        }
        SszList::try_from(outer).map_err(|err| D::Error::custom(format!("{err:?}")))
    }
}
