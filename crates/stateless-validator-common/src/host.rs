//! Stateless validator common types and utilities for host.

use sha2::{Digest, Sha256};

use crate::{
    execution_payload::{
        ConsolidationRequest, DepositRequest, ExecutionPayloadV1, ExecutionPayloadV2,
        ExecutionPayloadV3, ExecutionRequests, Hash32, NewPayloadRequest,
        NewPayloadRequestBellatrix, NewPayloadRequestCapella, NewPayloadRequestDeneb,
        NewPayloadRequestElectraFulu, WithdrawalRequest,
    },
    guest::StatelessValidatorOutput,
};
use anyhow::{Context, Result};
use ssz::{Decode, Encode};
use ssz_types::VariableList;

impl StatelessValidatorOutput {
    /// Returns sha256 digest of serialized output.
    pub fn sha256(&self) -> [u8; 32] {
        Sha256::digest(self.serialize()).into()
    }
}

impl NewPayloadRequest {
    /// Constructs a new [`NewPayloadRequest`] for Bellatrix.
    pub fn new_bellatrix(execution_payload: ExecutionPayloadV1) -> Self {
        NewPayloadRequest::Bellatrix(NewPayloadRequestBellatrix { execution_payload })
    }

    /// Constructs a new [`NewPayloadRequest`] for Capella.
    pub fn new_capella(execution_payload: ExecutionPayloadV2) -> Self {
        NewPayloadRequest::Capella(NewPayloadRequestCapella { execution_payload })
    }

    /// Constructs a new [`NewPayloadRequest`] for Deneb.
    pub fn new_deneb(
        execution_payload: ExecutionPayloadV3,
        versioned_hashes: Vec<Hash32>,
        parent_beacon_block_root: Hash32,
    ) -> Result<Self> {
        let versioned_hashes = VariableList::new(versioned_hashes).map_err(|err| {
            anyhow::anyhow!("Versioned hashes length should be within bounds: {:?}", err)
        })?;
        Ok(NewPayloadRequest::Deneb(NewPayloadRequestDeneb {
            execution_payload,
            versioned_hashes,
            parent_beacon_block_root,
        }))
    }

    /// Constructs a new [`NewPayloadRequest`] for Electra or Fulu.
    pub fn new_electra_fulu(
        execution_payload: ExecutionPayloadV3,
        versioned_hashes: Vec<Hash32>,
        parent_beacon_block_root: Hash32,
        execution_requests: &[impl AsRef<[u8]>],
    ) -> Result<Self> {
        let versioned_hashes = VariableList::new(versioned_hashes).map_err(|err| {
            anyhow::anyhow!("Versioned hashes length should be within bounds: {:?}", err)
        })?;
        let execution_requests = decode_execution_requests(execution_requests)
            .context("Decoding execution requests failed")?;
        Ok(NewPayloadRequest::ElectraFulu(
            NewPayloadRequestElectraFulu {
                execution_payload,
                versioned_hashes,
                parent_beacon_block_root,
                execution_requests,
            },
        ))
    }
}

/// Decodes a list of execution requests obtained from execution and deserializes them into an
/// [`ExecutionRequests`] struct.
fn decode_execution_requests(requests_list: &[impl AsRef<[u8]>]) -> Result<ExecutionRequests> {
    // EIP-7685: requests are encoded as request_type (1 byte) ++ request_data
    // Request types for Electra (Prague):
    // - 0x00: Deposit requests (EIP-6110)
    // - 0x01: Withdrawal requests (EIP-7002)
    // - 0x02: Consolidation requests (EIP-7251)

    const DEPOSIT_REQUEST_TYPE: u8 = 0x00;
    const WITHDRAWAL_REQUEST_TYPE: u8 = 0x01;
    const CONSOLIDATION_REQUEST_TYPE: u8 = 0x02;

    // Fixed SSZ sizes for each request type (excluding the type byte)
    let deposit_request_size = <DepositRequest as Encode>::ssz_fixed_len();
    let withdrawal_request_size = <WithdrawalRequest as Encode>::ssz_fixed_len();
    let consolidation_request_size = <ConsolidationRequest as Encode>::ssz_fixed_len();

    let mut deposits = Vec::new();
    let mut withdrawals = Vec::new();
    let mut consolidations = Vec::new();

    for (idx, request) in requests_list.iter().enumerate() {
        let request_bytes = request.as_ref();

        anyhow::ensure!(!request_bytes.is_empty(), "Empty request at index {}", idx);

        // Read request type (first byte)
        let request_type = request_bytes[0];
        let data = &request_bytes[1..];

        match request_type {
            DEPOSIT_REQUEST_TYPE => {
                anyhow::ensure!(
                    data.len() == deposit_request_size,
                    "Invalid deposit request size at index {}: expected {}, got {}",
                    idx,
                    deposit_request_size,
                    data.len()
                );

                let deposit = DepositRequest::from_ssz_bytes(data).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to SSZ decode deposit request at index {}: {:?}",
                        idx,
                        e
                    )
                })?;
                deposits.push(deposit);
            }
            WITHDRAWAL_REQUEST_TYPE => {
                anyhow::ensure!(
                    data.len() == withdrawal_request_size,
                    "Invalid withdrawal request size at index {}: expected {}, got {}",
                    idx,
                    withdrawal_request_size,
                    data.len()
                );

                let withdrawal = WithdrawalRequest::from_ssz_bytes(data).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to SSZ decode withdrawal request at index {}: {:?}",
                        idx,
                        e
                    )
                })?;
                withdrawals.push(withdrawal);
            }
            CONSOLIDATION_REQUEST_TYPE => {
                anyhow::ensure!(
                    data.len() == consolidation_request_size,
                    "Invalid consolidation request size at index {}: expected {}, got {}",
                    idx,
                    consolidation_request_size,
                    data.len()
                );

                let consolidation = ConsolidationRequest::from_ssz_bytes(data).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to SSZ decode consolidation request at index {}: {:?}",
                        idx,
                        e
                    )
                })?;
                consolidations.push(consolidation);
            }
            _ => {
                anyhow::bail!("Unknown request type at index {}: {:#x}", idx, request_type);
            }
        }
    }

    Ok(ExecutionRequests {
        deposits: VariableList::new(deposits)
            .map_err(|e| anyhow::anyhow!("Failed to create deposits VariableList: {:?}", e))?,
        withdrawals: VariableList::new(withdrawals)
            .map_err(|e| anyhow::anyhow!("Failed to create withdrawals VariableList: {:?}", e))?,
        consolidations: VariableList::new(consolidations).map_err(|e| {
            anyhow::anyhow!("Failed to create consolidations VariableList: {:?}", e)
        })?,
    })
}
