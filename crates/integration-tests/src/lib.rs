//! Integration test lib.

use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

use alloy_consensus::{Block, BlockBody, Header};
use alloy_eips::eip4895::Withdrawals;
use alloy_genesis::ChainConfig;
use ere_dockerized::{
    Compiler, CompilerKind, DockerizedCompiler, DockerizedzkVM, DockerizedzkVMConfig, Input,
    ProverResource, codec::Encode, zkVMKind,
};
use flate2::read::GzDecoder;
use guest::{Guest, GuestInput, GuestOutput, Platform};
use rayon::prelude::*;
use reth_ethereum_primitives::TransactionSigned;
use serde::Deserialize;
use serde_with::serde_as;
use sha2::{Digest, Sha256};
use stateless::{ExecutionWitness, StatelessInput};
use tar::Archive;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::stateless_validator::StatelessValidatorFixture;

pub mod stateless_validator;

/// Returns path to workspace
pub fn workspace() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path
}

/// Returns path to fixtures dir.
pub fn fixtures_dir() -> PathBuf {
    workspace().join("crates/integration-tests/fixtures")
}

/// Unpack all fixtures in fixtures dir.
pub fn untar_fixtures(target_dir: &Path) -> std::io::Result<()> {
    let fixtures_dir = fixtures_dir();

    for entry in fs::read_dir(&fixtures_dir)? {
        let path = entry?.path();
        let filename = path.file_name().and_then(|filename| filename.to_str());
        if filename.is_some_and(|file_name| file_name.ends_with(".tar.gz")) {
            let file = File::open(&path)?;
            let gz = GzDecoder::new(file);
            Archive::new(gz).unpack(target_dir)?;
        }
    }

    Ok(())
}

/// Reads all stateless validator fixtures.
pub fn get_fixtures() -> Vec<StatelessValidatorFixture> {
    let dir = tempfile::tempdir().unwrap();
    let dir_path = dir.path();
    untar_fixtures(dir_path).unwrap();
    fs::read_dir(dir_path.join("block"))
        .unwrap()
        .map(|file| {
            let bytes = fs::read(file.unwrap().path()).unwrap();
            let fixture: StatelessValidatorFixture =
                deserialize_stateless_validator_fixture(&bytes).unwrap();
            fixture
        })
        .collect()
}

fn deserialize_stateless_validator_fixture(
    bytes: &[u8],
) -> serde_json::Result<StatelessValidatorFixture> {
    serde_json::from_slice(bytes).or_else(|_| {
        serde_json::from_slice::<CompatStatelessValidatorFixture>(bytes).map(Into::into)
    })
}

#[derive(Debug, Deserialize)]
struct CompatStatelessValidatorFixture {
    name: String,
    stateless_input: CompatStatelessInput,
    success: bool,
}

impl From<CompatStatelessValidatorFixture> for StatelessValidatorFixture {
    fn from(value: CompatStatelessValidatorFixture) -> Self {
        Self {
            name: value.name,
            stateless_input: value.stateless_input.into(),
            success: value.success,
        }
    }
}

#[serde_as]
#[derive(Debug, Deserialize)]
struct CompatStatelessInput {
    block: CompatBlock,
    witness: ExecutionWitness,
    #[serde_as(as = "alloy_genesis::serde_bincode_compat::ChainConfig<'_>")]
    chain_config: ChainConfig,
}

impl From<CompatStatelessInput> for StatelessInput {
    fn from(value: CompatStatelessInput) -> Self {
        Self {
            block: value.block.into(),
            witness: value.witness,
            chain_config: value.chain_config,
        }
    }
}

#[serde_as]
#[derive(Debug, Deserialize)]
struct CompatBlock {
    #[serde_as(as = "alloy_consensus::serde_bincode_compat::Header")]
    header: Header,
    body: CompatBlockBody,
}

impl From<CompatBlock> for Block<TransactionSigned> {
    fn from(value: CompatBlock) -> Self {
        Self {
            header: value.header,
            body: value.body.into(),
        }
    }
}

