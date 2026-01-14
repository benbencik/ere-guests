use anyhow::{Context, Result};
use ethrex_common::types::Block;
use stateless_validator_common::new_payload_request::NewPayloadRequest;

/// Converts a [`NewPayloadRequest`] into a validated reth [`Block`].
pub fn new_payload_request_to_block(new_payload_request: NewPayloadRequest) -> Result<Block> {}
