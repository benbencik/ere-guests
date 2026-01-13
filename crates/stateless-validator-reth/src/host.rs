//! Implementations for host environment.

use alloc::{format, vec::Vec};

use alloy_eips::{Encodable2718, eip7685::Requests};
use alloy_genesis::ChainConfig;
use alloy_primitives::U256;
use anyhow::Context;
use ere_zkvm_interface::Input;
use guest::{GuestIo, Io};
use reth_ethereum_primitives::TransactionSigned;
use reth_primitives_traits::Block;
pub use reth_stateless::StatelessInput;
use reth_stateless::UncompressedPublicKey;
use ssz_types::{FixedVector, VariableList};
pub use stateless_validator_common::guest::StatelessValidatorOutput;
use stateless_validator_common::new_payload_request::{
    ExecutionPayloadV1, ExecutionPayloadV2, ExecutionPayloadV3, ForkName, NewPayloadRequest,
    Transaction as Tx, Transactions, Withdrawal, Withdrawals,
};

use crate::guest::{StatelessValidatorRethGuest, StatelessValidatorRethInput};

impl StatelessValidatorRethInput {
    /// Construct [`StatelessValidatorRethInput`] given [`StatelessInput`].
    pub fn new(stateless_input: &StatelessInput, requests: Requests) -> anyhow::Result<Self> {
        let new_payload_request = to_new_payload_request(stateless_input, requests)?;
        let signers = recover_signers(&stateless_input.block.body.transactions)?;

        Ok(Self {
            new_payload_request,
            witness: stateless_input.witness.clone(),
            chain_config: stateless_input.chain_config.clone(),
            public_keys: signers,
        })
    }

    /// Returns [`Input`] to [`zkVM`] methods.
    ///
    /// [`zkVM`]: ere_zkvm_interface::zkVM
    pub fn to_zkvm_input(&self) -> anyhow::Result<Input> {
        let stdin = GuestIo::<StatelessValidatorRethGuest>::serialize_input(self)?;
        Ok(Input::new().with_prefixed_stdin(stdin))
    }
}

/// Recover public keys from transaction signatures.
pub fn recover_signers<'a, I>(txs: I) -> anyhow::Result<Vec<UncompressedPublicKey>>
where
    I: IntoIterator<Item = &'a TransactionSigned>,
{
    txs.into_iter()
        .enumerate()
        .map(|(i, tx)| {
            tx.signature()
                .recover_from_prehash(&tx.signature_hash())
                .map(|key| key.to_encoded_point(false).as_bytes().try_into().unwrap())
                .map(UncompressedPublicKey)
                .with_context(|| format!("failed to recover signature for tx #{i}"))
        })
        .collect()
}

/// Determines the fork name based on alloy chain config and block timestamp.
pub fn determine_fork_name(chain_config: &ChainConfig, timestamp: u64) -> ForkName {
    // Check forks in reverse chronological order
    if chain_config
        .prague_time
        .is_some_and(|prague_time| timestamp >= prague_time)
    {
        return ForkName::Electra;
    }
    if chain_config
        .cancun_time
        .is_some_and(|cancun_time| timestamp >= cancun_time)
    {
        return ForkName::Deneb;
    }
    if chain_config
        .shanghai_time
        .is_some_and(|shanghai_time| timestamp >= shanghai_time)
    {
        return ForkName::Capella;
    }
    // Default to Bellatrix for post-merge blocks
    ForkName::Bellatrix
}

