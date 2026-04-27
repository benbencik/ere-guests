//! Implementations for host environment.

use alloy_eips::eip6110::MAINNET_DEPOSIT_CONTRACT_ADDRESS;
use anyhow::{Context, ensure};
use ethrex_common::{
    H160,
    types::{
        BlobSchedule, ChainConfig, ForkBlobSchedule,
        block_execution_witness::{self, RpcExecutionWitness},
    },
};
use stateless_validator_common::new_payload_request::{ForkName, NewPayloadRequest};
use stateless_validator_reth::{guest::StatelessValidatorRethInput, host::determine_fork_name};

#[rustfmt::skip]
pub use {
    ethrex_guest_program::input::ProgramInput,
    stateless::StatelessInput,
};

/// Source for the EIP-8025 host input buffer.
#[derive(Debug)]
pub enum Eip8025InputSource<'a> {
    /// Legacy `StatelessInput` (with witness etc.) plus expected validity flag.
    Legacy {
        /// The stateless input.
        stateless_input: &'a StatelessInput,
        /// Whether the block is expected to validate successfully.
        valid_block: bool,
    },
    /// EEST canonical `statelessInputBytes` SSZ payload + chain config.
    Canonical {
        /// SSZ-encoded EEST `statelessInputBytes`.
        ssz_input: &'a [u8],
        /// Chain config sourced from the fixture.
        chain_config: &'a alloy_genesis::ChainConfig,
    },
}

/// Builds the local EIP-8025 input buffer for the ethrex guest.
pub fn build_eip8025_input(source: Eip8025InputSource<'_>) -> anyhow::Result<Vec<u8>> {
    match source {
        Eip8025InputSource::Legacy {
            stateless_input,
            valid_block,
        } => build_legacy(stateless_input, valid_block),
        Eip8025InputSource::Canonical {
            ssz_input,
            chain_config,
        } => build_canonical(ssz_input, chain_config),
    }
}

fn build_legacy(stateless_input: &StatelessInput, valid_block: bool) -> anyhow::Result<Vec<u8>> {
    let fork = determine_fork_name(
        &stateless_input.chain_config,
        stateless_input.block.header.timestamp,
    );
    ensure!(
        matches!(fork, ForkName::Electra | ForkName::Fulu),
        "ethrex EIP-8025 input only supports Electra/Fulu fixtures, got {fork:?}"
    );

    let reth_input = StatelessValidatorRethInput::new(stateless_input, valid_block)?;
    let new_payload_request = match reth_input.new_payload_request {
        NewPayloadRequest::ElectraFulu(new_payload_request) => new_payload_request,
        _ => unreachable!("fork gate above guarantees an Electra/Fulu payload"),
    };

    let execution_witness =
        from_reth_witness_to_ethrex_witness(stateless_input.block.number, stateless_input)?;
    let witness_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&execution_witness)
        .map_err(|err| anyhow::anyhow!("failed to rkyv-encode execution witness: {err}"))?;

    Ok(crate::wire::encode_eip8025(
        &new_payload_request,
        witness_bytes.as_ref(),
    ))
}

fn build_canonical(
    ssz_input: &[u8],
    chain_config: &alloy_genesis::ChainConfig,
) -> anyhow::Result<Vec<u8>> {
    let ethrex_cfg = to_ethrex_chain_config(chain_config);
    let cfg_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&ethrex_cfg)
        .map_err(|err| anyhow::anyhow!("failed to rkyv-encode chain config: {err}"))?;

    Ok(crate::wire::encode_eip8025_canonical(
        ssz_input,
        cfg_bytes.as_ref(),
    ))
}

