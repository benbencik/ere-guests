#![allow(missing_docs)]
use bytes::Bytes;

use ethrex_common::{
    Address, Bloom, H256,
    constants::DEFAULT_OMMERS_HASH,
    types::{
        Block, BlockBody, BlockHeader, Transaction, Withdrawal, compute_transactions_root,
        compute_withdrawals_root,
    },
};
use ethrex_rlp::error::RLPDecodeError;

#[derive(Clone, Debug)]
pub struct ExecutionPayload {
    pub parent_hash: H256,
    pub fee_recipient: Address,
    pub state_root: H256,
    pub receipts_root: H256,
    pub logs_bloom: Bloom,
    pub prev_randao: H256,
    pub block_number: u64,
    pub gas_limit: u64,
    pub gas_used: u64,
    pub timestamp: u64,
    pub extra_data: Bytes,
    pub base_fee_per_gas: u64,
    pub block_hash: H256,
    pub transactions: Vec<EncodedTransaction>,
    pub withdrawals: Option<Vec<Withdrawal>>,
    // ExecutionPayloadV3 fields. Optional since we support V2 too
    pub blob_gas_used: Option<u64>,
    pub excess_blob_gas: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct EncodedTransaction(pub Bytes);

impl ExecutionPayload {
    /// Converts an `ExecutionPayload` into a block (aka a BlockHeader and BlockBody)
    /// using the parentBeaconBlockRoot received along with the payload in the rpc call `engine_newPayloadV2/V3`
    pub fn into_block(
        self,
        parent_beacon_block_root: Option<H256>,
        requests_hash: Option<H256>,
    ) -> Result<Block, RLPDecodeError> {
        let body = BlockBody {
            transactions: self
                .transactions
                .iter()
                .map(|encoded_tx| encoded_tx.decode())
                .collect::<Result<Vec<_>, RLPDecodeError>>()?,
            ommers: vec![],
            withdrawals: self.withdrawals,
        };
        let header = BlockHeader {
            parent_hash: self.parent_hash,
            ommers_hash: *DEFAULT_OMMERS_HASH,
            coinbase: self.fee_recipient,
            state_root: self.state_root,
            transactions_root: compute_transactions_root(&body.transactions),
            receipts_root: self.receipts_root,
            logs_bloom: self.logs_bloom,
            difficulty: 0.into(),
            number: self.block_number,
            gas_limit: self.gas_limit,
            gas_used: self.gas_used,
            timestamp: self.timestamp,
            extra_data: self.extra_data,
            prev_randao: self.prev_randao,
            nonce: 0,
            base_fee_per_gas: Some(self.base_fee_per_gas),
            withdrawals_root: body
                .withdrawals
                .as_ref()
                .map(|w| compute_withdrawals_root(w)),
            blob_gas_used: self.blob_gas_used,
            excess_blob_gas: self.excess_blob_gas,
            parent_beacon_block_root,
            // TODO: set the value properly
            requests_hash,
            ..Default::default()
        };

        Ok(Block::new(header, body))
    }
}

impl EncodedTransaction {
    fn decode(&self) -> Result<Transaction, RLPDecodeError> {
        Transaction::decode_canonical(self.0.as_ref())
    }
}
