//! Stateless validator guest program.

use alloc::{sync::Arc, vec::Vec};
use core::fmt;

use ethrex_crypto::Crypto;
use ethrex_guest_program::execution::execution_program_eip8025_bytes;
use stateless_validator_common::guest::STATELESS_VALIDATOR_OUTPUT_SIZE;

#[rustfmt::skip]
pub use {
    guest::*,
    stateless_validator_common::guest::StatelessValidatorOutput,
};

/// [`Io`] implementation of the ethrex stateless validator.
#[derive(Debug, Clone, Copy, Default)]
pub struct StatelessValidatorEthrexIo;

impl Io for StatelessValidatorEthrexIo {
    type Input = Vec<u8>;
    type Output = StatelessValidatorOutput;
    type Error = StatelessValidatorEthrexIoError;

    fn serialize_input(input: &Self::Input) -> Result<Vec<u8>, Self::Error> {
        Ok(input.clone())
    }

    fn deserialize_input(bytes: &[u8]) -> Result<Self::Input, Self::Error> {
        Ok(bytes.to_vec())
    }

    fn serialize_output(output: &Self::Output) -> Result<Vec<u8>, Self::Error> {
        Ok(output.serialize().to_vec())
    }

    fn deserialize_output(bytes: &[u8]) -> Result<Self::Output, Self::Error> {
        if bytes.len() != STATELESS_VALIDATOR_OUTPUT_SIZE {
            return Err(StatelessValidatorEthrexIoError::InvalidOutputLength(
                bytes.len(),
            ));
        }

        let mut new_payload_request_root = [0; 32];
        new_payload_request_root.copy_from_slice(&bytes[..32]);
        Ok(StatelessValidatorOutput::new(
            new_payload_request_root,
            bytes[32] != 0,
        ))
    }
}

/// Errors returned by the custom ethrex guest `Io` implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatelessValidatorEthrexIoError {
    /// The guest output is always a fixed-width byte array.
    InvalidOutputLength(usize),
}

impl fmt::Display for StatelessValidatorEthrexIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOutputLength(len) => {
                write!(f, "invalid output length: expected 33 bytes, got {len}")
            }
        }
    }
}

impl core::error::Error for StatelessValidatorEthrexIoError {}

/// [`Guest`] implementation for Ethrex stateless validator.
#[derive(Debug, Clone)]
pub struct StatelessValidatorEthrexGuest;

impl Guest for StatelessValidatorEthrexGuest {
    type Io = StatelessValidatorEthrexIo;

    fn compute<P: Platform>(input_bytes: GuestInput<Self>) -> GuestOutput<Self> {
        Self::compute_inner::<P>(&input_bytes, crypto())
    }
}

impl StatelessValidatorEthrexGuest {
    fn compute_inner<P: Platform>(
        input_bytes: &[u8],
        crypto: Arc<dyn Crypto>,
    ) -> GuestOutput<Self> {
        let output = P::cycle_scope("run_validation", || {
            execution_program_eip8025_bytes(input_bytes, crypto)
                .unwrap_or_else(|err| panic!("invalid EIP-8025 input: {err}"))
        });

        StatelessValidatorOutput::new(output.new_payload_request_root, output.valid)
    }
}

#[allow(unreachable_code)]
fn crypto() -> Arc<dyn Crypto> {
    #[cfg(feature = "risc0")]
    return Arc::new(ethrex_guest_program::crypto::risc0::Risc0Crypto);
    #[cfg(feature = "sp1")]
    return Arc::new(ethrex_guest_program::crypto::sp1::Sp1Crypto);
    #[cfg(feature = "zisk")]
    return Arc::new(ethrex_guest_program::crypto::zisk::ZiskCrypto);
    #[cfg(not(any(feature = "risc0", feature = "sp1", feature = "zisk")))]
    return Arc::new(ethrex_guest_program::crypto::NativeCrypto);
}

#[cfg(test)]
mod test {
    use stateless_validator_common::new_payload_request::{
        ExecutionPayloadV1, NativeSha256Hasher, NewPayloadRequest, NewPayloadRequestBellatrix,
    };

    use crate::guest::{Io, StatelessValidatorEthrexIo, StatelessValidatorOutput};

    #[test]
    fn serialize_output() {
        let dummy_new_payload_request_root =
            NewPayloadRequest::Bellatrix(NewPayloadRequestBellatrix {
                execution_payload: ExecutionPayloadV1 {
                    parent_hash: [1; 32],
                    fee_recipient: [2; 20],
                    state_root: [3; 32],
                    receipts_root: [4; 32],
                    logs_bloom: [0; 256],
                    prev_randao: [5; 32],
                    block_number: 1,
                    gas_limit: 2,
                    gas_used: 3,
                    timestamp: 4,
                    extra_data: Default::default(),
                    base_fee_per_gas: [6; 32],
                    block_hash: [7; 32],
                    transactions: Default::default(),
                },
            })
            .tree_hash_root(&NativeSha256Hasher);

        for output in [
            StatelessValidatorOutput::new(dummy_new_payload_request_root, false),
            StatelessValidatorOutput::new(dummy_new_payload_request_root, true),
        ] {
            assert_eq!(
                StatelessValidatorEthrexIo::serialize_output(&output).unwrap(),
                output.serialize()
            );
        }
    }
}
