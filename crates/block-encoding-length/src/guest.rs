//! [`Guest`] implementation for the block encoding length calculation.

use alloc::vec::Vec;
use core::ops::Deref;

use alloy_rlp::{Decodable, Encodable};
use guest::codec::impl_codec_by_bincode_legacy;
use libssz::SszEncode;
use reth_ethereum_primitives::Block;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[rustfmt::skip]
pub use guest::*;

mod block_ssz;

/// The encoding format used for the block encoding length calculation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum BlockEncodingFormat {
    /// RLP encoding format
    Rlp,
    /// SSZ encoding format
    Ssz,
}

/// Block wrapper encoded as RLP bytes inside the bincode guest input.
#[derive(Clone, Debug, Default)]
pub struct RethBlock(pub reth_ethereum_primitives::Block);

impl Serialize for RethBlock {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut encoded = Vec::new();
        self.0.encode(&mut encoded);
        encoded.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RethBlock {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = Vec::<u8>::deserialize(deserializer)?;
        let mut encoded = encoded.as_slice();
        let block = Block::decode(&mut encoded).map_err(serde::de::Error::custom)?;
        if !encoded.is_empty() {
            return Err(serde::de::Error::custom("trailing bytes after RLP block"));
        }
        Ok(Self(block))
    }
}

impl Deref for RethBlock {
    type Target = reth_ethereum_primitives::Block;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Input for the block encoding length calculation guest program.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockEncodingLengthInput {
    /// The block to calculate the encoding length for.
    pub block: RethBlock,
    /// The number of times to repeat the encoding length calculation.
    pub loop_count: u16,
    /// The encoding format to use.
    pub format: BlockEncodingFormat,
}

impl_codec_by_bincode_legacy!(BlockEncodingLengthInput);

/// [`Guest`] implementation for the block encoding length calculation.
#[derive(Debug, Clone)]
pub struct BlockEncodingLengthGuest;

impl Guest for BlockEncodingLengthGuest {
    type Input = BlockEncodingLengthInput;
    type Output = ();

    fn compute<P: Platform>(input: Self::Input) -> Self::Output {
        match input.format {
            BlockEncodingFormat::Rlp => {
                P::cycle_scope("block_encoding_length_calculation", || {
                    for _ in 0..input.loop_count {
                        Block::rlp_length_for(&input.block.header, &input.block.body);
                    }
                });
            }
            BlockEncodingFormat::Ssz => {
                let block: block_ssz::Block =
                    P::cycle_scope("block_format_conversion", || input.block.0.into());

                P::cycle_scope("block_encoding_length_calculation", || {
                    for _ in 0..input.loop_count {
                        block.encoded_len();
                    }
                });
            }
        }
    }
}
