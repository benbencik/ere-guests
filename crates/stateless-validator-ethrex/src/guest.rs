//! Stateless validator guest program.

use alloc::format;
use core::fmt::Debug;

use ere_io::rkyv::IoRkyv;
use ethrex_common::types::{block_execution_witness::ExecutionWitness, fee_config::FeeConfig};
use ethrex_guest_program::{execution::execution_program, input::ProgramInput};
use stateless_validator_common::new_payload_request::NewPayloadRequest;

use crate::new_payload_request::get_block_from_new_payload_request;

#[rustfmt::skip]
pub use {
    guest::*,
    stateless_validator_common::guest::StatelessValidatorOutput,
};

/// Input for the Ethrex stateless validator guest program.
#[derive(rkyv::Serialize, rkyv::Deserialize, rkyv::Archive)]
pub struct StatelessValidatorEthrexInput {
    /// New payload request data.
    pub new_payload_request: NewPayloadRequest,
    /// database containing all the data necessary to execute
    pub execution_witness: ExecutionWitness,
    /// value used to calculate base fee
    pub elasticity_multiplier: u64,
    /// Configuration for L2 fees used for each block
    pub fee_configs: Option<Vec<FeeConfig>>,
    #[cfg(feature = "l2")]
    /// KZG commitment to the blob data
    #[serde_as(as = "[_; 48]")]
    pub blob_commitment: blobs_bundle::Commitment,
    #[cfg(feature = "l2")]
    /// KZG opening for a challenge over the blob commitment
    #[serde_as(as = "[_; 48]")]
    pub blob_proof: blobs_bundle::Proof,
}

impl Clone for StatelessValidatorEthrexInput {
    fn clone(&self) -> Self {
        Self {
            new_payload_request: self.new_payload_request.clone(),
            execution_witness: self.execution_witness.clone(),
            elasticity_multiplier: self.elasticity_multiplier,
            fee_configs: self.fee_configs.clone(),
            #[cfg(feature = "l2")]
            blob_commitment: self.blob_commitment,
            #[cfg(feature = "l2")]
            blob_proof: self.blob_proof,
        }
    }
}

impl Debug for StatelessValidatorEthrexInput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        struct DebugExecutionWitness<'a>(&'a ExecutionWitness);

        impl Debug for DebugExecutionWitness<'_> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_struct("ExecutionWitness")
                    .field("codes", &self.0.codes)
                    .field("block_headers_bytes", &self.0.block_headers_bytes)
                    .field("first_block_number", &self.0.first_block_number)
                    .field("chain_config", &self.0.chain_config)
                    .field("state_trie_root", &self.0.state_trie_root)
                    .field("storage_trie_roots", &self.0.storage_trie_roots)
                    .field("keys", &self.0.keys)
                    .finish()
            }
        }

        f.debug_struct("StatelessValidatorEthrexInput")
            .field("new_payload_request", &self.new_payload_request)
            .field(
                "execution_witness",
                &DebugExecutionWitness(&self.execution_witness),
            )
            .field("elasticity_multiplier", &self.elasticity_multiplier)
            .field("fee_configs", &self.fee_configs)
            .finish()
    }
}

/// [`Io`] implementation of Ethrex stateless validator.
pub type StatelessValidatorEthrexIo =
    IoRkyv<StatelessValidatorEthrexInput, StatelessValidatorOutput>;

/// [`Guest`] implementation for Ethrex stateless validator.
#[derive(Debug, Clone)]
pub struct StatelessValidatorEthrexGuest;

impl Guest for StatelessValidatorEthrexGuest {
    type Io = StatelessValidatorEthrexIo;

    fn compute<P: Platform>(input: GuestInput<Self>) -> GuestOutput<Self> {
        let new_payload_request_root = input.new_payload_request.tree_hash_root();

        let block = match get_block_from_new_payload_request(input.new_payload_request) {
            Ok(block) => block,
            Err(err) => {
                P::print(&format!("Block construction failed: {err}\n"));
                return StatelessValidatorOutput::new(new_payload_request_root, false);
            }
        };
        let input = ProgramInput {
            blocks: vec![block],
            execution_witness: input.execution_witness,
            elasticity_multiplier: input.elasticity_multiplier,
            fee_configs: input.fee_configs,
            #[cfg(feature = "l2")]
            blob_commitment: input.blob_commitment,
            #[cfg(feature = "l2")]
            blob_proof: input.blob_proof,
        };

        let block_num = input.blocks[0].header.number;
        let res = P::cycle_scope("validation", || execution_program(input));

        match res {
            Ok(_) => {
                return StatelessValidatorOutput::new(new_payload_request_root, true);
            }
            Err(err) => {
                P::print(&format!("Block {} validation failed: {err}\n", block_num));
                return StatelessValidatorOutput::new(new_payload_request_root, false);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use stateless_validator_common::new_payload_request::{ExecutionPayloadV1, NewPayloadRequest};

    use crate::guest::{Io, StatelessValidatorEthrexIo, StatelessValidatorOutput};

    #[test]
    fn serialize_output() {
        let dummy_new_payload_request_root = NewPayloadRequest::new_bellatrix(ExecutionPayloadV1 {
            parent_hash: [1; 32],
            fee_recipient: [2; 20],
            state_root: [3; 32],
            receipts_root: [4; 32],
            logs_bloom: Default::default(),
            prev_randao: [5; 32],
            block_number: 1,
            gas_limit: 2,
            gas_used: 3,
            timestamp: 4,
            extra_data: Default::default(),
            base_fee_per_gas: [6; 32],
            block_hash: [7; 32],
            transactions: Default::default(),
        })
        .tree_hash_root();

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