fn to_ethrex_chain_config(cfg: &alloy_genesis::ChainConfig) -> ChainConfig {
    ChainConfig {
        chain_id: cfg.chain_id,
        homestead_block: cfg.homestead_block,
        dao_fork_block: cfg.dao_fork_block,
        dao_fork_support: cfg.dao_fork_support,
        eip150_block: cfg.eip150_block,
        eip155_block: cfg.eip155_block,
        eip158_block: cfg.eip158_block,
        byzantium_block: cfg.byzantium_block,
        constantinople_block: cfg.constantinople_block,
        petersburg_block: cfg.petersburg_block,
        istanbul_block: cfg.istanbul_block,
        muir_glacier_block: cfg.muir_glacier_block,
        berlin_block: cfg.berlin_block,
        london_block: cfg.london_block,
        arrow_glacier_block: cfg.arrow_glacier_block,
        gray_glacier_block: cfg.gray_glacier_block,
        merge_netsplit_block: cfg.merge_netsplit_block,
        shanghai_time: cfg.shanghai_time,
        cancun_time: cfg.cancun_time,
        prague_time: cfg.prague_time,
        verkle_time: None,
        osaka_time: cfg.osaka_time,
        terminal_total_difficulty: cfg
            .terminal_total_difficulty
            .map(|ttd| TryInto::<u128>::try_into(ttd).unwrap()),
        terminal_total_difficulty_passed: cfg.terminal_total_difficulty_passed,
        blob_schedule: BlobSchedule {
            cancun: get_blob_schedule(cfg, "cancun")
                .unwrap_or_else(|| BlobSchedule::default().cancun),
            prague: get_blob_schedule(cfg, "prague")
                .unwrap_or_else(|| BlobSchedule::default().prague),
            osaka: get_blob_schedule(cfg, "osaka").unwrap_or_else(|| BlobSchedule::default().osaka),
            bpo1: get_blob_schedule(cfg, "bpo1").unwrap_or_else(|| BlobSchedule::default().bpo1),
            bpo2: get_blob_schedule(cfg, "bpo2").unwrap_or_else(|| BlobSchedule::default().bpo2),
            bpo3: get_blob_schedule(cfg, "bpo3"),
            bpo4: get_blob_schedule(cfg, "bpo4"),
            bpo5: get_blob_schedule(cfg, "bpo5"),
            amsterdam: get_blob_schedule(cfg, "amsterdam"),
        },
        deposit_contract_address: cfg
            .deposit_contract_address
            .map(|addr| H160::from_slice(addr.as_slice()))
            .unwrap_or_else(|| H160::from_slice(MAINNET_DEPOSIT_CONTRACT_ADDRESS.as_slice())),
        bpo1_time: cfg.bpo1_time,
        bpo2_time: cfg.bpo2_time,
        bpo3_time: cfg.bpo3_time,
        bpo4_time: cfg.bpo4_time,
        bpo5_time: cfg.bpo5_time,
        amsterdam_time: cfg.amsterdam_time,
        enable_verkle_at_genesis: false,
    }
}

fn from_reth_witness_to_ethrex_witness(
    block_number: u64,
    stateless_input: &StatelessInput,
) -> anyhow::Result<block_execution_witness::ExecutionWitness> {
    let codes = stateless_input
        .witness
        .codes
        .iter()
        .map(|b| b.to_vec().into())
        .collect();
    let block_headers_bytes = stateless_input
        .witness
        .headers
        .iter()
        .map(|h| h.to_vec().into())
        .collect();

    let chain_config = to_ethrex_chain_config(&stateless_input.chain_config);

    let nodes = stateless_input
        .witness
        .state
        .iter()
        .map(|node_rlp| node_rlp.to_vec().into())
        .collect();

    let keys = stateless_input
        .witness
        .keys
        .iter()
        .map(|k| k.to_vec().into())
        .collect();

    let rpc_witness = RpcExecutionWitness {
        state: nodes,
        keys,
        codes,
        headers: block_headers_bytes,
    };

    rpc_witness
        .into_execution_witness(chain_config, block_number)
        .context("failed to convert reth witness into ethrex witness")
}

fn get_blob_schedule(
    chain_config: &alloy_genesis::ChainConfig,
    name: &str,
) -> Option<ethrex_common::types::ForkBlobSchedule> {
    chain_config
        .blob_schedule
        .get(name)
        .map(|s| ForkBlobSchedule {
            // Reth and Ethrex have some mismatched data type representations. Reth uses bigger
            // ints. Downcasting should never cause an overflow, but let's be safe and
            // panic if this ever happens.
            base_fee_update_fraction: s.update_fraction.try_into().unwrap(),
            target: s.target_blob_count.try_into().unwrap(),
            max: s.max_blob_count.try_into().unwrap(),
        })
}
