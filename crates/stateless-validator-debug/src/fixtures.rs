//! Fixture loading and discovery for the stateless validator debug runner.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use alloy_eips::eip7840::BlobParams;
use alloy_genesis::ChainConfig;
use anyhow::{Context, bail};
use ef_tests::models::ForkSpec;
use reth_chainspec::{Chain, blob_params_to_schedule, create_chain_config};
use serde::Deserialize;
use stateless::StatelessInput;

/// Deserialized JSON fixture supported by the debug runner.
#[derive(Debug, Clone)]
pub struct StatelessValidatorFixture {
    /// Human-readable fixture identifier.
    pub name: String,
    /// Stateless input consumed by the host-side input builders.
    pub input: FixtureInput,
    /// Expected validation outcome.
    pub success: bool,
}

/// Either the legacy in-memory `StatelessInput` (existing JSON layout) or
/// the EEST canonical SSZ payload extracted from a `blockchain_test` JSON.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum FixtureInput {
    /// Legacy `{name, stateless_input, success}` fixture.
    Legacy(StatelessInput),
    /// EEST blockchain-test canonical SSZ bytes.
    Canonical(CanonicalInput),
}

/// EEST canonical-fixture `statelessInputBytes` payload paired with the
/// [`ChainConfig`] needed to resolve the fork from the embedded timestamp.
#[derive(Debug, Clone)]
pub struct CanonicalInput {
    /// SSZ-encoded `SszStatelessInput` bytes.
    pub ssz_bytes: Vec<u8>,
    /// Chain configuration.
    pub chain_config: ChainConfig,
}

/// Wire shape for legacy `{name, stateless_input, success}` fixtures.
#[derive(Debug, Clone, Deserialize)]
struct LegacyFixture {
    name: String,
    stateless_input: StatelessInput,
    success: bool,
}

/// Minimal projection of an EEST `blockchain_test` body — only the fields the
/// debug runner needs.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EestStatelessTest {
    network: String,
    config: EestConfig,
    blocks: Vec<EestStatelessBlock>,
}

#[derive(Debug, Deserialize)]
struct EestConfig {
    #[serde(rename = "chainid", deserialize_with = "deserialize_hex_u64")]
    chain_id: u64,
    #[serde(default, rename = "blobSchedule")]
    blob_schedule: BTreeMap<String, EestBlobParams>,
}

/// Hex-encoded blob-schedule entry as it appears in EEST fixtures.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EestBlobParams {
    #[serde(deserialize_with = "deserialize_hex_u64")]
    target: u64,
    #[serde(deserialize_with = "deserialize_hex_u64")]
    max: u64,
    #[serde(deserialize_with = "deserialize_hex_u128")]
    base_fee_update_fraction: u128,
}

impl From<&EestBlobParams> for BlobParams {
    fn from(p: &EestBlobParams) -> Self {
        BlobParams {
            target_blob_count: p.target,
            max_blob_count: p.max,
            update_fraction: p.base_fee_update_fraction,
            min_blob_fee: 0,
            max_blobs_per_tx: p.max,
            blob_base_cost: 0,
        }
    }
}

fn deserialize_hex_u64<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    let s = String::deserialize(d)?;
    let stripped = s.strip_prefix("0x").unwrap_or(&s);
    u64::from_str_radix(stripped, 16).map_err(serde::de::Error::custom)
}

fn deserialize_hex_u128<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u128, D::Error> {
    let s = String::deserialize(d)?;
    let stripped = s.strip_prefix("0x").unwrap_or(&s);
    u128::from_str_radix(stripped, 16).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EestStatelessBlock {
    #[serde(default)]
    stateless_input_bytes: Option<alloy_primitives::Bytes>,
    #[serde(default)]
    expect_exception: Option<String>,
}

/// Collects fixture file paths from a JSON file or a directory.
pub fn collect_fixture_paths(path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    if path.is_file() {
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            bail!(
                "fixture file {} must have a .json extension",
                path.display()
            );
        }
        return Ok(vec![path.to_path_buf()]);
    }

    if !path.exists() {
        bail!("path {} does not exist", path.display());
    }

    if !path.is_dir() {
        bail!("path {} must be a file or directory", path.display());
    }

    let mut paths = Vec::new();
    collect_json_fixture_paths(path, &mut paths)?;
    paths.sort();

    if paths.is_empty() {
        bail!("no JSON fixtures found in {}", path.display());
    }

    Ok(paths)
}

fn collect_json_fixture_paths(path: &Path, paths: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    let entries = fs::read_dir(path)
        .with_context(|| format!("failed to read fixture directory {}", path.display()))?;

    for entry in entries {
        let entry = entry.with_context(|| format!("failed to read entry in {}", path.display()))?;
        let entry_path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect path {}", entry_path.display()))?;

        if file_type.is_dir() {
            collect_json_fixture_paths(&entry_path, paths)?;
        } else if file_type.is_file()
            && entry_path.extension().and_then(|ext| ext.to_str()) == Some("json")
        {
            paths.push(entry_path);
        }
    }

    Ok(())
}

