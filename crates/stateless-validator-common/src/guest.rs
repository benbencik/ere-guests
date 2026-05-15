//! Stateless validator common types and utilities for guest.

use alloc::vec::Vec;
use core::{
    error::Error,
    fmt::{self, Display},
    mem::size_of,
};

use ere_codec::{Decode, Encode};

const NEW_PAYLOAD_REQUEST_ROOT_SIZE: usize = 32;
const SUCCESSFUL_BLOCK_VALIDATION_SIZE: usize = 1;
const CHAIN_ID_SIZE: usize = size_of::<u64>();

/// Static size of [`StatelessValidatorOutput`].
pub const STATELESS_VALIDATOR_OUTPUT_SIZE: usize =
    NEW_PAYLOAD_REQUEST_ROOT_SIZE + SUCCESSFUL_BLOCK_VALIDATION_SIZE + CHAIN_ID_SIZE;

/// Output of stateless validator guest program.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)
)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StatelessValidatorOutput {
    /// New payload request root.
    pub new_payload_request_root: [u8; 32],
    /// Stateless validation is successful or not.
    pub successful_block_validation: bool,
    /// Chain ID from the stateless validation chain configuration.
    pub chain_id: u64,
}

impl StatelessValidatorOutput {
    /// Constructs a new [`StatelessValidatorOutput`].
    pub fn new(
        new_payload_request_root: [u8; 32],
        successful_block_validation: bool,
        chain_id: u64,
    ) -> Self {
        Self {
            new_payload_request_root,
            successful_block_validation,
            chain_id,
        }
    }

    /// Returns serialized output.
    pub fn serialize(&self) -> [u8; STATELESS_VALIDATOR_OUTPUT_SIZE] {
        let mut buf = [0; STATELESS_VALIDATOR_OUTPUT_SIZE];
        buf[0..32].copy_from_slice(&self.new_payload_request_root);
        buf[32] = self.successful_block_validation as u8;
        buf[33..41].copy_from_slice(&self.chain_id.to_le_bytes());
        buf
    }
}

impl Encode for StatelessValidatorOutput {
    type Error = core::convert::Infallible;

    fn encode_to_vec(&self) -> Result<Vec<u8>, Self::Error> {
        Ok(self.serialize().to_vec())
    }
}

/// Error returned when decoding a [`StatelessValidatorOutput`] fails.
#[derive(Debug)]
pub enum StatelessValidatorOutputDecodeError {
    /// Buffer length is not [`STATELESS_VALIDATOR_OUTPUT_SIZE`].
    InvalidLength {
        /// Actual length of the provided buffer.
        len: usize,
    },
    /// Byte at index 32 (successful-validation flag) is not `0` or `1`.
    InvalidSuccessulBit {
        /// The offending byte value.
        byte: u8,
    },
}

impl Display for StatelessValidatorOutputDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength { len } => write!(
                f,
                "buffer length {len} does not match STATELESS_VALIDATOR_OUTPUT_SIZE",
            ),
            Self::InvalidSuccessulBit { byte } => {
                write!(f, "successful-validation byte {byte} is not 0 or 1")
            }
        }
    }
}

impl Error for StatelessValidatorOutputDecodeError {}

impl Decode for StatelessValidatorOutput {
    type Error = StatelessValidatorOutputDecodeError;

    fn decode_from_slice(slice: &[u8]) -> Result<Self, Self::Error> {
        if slice.len() != STATELESS_VALIDATOR_OUTPUT_SIZE {
            return Err(StatelessValidatorOutputDecodeError::InvalidLength { len: slice.len() });
        }
        let successful_block_validation = match slice[32] {
            0 => false,
            1 => true,
            byte => {
                return Err(StatelessValidatorOutputDecodeError::InvalidSuccessulBit { byte });
            }
        };
        Ok(Self {
            new_payload_request_root: slice[..32].try_into().unwrap(),
            successful_block_validation,
            chain_id: u64::from_le_bytes(slice[33..41].try_into().unwrap()),
        })
    }
}
