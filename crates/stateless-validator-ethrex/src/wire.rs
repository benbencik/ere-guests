//! EIP-8025 framing helpers for the ethrex guest path.

use alloc::vec::Vec;
use ethrex_common::types::block_execution_witness::ExecutionWitness;
use libssz::SszEncode;
use libssz_derive::SszEncode;
use libssz_types::SszList;
use stateless_validator_common::new_payload_request::{
    ConsolidationRequest, DepositRequest, NewPayloadRequestElectraFulu, WithdrawalRequest,
    Withdrawal,
};

use stateless_validator_common::new_payload_request::{
    MAX_BYTES_PER_TRANSACTION, MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD,
    MAX_DEPOSIT_REQUESTS_PER_PAYLOAD, MAX_TRANSACTIONS_PER_PAYLOAD,
    MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD, MAX_WITHDRAWALS_PER_PAYLOAD,
};

const MAX_BLOB_COMMITMENTS: usize = 4096;
const MAX_EXECUTION_REQUESTS: usize = 16;
const MAX_EXECUTION_REQUEST_BYTES: usize = 1_073_741_824;
const MAX_EXTRA_DATA_BYTES: usize = 32;

type Transaction = SszList<u8, MAX_BYTES_PER_TRANSACTION>;
type Transactions = SszList<Transaction, MAX_TRANSACTIONS_PER_PAYLOAD>;

/// ExecutionPayload matching ethrex's `eip8025_ssz::ExecutionPayload`.
#[derive(SszEncode)]
struct EthrexExecutionPayload {
    parent_hash: [u8; 32],
    fee_recipient: [u8; 20],
    state_root: [u8; 32],
    receipts_root: [u8; 32],
    logs_bloom: [u8; 256],
    prev_randao: [u8; 32],
    block_number: u64,
    gas_limit: u64,
    gas_used: u64,
    timestamp: u64,
    extra_data: SszList<u8, MAX_EXTRA_DATA_BYTES>,
    base_fee_per_gas: [u8; 32],
    block_hash: [u8; 32],
    transactions: Transactions,
    withdrawals: SszList<Withdrawal, MAX_WITHDRAWALS_PER_PAYLOAD>,
    blob_gas_used: u64,
    excess_blob_gas: u64,
    deposit_requests: SszList<DepositRequest, MAX_DEPOSIT_REQUESTS_PER_PAYLOAD>,
    withdrawal_requests: SszList<WithdrawalRequest, MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD>,
    consolidation_requests: SszList<ConsolidationRequest, MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD>,
}

/// NewPayloadRequest matching ethrex's `eip8025_ssz::NewPayloadRequest`.
#[derive(SszEncode)]
struct EthrexNewPayloadRequest {
    execution_payload: EthrexExecutionPayload,
    versioned_hashes: SszList<[u8; 32], MAX_BLOB_COMMITMENTS>,
    parent_beacon_block_root: [u8; 32],
    execution_requests: SszList<SszList<u8, MAX_EXECUTION_REQUEST_BYTES>, MAX_EXECUTION_REQUESTS>,
}

/// Encodes an Electra/Fulu new payload request and opaque witness bytes as
/// `[ssz_len: u32 LE][ssz_bytes][ssz_bytes]`.
pub fn encode_eip8025(
    npr: &NewPayloadRequestElectraFulu,
    execution_witness: &ExecutionWitness,
) -> Result<Vec<u8>, ethrex_common::types::block_execution_witness::ExecutionWitnessSszError> {
    let p = &npr.execution_payload;
    let r = &npr.execution_requests;

    let ethrex_payload = EthrexExecutionPayload {
        parent_hash: p.parent_hash,
        fee_recipient: p.fee_recipient,
        state_root: p.state_root,
        receipts_root: p.receipts_root,
        logs_bloom: p.logs_bloom,
        prev_randao: p.prev_randao,
        block_number: p.block_number,
        gas_limit: p.gas_limit,
        gas_used: p.gas_used,
        timestamp: p.timestamp,
        extra_data: p.extra_data.clone(),
        base_fee_per_gas: p.base_fee_per_gas,
        block_hash: p.block_hash,
        transactions: p.transactions.clone(),
        withdrawals: p.withdrawals.clone(),
        blob_gas_used: p.blob_gas_used,
        excess_blob_gas: p.excess_blob_gas,
        deposit_requests: r.deposits.clone(),
        withdrawal_requests: r.withdrawals.clone(),
        consolidation_requests: r.consolidations.clone(),
    };

    let mut opaque_requests: Vec<SszList<u8, MAX_EXECUTION_REQUEST_BYTES>> = Vec::new();

    if !r.deposits.is_empty() {
        let mut buf = alloc::vec![0x00u8]; // deposit type byte
        for deposit in r.deposits.iter() {
            buf.extend_from_slice(&deposit.to_ssz());
        }
        opaque_requests.push(buf.try_into().expect("deposit requests too large"));
    }

    if !r.withdrawals.is_empty() {
        let mut buf = alloc::vec![0x01u8]; // withdrawal type byte
        for withdrawal in r.withdrawals.iter() {
            buf.extend_from_slice(&withdrawal.to_ssz());
        }
        opaque_requests.push(buf.try_into().expect("withdrawal requests too large"));
    }

    if !r.consolidations.is_empty() {
        let mut buf = alloc::vec![0x02u8]; // consolidation type byte
        for consolidation in r.consolidations.iter() {
            buf.extend_from_slice(&consolidation.to_ssz());
        }
        opaque_requests.push(buf.try_into().expect("consolidation requests too large"));
    }

    let ethrex_request = EthrexNewPayloadRequest {
        execution_payload: ethrex_payload,
        versioned_hashes: npr.versioned_hashes.clone(),
        parent_beacon_block_root: npr.parent_beacon_block_root,
        execution_requests: opaque_requests
            .try_into()
            .expect("too many execution request types"),
    };

    let ssz_bytes = ethrex_request.to_ssz();
    let ssz_len = u32::try_from(ssz_bytes.len()).expect("SSZ payload length exceeds u32");
    let witness_ssz_bytes = execution_witness.to_ssz_bytes()?;

    let mut out = Vec::with_capacity(4 + ssz_bytes.len() + witness_ssz_bytes.len());
    out.extend_from_slice(&ssz_len.to_le_bytes());
    out.extend_from_slice(&ssz_bytes);
    out.extend_from_slice(&witness_ssz_bytes);
    Ok(out)
}