/// Converts a [`StatelessInput`] to a [`NewPayloadRequest`].
///
/// This creates the appropriate NewPayloadRequest variant based on the fork.
pub fn to_new_payload_request(
    stateless_input: &StatelessInput,
    requests: Requests,
) -> anyhow::Result<NewPayloadRequest> {
    use alloy_consensus::transaction::Transaction;

    let header = stateless_input.block.header();
    let body = stateless_input.block.body();
    let fork = determine_fork_name(&stateless_input.chain_config, header.timestamp);

    // Convert transactions to RLP-encoded bytes
    let transactions: Transactions = {
        let txs: Vec<Tx> = body
            .transactions()
            .map(|tx| {
                let mut buf = Vec::new();
                tx.encode_2718(&mut buf);
                Tx::from(buf)
            })
            .collect();
        Transactions::from(txs)
    };

    // Helper to convert alloy withdrawal to our neutral type
    let convert_withdrawal = |w: &alloy_eips::eip4895::Withdrawal| Withdrawal {
        index: w.index,
        validator_index: w.validator_index,
        address: w.address.0.0,
        amount: w.amount,
    };

    match fork {
        ForkName::Bellatrix => {
            let payload = ExecutionPayloadV1 {
                parent_hash: header.parent_hash.0,
                fee_recipient: header.beneficiary.0.0,
                state_root: header.state_root.0,
                receipts_root: header.receipts_root.0,
                logs_bloom: FixedVector::from(header.logs_bloom.0.to_vec()),
                prev_randao: header.mix_hash.0,
                block_number: header.number,
                gas_limit: header.gas_limit,
                gas_used: header.gas_used,
                timestamp: header.timestamp,
                extra_data: VariableList::from(header.extra_data.to_vec()),
                base_fee_per_gas: U256::from(header.base_fee_per_gas.unwrap_or_default())
                    .to_le_bytes(),
                block_hash: stateless_input.block.hash_slow().0,
                transactions: transactions.clone(),
            };
            Ok(NewPayloadRequest::new_bellatrix(payload))
        }
        ForkName::Capella => {
            let withdrawals: Withdrawals = {
                let wdls: Vec<Withdrawal> = body
                    .withdrawals
                    .as_ref()
                    .map(|ws| ws.iter().map(convert_withdrawal).collect())
                    .unwrap_or_default();
                Withdrawals::from(wdls)
            };

            let payload = ExecutionPayloadV2 {
                parent_hash: header.parent_hash.0,
                fee_recipient: header.beneficiary.0.0,
                state_root: header.state_root.0,
                receipts_root: header.receipts_root.0,
                logs_bloom: FixedVector::from(header.logs_bloom.0.to_vec()),
                prev_randao: header.mix_hash.0,
                block_number: header.number,
                gas_limit: header.gas_limit,
                gas_used: header.gas_used,
                timestamp: header.timestamp,
                extra_data: VariableList::from(header.extra_data.to_vec()),
                base_fee_per_gas: U256::from(header.base_fee_per_gas.unwrap_or_default())
                    .to_le_bytes(),
                block_hash: stateless_input.block.hash_slow().0,
                transactions: transactions.clone(),
                withdrawals,
            };
            Ok(NewPayloadRequest::new_capella(payload))
        }
        ForkName::Deneb => {
            let withdrawals: Withdrawals = {
                let wdls: Vec<Withdrawal> = body
                    .withdrawals
                    .as_ref()
                    .map(|ws| ws.iter().map(convert_withdrawal).collect())
                    .unwrap_or_default();
                Withdrawals::from(wdls)
            };

            let payload = ExecutionPayloadV3 {
                parent_hash: header.parent_hash.0,
                fee_recipient: header.beneficiary.0.0,
                state_root: header.state_root.0,
                receipts_root: header.receipts_root.0,
                logs_bloom: FixedVector::from(header.logs_bloom.0.to_vec()),
                prev_randao: header.mix_hash.0,
                block_number: header.number,
                gas_limit: header.gas_limit,
                gas_used: header.gas_used,
                timestamp: header.timestamp,
                extra_data: VariableList::from(header.extra_data.to_vec()),
                base_fee_per_gas: U256::from(header.base_fee_per_gas.unwrap_or_default())
                    .to_le_bytes(),
                block_hash: stateless_input.block.hash_slow().0,
                transactions: transactions.clone(),
                withdrawals,
                blob_gas_used: header.blob_gas_used.unwrap_or_default(),
                excess_blob_gas: header.excess_blob_gas.unwrap_or_default(),
            };

            // Collect blob versioned hashes from all blob transactions
            let versioned_hashes: Vec<[u8; 32]> = body
                .transactions()
                .filter_map(|tx| tx.blob_versioned_hashes())
                .flatten()
                .map(|h| h.0)
                .collect();

            let parent_beacon_block_root = stateless_input
                .block
                .parent_beacon_block_root
                .unwrap_or_default()
                .0;

            NewPayloadRequest::new_deneb(payload, versioned_hashes, parent_beacon_block_root)
        }
        ForkName::Electra => {
            let withdrawals: Withdrawals = {
                let wdls: Vec<Withdrawal> = body
                    .withdrawals
                    .as_ref()
                    .map(|ws| ws.iter().map(convert_withdrawal).collect())
                    .unwrap_or_default();
                Withdrawals::from(wdls)
            };

            let payload = ExecutionPayloadV3 {
                parent_hash: header.parent_hash.0,
                fee_recipient: header.beneficiary.0.0,
                state_root: header.state_root.0,
                receipts_root: header.receipts_root.0,
                logs_bloom: FixedVector::from(header.logs_bloom.0.to_vec()),
                prev_randao: header.mix_hash.0,
                block_number: header.number,
                gas_limit: header.gas_limit,
                gas_used: header.gas_used,
                timestamp: header.timestamp,
                extra_data: VariableList::from(header.extra_data.to_vec()),
                base_fee_per_gas: U256::from(header.base_fee_per_gas.unwrap_or_default())
                    .to_le_bytes(),
                block_hash: stateless_input.block.hash_slow().0,
                transactions,
                withdrawals,
                blob_gas_used: header.blob_gas_used.unwrap_or_default(),
                excess_blob_gas: header.excess_blob_gas.unwrap_or_default(),
            };

            // Collect blob versioned hashes from all blob transactions
            let versioned_hashes: Vec<[u8; 32]> = body
                .transactions()
                .filter_map(|tx| tx.blob_versioned_hashes())
                .flatten()
                .map(|h| h.0)
                .collect();

            let parent_beacon_block_root = stateless_input
                .block
                .parent_beacon_block_root
                .unwrap_or_default()
                .0;

            NewPayloadRequest::new_electra_fulu(
                payload,
                versioned_hashes,
                parent_beacon_block_root,
                &requests,
            )
        }
    }
}

#[cfg(test)]
mod test {
    use stateless_validator_common::new_payload_request::{ExecutionPayloadV1, NewPayloadRequest};

    use crate::guest::{Io, StatelessValidatorOutput, StatelessValidatorRethIo};

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
                StatelessValidatorRethIo::serialize_output(&output).unwrap(),
                output.serialize()
            );
        }
    }
}