#[serde_as]
#[derive(Debug, Deserialize)]
struct CompatBlockBody {
    #[serde_as(as = "Vec<alloy_consensus::serde_bincode_compat::EthereumTxEnvelope<'_>>")]
    transactions: Vec<TransactionSigned>,
    #[serde_as(as = "Vec<alloy_consensus::serde_bincode_compat::Header>")]
    ommers: Vec<Header>,
    withdrawals: Option<Withdrawals>,
}

impl From<CompatBlockBody> for BlockBody<TransactionSigned> {
    fn from(value: CompatBlockBody) -> Self {
        Self {
            transactions: value.transactions,
            ommers: value.ommers,
            withdrawals: value.withdrawals,
        }
    }
}

/// Compiles guest program and initialize zkVM.
pub fn compile_and_init_zkvm(guest: &str, zkvm_kind: zkVMKind) -> DockerizedzkVM {
    let workspace = workspace();

    let compiler =
        DockerizedCompiler::new(zkvm_kind, CompilerKind::RustCustomized, &workspace).unwrap();
    let bin = workspace.join("bin").join(guest).join(zkvm_kind.as_str());
    let program = compiler.compile(&bin).unwrap();

    DockerizedzkVM::new(
        zkvm_kind,
        program,
        ProverResource::Cpu,
        DockerizedzkVMConfig::default(),
    )
    .unwrap()
}

/// Compiles guest program and runs execution, then check output are expected.
pub fn test_execution(
    guest: &str,
    zkvm_kind: zkVMKind,
    test_cases: impl IntoIterator<Item = TestCase>,
) {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    let test_cases = test_cases.into_iter().collect::<Vec<_>>();
    assert!(!test_cases.is_empty());

    let zkvm = compile_and_init_zkvm(guest, zkvm_kind);

    test_cases.into_par_iter().for_each(|test_case| {
        info!("Running execution of test case {}", test_case.name);

        let (public_values, report) = zkvm.execute(&test_case.input).unwrap();

        info!(
            "Execution of test case {} took {:?}",
            test_case.name, report.execution_duration
        );

        let mut expected_public_values = test_case.expected_public_values;

        // Add padding for those zkVMs that have fixed size public values.
        if matches!(zkvm_kind, zkVMKind::Airbender | zkVMKind::OpenVM)
            && expected_public_values.len() < 32
        {
            expected_public_values.resize(32, 0);
        }

        if matches!(zkvm_kind, zkVMKind::Zisk) && expected_public_values.len() < 256 {
            expected_public_values.resize(256, 0);
        }

        assert_eq!(
            public_values.0, expected_public_values,
            "Expected public values of test case {} to be \
                {expected_public_values:?}, but got {public_values:?}",
            test_case.name
        );
    });
}

/// Guest program test case.
#[derive(Debug, Default)]
pub struct TestCase {
    /// Identifier of the test case.
    name: String,
    /// [`Input`] of the guest program.
    input: Input,
    /// The expected public values of guest program.
    expected_public_values: Vec<u8>,
}

impl TestCase {
    /// Constructs a new [`TestCase`].
    pub fn new<G: Guest>(
        name: impl AsRef<str>,
        input: GuestInput<G>,
        output: GuestOutput<G>,
    ) -> Self {
        Self {
            name: name.as_ref().to_string(),
            input: Input::new().with_prefixed_stdin(input.encode_to_vec().unwrap()),
            expected_public_values: output.encode_to_vec().unwrap(),
        }
    }

    /// Consumes the [`TestCase`] and constructs a new one with sha256 output.
    pub fn output_sha256(mut self) -> Self {
        self.expected_public_values = Sha256::digest(self.expected_public_values).to_vec();
        self
    }
}
/// A platform that to run guests outside zkVMs.
#[derive(Debug)]
pub struct NoopPlatform;

impl Platform for NoopPlatform {
    #[allow(unreachable_code)]
    fn read_whole_input() -> impl std::ops::Deref<Target = [u8]> {
        panic!("Can't read input in NoopPlatform");
        &[] as &[u8]
    }

    fn write_whole_output(_: &[u8]) {
        panic!("Can't write output in NoopPlatform");
    }

    fn print(message: &str) {
        println!("{}", message);
    }
}
