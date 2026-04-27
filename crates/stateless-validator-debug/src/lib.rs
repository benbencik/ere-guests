//! Host-side debug runner for stateless validator guest fixtures.

mod fixtures;

use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, bail};
use clap::{Parser, ValueEnum};
pub use fixtures::{
    CanonicalInput, FixtureInput, StatelessValidatorFixture, collect_fixture_paths, load_fixtures,
};
use guest::{Guest, Platform};
use stateless_validator_ethrex::{
    guest::StatelessValidatorEthrexGuest,
    host::{Eip8025InputSource, build_eip8025_input},
};
use stateless_validator_reth::guest::{
    StatelessValidatorOutput, StatelessValidatorRethGuest, StatelessValidatorRethInput,
};
use tracing_subscriber::EnvFilter;

/// CLI options for the stateless validator debug runner.
#[derive(Debug, Clone, PartialEq, Eq, Parser)]
#[command(
    name = "stateless-validator-debug",
    about = "Run stateless validator guest fixtures directly on the host.",
    long_about = None,
    arg_required_else_help = true
)]
pub struct Cli {
    /// Guest program to run.
    #[arg(long, value_enum)]
    pub guest: GuestKind,
    /// Warn and continue when fixture success does not match guest output.
    #[arg(long)]
    pub allow_success_mismatch: bool,
    /// Path to a fixture file or directory.
    pub path: PathBuf,
}

/// Stateless validator guest selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum GuestKind {
    /// Run the Reth stateless validator guest.
    Reth,
    /// Run the Ethrex stateless validator guest.
    Ethrex,
}

impl GuestKind {
    fn run_fixture(self, fixture: &StatelessValidatorFixture) -> anyhow::Result<RunSummary> {
        let output: StatelessValidatorOutput = match self {
            Self::Reth => match &fixture.input {
                FixtureInput::Legacy(stateless_input) => {
                    let input = StatelessValidatorRethInput::new(stateless_input, fixture.success)?;
                    StatelessValidatorRethGuest::compute::<StdoutNoopPlatform>(input)
                }
                FixtureInput::Canonical(_) => {
                    bail!("reth guest does not yet accept EEST canonical SSZ input")
                }
            },
            Self::Ethrex => {
                let source = match &fixture.input {
                    FixtureInput::Legacy(stateless_input) => Eip8025InputSource::Legacy {
                        stateless_input,
                        valid_block: fixture.success,
                    },
                    FixtureInput::Canonical(canonical) => Eip8025InputSource::Canonical {
                        ssz_input: &canonical.ssz_bytes,
                        chain_config: &canonical.chain_config,
                    },
                };
                let input = build_eip8025_input(source)?;
                StatelessValidatorEthrexGuest::compute::<StdoutNoopPlatform>(input)
            }
        };

        Ok(RunSummary {
            fixture_name: fixture.name.clone(),
            guest: self,
            expected_success: fixture.success,
            actual_success: output.successful_block_validation,
            new_payload_request_root: output.new_payload_request_root,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Reth => "reth",
            Self::Ethrex => "ethrex",
        }
    }
}

/// Summary of one guest execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSummary {
    /// Name of the fixture that ran.
    pub fixture_name: String,
    /// Guest program that was executed.
    pub guest: GuestKind,
    /// Expected guest success from the fixture.
    pub expected_success: bool,
    /// Actual guest success reported by the guest output.
    pub actual_success: bool,
    /// The resulting new payload request root.
    pub new_payload_request_root: [u8; 32],
}

impl std::fmt::Display for RunSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "fixture={} guest={} expected_success={} actual_success={} new_payload_request_root=0x{}",
            self.fixture_name,
            self.guest.as_str(),
            self.expected_success,
            self.actual_success,
            encode_hex(&self.new_payload_request_root),
        )
    }
}

/// A no-op platform for host-side guest execution that forwards debug messages to stdout.
#[derive(Debug)]
pub struct StdoutNoopPlatform;

impl Platform for StdoutNoopPlatform {
    #[allow(unreachable_code)]
    fn read_whole_input() -> impl std::ops::Deref<Target = [u8]> {
        panic!("Can't read input in StdoutNoopPlatform");
        &[] as &[u8]
    }

    fn write_whole_output(_: &[u8]) {
        panic!("Can't write output in StdoutNoopPlatform");
    }

    fn print(message: &str) {
        println!("{message}");
        let _ = io::stdout().flush();
    }
}

/// Entry point for the debug runner binary.
pub fn main_entry() -> anyhow::Result<()> {
    init_tracing();
    execute(Cli::parse(), |summary| println!("{summary}"))
}

/// Executes one or more fixtures and reports each summary via `on_summary`.
pub fn execute(cli: Cli, mut on_summary: impl FnMut(&RunSummary)) -> anyhow::Result<()> {
    let fixture_paths = collect_fixture_paths(&cli.path)?;

    for fixture_path in fixture_paths {
        let fixtures = load_fixtures(&fixture_path)?;
        for fixture in &fixtures {
            let summary = cli
                .guest
                .run_fixture(fixture)
                .with_context(|| format!("failed to execute fixture {}", fixture_path.display()))?;
            on_summary(&summary);

            handle_success_mismatch(&summary, &fixture_path, cli.allow_success_mismatch)?;
        }
    }

    Ok(())
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        )
        .try_init();
}

fn handle_success_mismatch(
    summary: &RunSummary,
    fixture_path: &Path,
    allow_success_mismatch: bool,
) -> anyhow::Result<()> {
    if summary.actual_success == summary.expected_success {
        return Ok(());
    }

    if allow_success_mismatch {
        tracing::warn!(
            fixture_name = summary.fixture_name.as_str(),
            fixture_path = %fixture_path.display(),
            expected_success = summary.expected_success,
            actual_success = summary.actual_success,
            "fixture success mismatch",
        );
        return Ok(());
    }

    bail!(
        "fixture {} ({}) expected success={}, got success={}",
        summary.fixture_name,
        fixture_path.display(),
        summary.expected_success,
        summary.actual_success,
    );
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;

        let _ = write!(hex, "{byte:02x}");
    }
    hex
}