/// Loads one or more fixtures from a JSON file. Supports two layouts:
/// - The legacy `{name, stateless_input, success}` shape used by repo fixtures.
/// - The EEST `blockchain_test` shape: a top-level map of test-name → `{network, blocks:
///   [{statelessInputBytes, ...}, ...], ...}`. Each `(test, block)` pair becomes one canonical
///   fixture.
pub fn load_fixtures(path: &Path) -> anyhow::Result<Vec<StatelessValidatorFixture>> {
    let bytes =
        fs::read(path).with_context(|| format!("failed to read fixture {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse fixture JSON {}", path.display()))?;

    if value.get("stateless_input").is_some() {
        // Legacy shape.
        let wire: LegacyFixture = serde_json::from_value(value)
            .with_context(|| format!("failed to deserialize legacy fixture {}", path.display()))?;
        return Ok(vec![StatelessValidatorFixture {
            name: wire.name,
            input: FixtureInput::Legacy(wire.stateless_input),
            success: wire.success,
        }]);
    }

    let map: BTreeMap<String, EestStatelessTest> =
        serde_json::from_value(value).with_context(|| {
            format!(
                "fixture {} is neither a legacy fixture nor an EEST blockchain_test",
                path.display(),
            )
        })?;

    let mut out = Vec::new();
    for (test_name, case) in map {
        let chain_config = chain_config_for_test(
            &case.network,
            case.config.chain_id,
            &case.config.blob_schedule,
        )
        .with_context(|| {
            format!(
                "failed to build chain config for {} (network={})",
                path.display(),
                case.network,
            )
        })?;
        // EEST puts `expectException` on the last block when the test is
        // expected to fail — mirrors `eest_generator::gen_fixture`.
        let success = case
            .blocks
            .last()
            .is_some_and(|b| b.expect_exception.is_none());
        for (idx, block) in case.blocks.iter().enumerate() {
            let Some(bytes) = &block.stateless_input_bytes else {
                continue;
            };
            if bytes.is_empty() {
                continue;
            }
            out.push(StatelessValidatorFixture {
                name: format!("{test_name}#block{idx}"),
                input: FixtureInput::Canonical(CanonicalInput {
                    ssz_bytes: bytes.to_vec(),
                    chain_config: chain_config.clone(),
                }),
                success,
            });
        }
    }

    if out.is_empty() {
        bail!(
            "no canonical `statelessInputBytes` found in {}; nothing to run",
            path.display()
        );
    }
    Ok(out)
}

/// Builds an `alloy_genesis::ChainConfig` for a fixture's `network` field.
fn chain_config_for_test(
    network: &str,
    chain_id: u64,
    blob_schedule: &BTreeMap<String, EestBlobParams>,
) -> anyhow::Result<alloy_genesis::ChainConfig> {
    // Construct Amsterdam chain config manually since Reth stateless crate
    // doesn't have Amsterdam support in the main branch. To avoid being
    // blocked by this, construct it manually. Eventually we can remove this.
    if network == "Amsterdam" {
        return Ok(amsterdam_chain_config(chain_id, blob_schedule));
    }

    let fork: ForkSpec = serde_json::from_value(serde_json::Value::String(network.to_string()))
        .with_context(|| format!("unknown fork {network:?}"))?;
    let spec = fork.to_chain_spec();
    let mut cfg = create_chain_config(
        Some(Chain::from_id(chain_id)),
        &spec.hardforks,
        spec.deposit_contract.map(|dc| dc.address),
        blob_params_to_schedule(&spec.blob_params, &spec.hardforks),
    );
    // `Chain::from_id` for a known id (e.g. 1) makes `create_chain_config`
    // copy mainnet's id; force the fixture's value verbatim.
    cfg.chain_id = chain_id;
    Ok(cfg)
}

/// Manually constructs an Amsterdam `alloy_genesis::ChainConfig`. All
/// pre-Amsterdam fork transitions are activated at block/timestamp 0. The
/// blob schedule is lifted verbatim from the fixture (with keys lowercased to
/// match the `to_ethrex_chain_config` lookup convention in `host.rs`).
fn amsterdam_chain_config(
    chain_id: u64,
    blob_schedule: &BTreeMap<String, EestBlobParams>,
) -> alloy_genesis::ChainConfig {
    let blob_schedule = blob_schedule
        .iter()
        .map(|(name, params)| (name.to_ascii_lowercase(), BlobParams::from(params)))
        .collect();

    alloy_genesis::ChainConfig {
        chain_id,
        homestead_block: Some(0),
        dao_fork_block: None,
        dao_fork_support: false,
        eip150_block: Some(0),
        eip155_block: Some(0),
        eip158_block: Some(0),
        byzantium_block: Some(0),
        constantinople_block: Some(0),
        petersburg_block: Some(0),
        istanbul_block: Some(0),
        muir_glacier_block: Some(0),
        berlin_block: Some(0),
        london_block: Some(0),
        arrow_glacier_block: Some(0),
        gray_glacier_block: Some(0),
        merge_netsplit_block: Some(0),
        shanghai_time: Some(0),
        cancun_time: Some(0),
        prague_time: Some(0),
        osaka_time: Some(0),
        bpo1_time: Some(0),
        bpo2_time: Some(0),
        bpo3_time: None,
        bpo4_time: None,
        bpo5_time: None,
        amsterdam_time: Some(0),
        terminal_total_difficulty: Some(alloy_primitives::U256::ZERO),
        terminal_total_difficulty_passed: true,
        blob_schedule,
        deposit_contract_address: None,
        ..Default::default()
    }
}
