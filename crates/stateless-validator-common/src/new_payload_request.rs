//! Consensus types to support new payload requests.

#![allow(missing_docs)]

use alloc::{vec, vec::Vec};

use libssz_derive::{HashTreeRoot, SszDecode, SszEncode};
use libssz_merkle::HashTreeRoot;
use libssz_types::SszList;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Primitive types
pub type Hash32 = [u8; 32];
pub type Bytes48 = [u8; 48];
pub type Bytes96 = [u8; 96];
pub type Address20 = [u8; 20];
pub type Uint256Bytes = [u8; 32];
pub type LogsBloom = [u8; 256];
pub type ExtraData = SszList<u8, 32>;

/// Limits
pub const MAX_WITHDRAWALS_PER_PAYLOAD: usize = 16;
pub const MAX_TRANSACTIONS_PER_PAYLOAD: usize = 1024 * 1024;
pub const MAX_BYTES_PER_TRANSACTION: usize = MAX_TRANSACTIONS_PER_PAYLOAD * 1024;
pub const MAX_BLOB_COMMITMENTS_PER_BLOCK: usize = 4096;
pub const MAX_DEPOSIT_REQUESTS_PER_PAYLOAD: usize = 8192;
pub const MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD: usize = 16;
pub const MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD: usize = 2;

/// Composite types
pub type Transaction = SszList<u8, MAX_BYTES_PER_TRANSACTION>;
pub type Transactions = SszList<Transaction, MAX_TRANSACTIONS_PER_PAYLOAD>;
pub type Withdrawals = SszList<Withdrawal, MAX_WITHDRAWALS_PER_PAYLOAD>;
pub type VersionedHashes = SszList<Hash32, MAX_BLOB_COMMITMENTS_PER_BLOCK>;
pub type DepositRequests = SszList<DepositRequest, MAX_DEPOSIT_REQUESTS_PER_PAYLOAD>;
pub type WithdrawalRequests = SszList<WithdrawalRequest, MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD>;
pub type ConsolidationRequests =
    SszList<ConsolidationRequest, MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD>;

#[derive(Debug, Clone, HashTreeRoot, SszEncode, SszDecode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct Withdrawal {
    pub index: u64,
    pub validator_index: u64,
    pub address: Address20,
    pub amount: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, HashTreeRoot, SszEncode, SszDecode)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct DepositRequest {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_wrappers::bytes_array"))]
    pub pubkey: Bytes48,
    pub withdrawal_credentials: Hash32,
    pub amount: u64,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_wrappers::bytes_array"))]
    pub signature: Bytes96,
    pub index: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, HashTreeRoot, SszEncode, SszDecode)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct WithdrawalRequest {
    pub source_address: Address20,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_wrappers::bytes_array"))]
    pub validator_pubkey: Bytes48,
    pub amount: u64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, HashTreeRoot, SszEncode, SszDecode)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct ConsolidationRequest {
    pub source_address: Address20,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_wrappers::bytes_array"))]
    pub source_pubkey: Bytes48,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_wrappers::bytes_array"))]
    pub target_pubkey: Bytes48,
}

#[derive(Debug, Clone, Default, HashTreeRoot)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct ExecutionRequests {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_wrappers::ssz_list"))]
    #[cfg_attr(feature = "rkyv", rkyv(with = crate::rkyv_wrappers::AsVariableList))]
    pub deposits: DepositRequests,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_wrappers::ssz_list"))]
    #[cfg_attr(feature = "rkyv", rkyv(with = crate::rkyv_wrappers::AsVariableList))]
    pub withdrawals: WithdrawalRequests,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_wrappers::ssz_list"))]
    #[cfg_attr(feature = "rkyv", rkyv(with = crate::rkyv_wrappers::AsVariableList))]
    pub consolidations: ConsolidationRequests,
}

#[derive(Debug, Clone, Copy)]
pub enum ForkName {
    Bellatrix,
    Capella,
    Deneb,
    Electra,
    Fulu,
}

#[derive(Debug, Clone, HashTreeRoot)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct ExecutionPayloadV1 {
    pub parent_hash: Hash32,
    pub fee_recipient: Address20,
    pub state_root: Hash32,
    pub receipts_root: Hash32,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_wrappers::bytes_array"))]
    pub logs_bloom: LogsBloom,
    pub prev_randao: Hash32,
    pub block_number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_wrappers::ssz_list"))]
    #[cfg_attr(feature = "rkyv", rkyv(with = crate::rkyv_wrappers::AsVariableList))]
    pub extra_data: ExtraData,
    pub base_fee_per_gas: Uint256Bytes,
    pub block_hash: Hash32,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::serde_wrappers::nested_ssz_list")
    )]
    #[cfg_attr(feature = "rkyv", rkyv(with = crate::rkyv_wrappers::AsNestedVariableList))]
    pub transactions: Transactions,
}

