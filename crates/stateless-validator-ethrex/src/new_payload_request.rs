//! Conversion utilities for NewPayloadRequest to ethrex Block.

use anyhow::{Context, Result};
use ethrex_common::{Address, Bloom, Bytes, H256, types::Block};
use stateless_validator_common::new_payload_request::{
    ExecutionPayloadV1, ExecutionPayloadV2, ExecutionPayloadV3, NewPayloadRequest, Withdrawal,
    compute_requests_hash,
};

use crate::execution_payload::{
    EncodedTransaction, ExecutionPayload, validate_execution_payload_v1,
    validate_execution_payload_v2, validate_execution_payload_v3,
};

/// Converts a [`NewPayloadRequest`] into an ethrex [`Block`].
pub fn get_block_from_new_payload_request(req: NewPayloadRequest) -> Result<Block> {
    match req {
        NewPayloadRequest::Bellatrix(b) => {
            let payload = convert_v1_to_ethrex(b.execution_payload);
            validate_execution_payload_v1(&payload).context("V1 payload validation failed")?;
            payload.into_block(None, None).context("into_block failed")
        }
        NewPayloadRequest::Capella(c) => {
            let payload = convert_v2_to_ethrex(c.execution_payload);
            validate_execution_payload_v2(&payload).context("V2 payload validation failed")?;
            payload.into_block(None, None).context("into_block failed")
        }
        NewPayloadRequest::Deneb(d) => {
            let parent_beacon_block_root = Some(H256::from(d.parent_beacon_block_root));
            let payload = convert_v3_to_ethrex(d.execution_payload);
            validate_execution_payload_v3(&payload).context("V3 payload validation failed")?;
            payload
                .into_block(parent_beacon_block_root, None)
                .context("into_block failed")
        }
        NewPayloadRequest::ElectraFulu(e) => {
            let parent_beacon_block_root = Some(H256::from(e.parent_beacon_block_root));
            let requests_hash = Some(H256::from(compute_requests_hash(&e.execution_requests)));
            let payload = convert_v3_to_ethrex(e.execution_payload);
            validate_execution_payload_v3(&payload).context("V3 payload validation failed")?;
            payload
                .into_block(parent_beacon_block_root, requests_hash)
                .context("into_block failed")
        }
    }
}

/// Convert V1 payload (Bellatrix) to ethrex ExecutionPayload.
fn convert_v1_to_ethrex(payload: ExecutionPayloadV1) -> ExecutionPayload {
    ExecutionPayload {
        parent_hash: H256::from(payload.parent_hash),
        fee_recipient: Address::from(payload.fee_recipient),
        state_root: H256::from(payload.state_root),
        receipts_root: H256::from(payload.receipts_root),
        logs_bloom: Bloom::from_slice(&payload.logs_bloom[..]),
        prev_randao: H256::from(payload.prev_randao),
        block_number: payload.block_number,
        gas_limit: payload.gas_limit,
        gas_used: payload.gas_used,
        timestamp: payload.timestamp,
        extra_data: Bytes::from(Vec::from(payload.extra_data)),
        base_fee_per_gas: base_fee_to_u64(&payload.base_fee_per_gas),
        block_hash: H256::from(payload.block_hash),
        transactions: payload
            .transactions
            .into_iter()
            .map(|t| EncodedTransaction(Bytes::from(Vec::from(t))))
            .collect(),
        withdrawals: None,
        blob_gas_used: None,
        excess_blob_gas: None,
    }
}

/// Convert V2 payload (Capella) to ethrex ExecutionPayload.
fn convert_v2_to_ethrex(payload: ExecutionPayloadV2) -> ExecutionPayload {
    ExecutionPayload {
        parent_hash: H256::from(payload.parent_hash),
        fee_recipient: Address::from(payload.fee_recipient),
        state_root: H256::from(payload.state_root),
        receipts_root: H256::from(payload.receipts_root),
        logs_bloom: Bloom::from_slice(&payload.logs_bloom[..]),
        prev_randao: H256::from(payload.prev_randao),
        block_number: payload.block_number,
        gas_limit: payload.gas_limit,
        gas_used: payload.gas_used,
        timestamp: payload.timestamp,
        extra_data: Bytes::from(Vec::from(payload.extra_data)),
        base_fee_per_gas: base_fee_to_u64(&payload.base_fee_per_gas),
        block_hash: H256::from(payload.block_hash),
        transactions: payload
            .transactions
            .into_iter()
            .map(|t| EncodedTransaction(Bytes::from(Vec::from(t))))
            .collect(),
        withdrawals: Some(
            payload
                .withdrawals
                .into_iter()
                .map(convert_withdrawal)
                .collect(),
        ),
        blob_gas_used: None,
        excess_blob_gas: None,
    }
}

/// Convert V3 payload (Deneb/Electra) to ethrex ExecutionPayload.
fn convert_v3_to_ethrex(payload: ExecutionPayloadV3) -> ExecutionPayload {
    ExecutionPayload {
        parent_hash: H256::from(payload.parent_hash),
        fee_recipient: Address::from(payload.fee_recipient),
        state_root: H256::from(payload.state_root),
        receipts_root: H256::from(payload.receipts_root),
        logs_bloom: Bloom::from_slice(&payload.logs_bloom[..]),
        prev_randao: H256::from(payload.prev_randao),
        block_number: payload.block_number,
        gas_limit: payload.gas_limit,
        gas_used: payload.gas_used,
        timestamp: payload.timestamp,
        extra_data: Bytes::from(Vec::from(payload.extra_data)),
        base_fee_per_gas: base_fee_to_u64(&payload.base_fee_per_gas),
        block_hash: H256::from(payload.block_hash),
        transactions: payload
            .transactions
            .into_iter()
            .map(|t| EncodedTransaction(Bytes::from(Vec::from(t))))
            .collect(),
        withdrawals: Some(
            payload
                .withdrawals
                .into_iter()
                .map(convert_withdrawal)
                .collect(),
        ),
        blob_gas_used: Some(payload.blob_gas_used),
        excess_blob_gas: Some(payload.excess_blob_gas),
    }
}

/// Convert our Withdrawal type to ethrex's Withdrawal type.
fn convert_withdrawal(w: Withdrawal) -> ethrex_common::types::Withdrawal {
    ethrex_common::types::Withdrawal {
        index: w.index,
        validator_index: w.validator_index,
        address: Address::from(w.address),
        amount: w.amount,
    }
}

/// Convert base_fee_per_gas from 32-byte little-endian to u64.
fn base_fee_to_u64(base_fee: &[u8; 32]) -> u64 {
    u64::from_le_bytes(base_fee[..8].try_into().unwrap())
}
