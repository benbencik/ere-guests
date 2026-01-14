//! Test for StatelessInput <-> NewPayloadRequest conversion
use std::sync::Arc;

use integration_tests::get_fixtures;
use reth_chainspec::ChainSpec;
use reth_stateless::Genesis;
use stateless_validator_reth::{
    guest::StatelessValidatorRethInput, new_payload_request::new_payload_request_to_block,
};

// The guest program input is NewPayloadRequest but the prover input is StatelessInput.
// This test verifies that the guest program reconstructs the same block as the original StatelessInput.
#[test]
fn test_block_roundtrip() {
    for fixture in get_fixtures() {
        // Simulate the preparation the prover does to send input to the guest.
        let input =
            StatelessValidatorRethInput::new(&fixture.stateless_input, fixture.success).unwrap();
        let new_payload_request = input.new_payload_request;

        let genesis = Genesis {
            config: fixture.stateless_input.chain_config.clone(),
            ..Default::default()
        };
        let chain_spec: Arc<ChainSpec> = Arc::new(genesis.into());
        // In the guest, reconstruct the block from NewPayloadRequest.
        let block = new_payload_request_to_block(new_payload_request, chain_spec).unwrap();

        // Assert that the reconstructed block matches the original block in StatelessInput.
        let guest_block_hash = block.hash_slow();
        let stateless_input_block_hash = fixture.stateless_input.block.hash_slow();
        assert_eq!(
            stateless_input_block_hash, guest_block_hash,
            "Block hash mismatch for fixture: {}",
            fixture.name
        );
    }
}
