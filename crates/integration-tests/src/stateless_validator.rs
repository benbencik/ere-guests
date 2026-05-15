//! This module provides struct for stateless validator test fixture.

use std::collections::HashMap;

use alloy_primitives::{B256, b256};
use serde::{Deserialize, Serialize};
use stateless::StatelessInput;
use stateless_validator_common::guest::StatelessValidatorOutput;

/// A stateless validation fixture containing block data and witness information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatelessValidatorFixture {
    /// Name of the blockchain test case (e.g., "`ModExpAttackContract`").
    pub name: String,
    /// The stateless input for the block validation.
    pub stateless_input: StatelessInput,
    /// Whether the stateless block validation is successful.
    pub success: bool,
}

/// Returns the expected `StatelessValidatorOutput` for a given block hash in the fixtures, which
/// was calculated by an independent implementation.
pub fn get_stateless_validator_output(
    block_hash: B256,
    success: bool,
    chain_id: u64,
) -> StatelessValidatorOutput {
    let expected_roots = expected_execution_payload_tree_roots();
    let expected_root = *expected_roots.get(&block_hash).unwrap();

    StatelessValidatorOutput::new(expected_root.0, success, chain_id)
}

/// Returns a mapping of block hashes from fixtures to their expected execution payload tree roots.
/// These roots where independently computed from this repository.
fn expected_execution_payload_tree_roots() -> HashMap<B256, B256> {
    HashMap::from([
        (
            b256!("e4bd1c4dc22a58a0a9a8e789e2c54b4ace2d1ebc16a605c3976723b52fc011f1"),
            b256!("63477d4b2d525534352ea13c86189340fcb6a51567c8a3a850220b549102e2d5"),
        ),
        (
            b256!("88c74fb93052f3bccc20e2ce709a97a6bd669cbdf1c3a54997b6f5cfda03accc"),
            b256!("7af2f7a50e5f37f8292ed679a760264706cb0caafd57111398055bea41fbeaeb"),
        ),
        (
            b256!("77bd26b8aed0d14e1d78c180196de399840ee7462cb3be6f20981f63284b0bb7"),
            b256!("6f8fde1fc6d437b3088a2d40a40d7bc673ab15bbdbbf3f72bca894856bc9126a"),
        ),
        (
            b256!("4ea1ba2443afd99b07534559feb2d57c390489c2594c2a053e79ff066851db63"),
            b256!("0100fc3604f2bb16343598faa4bbb307c7572b97e55f8b563003e70d1cb3ee68"),
        ),
        (
            b256!("e2154b8a9ad9fdc0e6e23dbd6be53b568cfe15da66d945272bd627ab488bc7fc"),
            b256!("70ddf6f74e2faf7bab6459eb7e32535ec1a3c226bd02610644f8f1e5cefef846"),
        ),
        (
            b256!("18e16e8807c5d723cfa8c1146474c63e0e334abedeb7bf4b1c3dfc44860d2db6"),
            b256!("2b54f99dc26a518209641603ad50d9e100114359e0dfa62a4611e2930639ec87"),
        ),
        (
            b256!("0853345a27fcf3de0d8f77be6465ccd638ab6fceff4f1aa95b2c8e3a20a94843"),
            b256!("d20afc195c1439086fef74cc2406c026fb44db77d7a61d5e6fbac6b2f205b0b0"),
        ),
        (
            b256!("f93cbd9836c96bc35a5f3e97fcd5887fa4cf4fead6d5b5fb796df34b76506d14"),
            b256!("a884be1af18cd8f213f6fa6db255b7281b647b5c8fd4ce984bf97bdc2ef26fa8"),
        ),
        (
            b256!("91ff4738a9ca92dc46ad86cfad7d6e31f678553d965357180537f4653bb48d9b"),
            b256!("95c63b0bd353b902db7138db05febca7ebd1ba112b4b581702f28128a0db5c57"),
        ),
        (
            b256!("2c576b118b1919886188c4a3b1f71143ca8b8a5c1814d8908beb004a6acb9fa1"),
            b256!("26a3e9f1a689222cf9e7f136fd7f7ebaaa712896fe726f9b1a953ad4b67731af"),
        ),
        (
            b256!("53dcdbca6be3f94cd96a8f37900082d04850bfc5a285e9a88b2498f318c89809"),
            b256!("d2f14474f7d3c364abc572439d90230d1f13e4d444b8612711bfa715ea3d9a8d"),
        ),
    ])
}
