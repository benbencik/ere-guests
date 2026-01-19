//! Serde wrappers for byte arrays.
//!
//! Provides helpers for serializing fixed-size byte arrays using serde_with::Bytes,
//! which is efficient for binary formats and uses byte sequences for human-readable formats.

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