#[derive(Debug, Clone, HashTreeRoot)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct ExecutionPayloadV2 {
    pub parent_hash: Hash32,
    pub fee_recipient: Address20,
    pub state_root: Hash32,
    pub receipts_root: Hash32,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_wrappers::bytes_array"))]
    pub logs_bloom: LogsBloom,
    pub prev_randao: Hash32,
    pub block_number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_wrappers::ssz_list"))]
    #[cfg_attr(feature = "rkyv", rkyv(with = crate::rkyv_wrappers::AsVariableList))]
    pub extra_data: ExtraData,
    pub base_fee_per_gas: Uint256Bytes,
    pub block_hash: Hash32,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::serde_wrappers::nested_ssz_list")
    )]
    #[cfg_attr(feature = "rkyv", rkyv(with = crate::rkyv_wrappers::AsNestedVariableList))]
    pub transactions: Transactions,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_wrappers::ssz_list"))]
    #[cfg_attr(feature = "rkyv", rkyv(with = crate::rkyv_wrappers::AsVariableList))]
    pub withdrawals: Withdrawals,
}

#[derive(Debug, Clone, HashTreeRoot)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct ExecutionPayloadV3 {
    pub parent_hash: Hash32,
    pub fee_recipient: Address20,
    pub state_root: Hash32,
    pub receipts_root: Hash32,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_wrappers::bytes_array"))]
    pub logs_bloom: LogsBloom,
    pub prev_randao: Hash32,
    pub block_number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_wrappers::ssz_list"))]
    #[cfg_attr(feature = "rkyv", rkyv(with = crate::rkyv_wrappers::AsVariableList))]
    pub extra_data: ExtraData,
    pub base_fee_per_gas: Uint256Bytes,
    pub block_hash: Hash32,
    #[cfg_attr(
        feature = "serde",
        serde(with = "crate::serde_wrappers::nested_ssz_list")
    )]
    #[cfg_attr(feature = "rkyv", rkyv(with = crate::rkyv_wrappers::AsNestedVariableList))]
    pub transactions: Transactions,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_wrappers::ssz_list"))]
    #[cfg_attr(feature = "rkyv", rkyv(with = crate::rkyv_wrappers::AsVariableList))]
    pub withdrawals: Withdrawals,
    pub blob_gas_used: u64,
    pub excess_blob_gas: u64,
}

#[derive(Debug, Clone, HashTreeRoot)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct NewPayloadRequestBellatrix {
    pub execution_payload: ExecutionPayloadV1,
}

#[derive(Debug, Clone, HashTreeRoot)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct NewPayloadRequestCapella {
    pub execution_payload: ExecutionPayloadV2,
}

#[derive(Debug, Clone, HashTreeRoot)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct NewPayloadRequestDeneb {
    pub execution_payload: ExecutionPayloadV3,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_wrappers::ssz_list"))]
    #[cfg_attr(feature = "rkyv", rkyv(with = crate::rkyv_wrappers::AsVariableList))]
    pub versioned_hashes: VersionedHashes,
    pub parent_beacon_block_root: Hash32,
}

#[derive(Debug, Clone, HashTreeRoot)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub struct NewPayloadRequestElectraFulu {
    pub execution_payload: ExecutionPayloadV3,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_wrappers::ssz_list"))]
    #[cfg_attr(feature = "rkyv", rkyv(with = crate::rkyv_wrappers::AsVariableList))]
    pub versioned_hashes: VersionedHashes,
    pub parent_beacon_block_root: Hash32,
    pub execution_requests: ExecutionRequests,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)
)]
pub enum NewPayloadRequest {
    Bellatrix(NewPayloadRequestBellatrix),
    Capella(NewPayloadRequestCapella),
    Deneb(NewPayloadRequestDeneb),
    ElectraFulu(NewPayloadRequestElectraFulu),
}

impl NewPayloadRequest {
    pub fn tree_hash_root(&self) -> [u8; 32] {
        match self {
            NewPayloadRequest::Bellatrix(req) => req.hash_tree_root(),
            NewPayloadRequest::Capella(req) => req.hash_tree_root(),
            NewPayloadRequest::Deneb(req) => req.hash_tree_root(),
            NewPayloadRequest::ElectraFulu(req) => req.hash_tree_root(),
        }
    }
}

/// Computes the requests hash for EL block construction.
pub fn compute_requests_hash(requests: &ExecutionRequests) -> [u8; 32] {
    use libssz::SszEncode;
    use sha2::{Digest, Sha256};

    let mut outer_hasher = Sha256::new();

    // Deposit requests (type 0x00)
    let mut deposits_bytes = vec![0x00u8];
    for deposit in requests.deposits.iter() {
        deposits_bytes.extend(deposit.to_ssz());
    }
    if deposits_bytes.len() > 1 {
        outer_hasher.update(Sha256::digest(&deposits_bytes));
    }

    // Withdrawal requests (type 0x01)
    let mut withdrawals_bytes = vec![0x01u8];
    for withdrawal in requests.withdrawals.iter() {
        withdrawals_bytes.extend(withdrawal.to_ssz());
    }
    if withdrawals_bytes.len() > 1 {
        outer_hasher.update(Sha256::digest(&withdrawals_bytes));
    }

    // Consolidation requests (type 0x02)
    let mut consolidations_bytes = vec![0x02u8];
    for consolidation in requests.consolidations.iter() {
        consolidations_bytes.extend(consolidation.to_ssz());
    }
    if consolidations_bytes.len() > 1 {
        outer_hasher.update(Sha256::digest(&consolidations_bytes));
    }

    outer_hasher.finalize().into()
}
