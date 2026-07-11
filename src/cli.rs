use std::fmt;
use std::path::PathBuf;
use std::thread::available_parallelism;

use clap::{Args, Parser, Subcommand, ValueEnum};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::mode::FpMode;

/// A CPU architecture a backend can be built and measured on.
///
/// The two this project supports, because they are the two the ISA rule is
/// written for. See `METHODOLOGY.md#the-isa-rule`.
///
/// This exists because a toolchain is allowed to be *missing*. Kotlin/Native
/// publishes no `linux-aarch64` host compiler, so a backend using it cannot be
/// built on an AArch64 bench machine at all — not slowly, not under emulation
/// (which this project forbids outright): it simply does not exist there. That is
/// a fact about the backend, so the backend declares it, and a campaign on the
/// other architecture skips the row and says why. The alternative is a `docker
/// build` that fails halfway through a campaign with a 404.
#[derive(
    Clone, Copy, Debug, Deserialize, Eq, Hash, JsonSchema, PartialEq, Serialize, ValueEnum,
)]
#[serde(rename_all = "snake_case")]
#[schemars(rename_all = "snake_case")]
pub enum Arch {
    /// 64-bit x86. `-march=x86-64-v3` and friends.
    X86_64,
    /// 64-bit ARM. `-march=armv8.2-a` and friends.
    Aarch64,
}

impl Arch {
    pub const ALL: [Self; 2] = [Self::X86_64, Self::Aarch64];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::Aarch64 => "aarch64",
        }
    }

    /// Parse an architecture as a `bench.yaml` spells it.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|arch| arch.as_str() == value)
    }

    /// The architecture the harness is running on — which is the architecture the
    /// containers will run on, since the harness never builds for another one.
    /// Cross-building would mean measuring under emulation, and this project does
    /// not do that.
    ///
    /// `None` on anything else: the ISA rule only knows these two, and a campaign
    /// on a third has no baseline to pin.
    pub fn current() -> Option<Self> {
        Self::parse(std::env::consts::ARCH)
    }
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Parser)]
#[command(name = "langbench", version, about, long_about = None)]
pub struct Cli {
    /// `tracing` filter directive (e.g. `info`, `langbench=debug`).
    ///
    /// Syntax: <https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives>
    #[arg(long = "log-filter", env = "LOG_FILTER", default_value = "info")]
    pub log_filter: String,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Build every selected implementation and measure it.
    ///
    /// Writes `samples.ndjson` and nothing else. Render it with `csv` or `md`.
    Run(RunArgs),

    /// Render a campaign's samples as CSV, for a spreadsheet or a dataframe.
    Csv(CsvArgs),

    /// Render a campaign's samples as a Markdown report.
    Md(MarkdownArgs),

    /// Check every `bench.yaml` on disk, and report all of them at once.
    ///
    /// Every failure a campaign would hit at discovery time — a misspelled key,
    /// an unknown FP mode, a backend that neither compiles nor interprets, two
    /// manifests declaring the same identity — without building anything.
    Validate(ValidateArgs),

    /// Write the JSON Schema of a `bench.yaml`.
    ///
    /// Generated from the struct the harness deserializes, so it cannot drift
    /// from what the campaign actually accepts. Point an editor at it and get
    /// completion and validation as you type a manifest.
    Jsonschema(JsonSchemaArgs),

    /// Describe this machine, and why it may be a poor benchmark target.
    ///
    /// Prints exactly what a campaign would record in its header, so you can
    /// check a host before spending an hour measuring on it.
    Machine,
}

/// The one file a campaign writes, and the one file a rendering reads.
/// `SAMPLES_OUTPUT` names it on both sides, so a `run` and the `md` that follows
/// agree without being told twice.
pub const DEFAULT_SAMPLES_OUTPUT: &str = "samples.ndjson";

/// Where `langbench csv` writes its table.
pub const DEFAULT_CSV_OUTPUT: &str = "samples.csv";

/// Where `langbench md` writes its report.
pub const DEFAULT_MD_OUTPUT: &str = "report.md";

/// Where `langbench jsonschema` writes the manifest schema. At the repo root,
/// because that is where an editor's `yaml.schemas` mapping looks for it, and a
/// pre-commit hook keeps it honest.
pub const DEFAULT_SCHEMA_OUTPUT: &str = "bench.schema.json";

#[derive(Args, Debug)]
pub struct ValidateArgs {
    /// Manifests to check, or directories to walk. Defaults to the whole tree.
    ///
    /// Give it directories, not a hand-picked list of changed files: the
    /// duplicate-identity check can only see a collision if it sees both halves.
    #[arg(
        value_name = "PATHS",
        env = "BENCHMARKS_DIR",
        default_values_os_t = vec![PathBuf::from("benchmarks")],
    )]
    pub paths: Vec<PathBuf>,
}

#[derive(Args, Debug)]
pub struct JsonSchemaArgs {
    /// Path of the schema to write. Missing parent directories are created.
    #[arg(long, short, env = "SCHEMA_OUTPUT", default_value_os_t = PathBuf::from(DEFAULT_SCHEMA_OUTPUT))]
    pub output: PathBuf,
}

/// Reading a campaign back. Rendering is never part of measuring: the samples
/// are the source of truth, and every artifact is recomputed from them.
///
/// The samples are an *input* here, so they keep their own name —
/// `SAMPLES_OUTPUT`, the value `run` wrote — and each rendering names its own
/// destination separately.
#[derive(Args, Debug)]
pub struct RenderArgs {
    /// The `samples.ndjson` a campaign wrote.
    #[arg(
        value_name = "SAMPLES",
        env = "SAMPLES_OUTPUT",
        default_value_os_t = PathBuf::from(DEFAULT_SAMPLES_OUTPUT),
    )]
    pub samples: PathBuf,
}

#[derive(Args, Debug)]
pub struct CsvArgs {
    #[command(flatten)]
    pub render: RenderArgs,

    /// Path of the CSV to write. Missing parent directories are created.
    #[arg(long, short, env = "CSV_OUTPUT", default_value_os_t = PathBuf::from(DEFAULT_CSV_OUTPUT))]
    pub output: PathBuf,
}

#[derive(Args, Debug)]
pub struct MarkdownArgs {
    #[command(flatten)]
    pub render: RenderArgs,

    /// Path of the report to write. Missing parent directories are created.
    #[arg(long, short, env = "MD_OUTPUT", default_value_os_t = PathBuf::from(DEFAULT_MD_OUTPUT))]
    pub output: PathBuf,

    /// A Liquid template to render instead of the built-in one.
    ///
    /// It receives exactly the same variables. The built-in one lives at
    /// `templates/report.md.liquid`; copy it as a starting point.
    #[arg(long, env = "TEMPLATE")]
    pub template: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct RunArgs {
    /// Algorithms to measure. Defaults to every algorithm discovered on disk.
    #[arg(long, env = "ALGO", value_delimiter = ',')]
    pub algo: Vec<String>,

    /// Floating-point modes to build and measure.
    #[arg(long, env = "FP_MODE", value_delimiter = ',', default_values_t = FpMode::ALL)]
    pub mode: Vec<FpMode>,

    /// Threads handed to the kernels and to the compilers.
    ///
    /// The harness resolves a default and passes it explicitly; the kernels
    /// themselves must never auto-detect. See `METHODOLOGY.md#anti-cheating-contract`.
    #[arg(long, env = "CPUS", default_value_t = default_cpu())]
    pub cpu: usize,

    /// Path of the `samples.ndjson` this campaign writes: its only artifact.
    ///
    /// Missing parent directories are created.
    #[arg(long, short, env = "SAMPLES_OUTPUT", default_value_os_t = PathBuf::from(DEFAULT_SAMPLES_OUTPUT))]
    pub output: PathBuf,

    /// Root of the tree to walk for `bench.yaml` manifests.
    #[arg(long, env = "BENCHMARKS_DIR", default_value_os_t = PathBuf::from("benchmarks"))]
    pub benchmarks_dir: PathBuf,

    /// Side of the N x N grid.
    ///
    /// The default is sized for iteration speed, not for a final campaign: the
    /// work scales as `grid_size^2 * max_iter`, and the slowest backend
    /// (CPython) is what a campaign actually waits on. Raise it to `4096` when
    /// publishing numbers.
    #[arg(long, env = "GRID_SIZE", default_value_t = 2048)]
    pub grid_size: u32,

    /// Iteration ceiling before a pixel is declared inside the set.
    #[arg(long, env = "MAX_ITER", default_value_t = 1000)]
    pub max_iter: u32,

    /// Measured rounds of the run phase.
    ///
    /// The estimate is a min-of-N: more rounds only ever lower it, so a small N
    /// is a faster but slightly pessimistic campaign, never a wrong one. The
    /// dispersion published beside it says whether N was large enough.
    #[arg(long, env = "ROUNDS", default_value_t = 10)]
    pub rounds: u32,

    /// Measured rounds of the build phase. Builds are slow; fewer suffice.
    #[arg(long, env = "BUILD_ROUNDS", default_value_t = 3)]
    pub build_rounds: u32,

    /// Rounds recorded but flagged as warmup, for both phases.
    #[arg(long, env = "WARMUP_ROUNDS", default_value_t = 1)]
    pub warmup_rounds: u32,

    /// ISA baseline passed to the compilers as `-march`. Never `native`.
    #[arg(long, env = "MARCH", default_value_t = default_march(), value_parser = parse_march)]
    pub march: String,

    /// Size of the tmpfs mounted on the container's build directory.
    #[arg(long, env = "TMPFS_SIZE_MB", default_value_t = 2048)]
    pub tmpfs_size_mb: u64,

    /// Wall-clock ceiling for a single container invocation, in seconds.
    ///
    /// A container that exceeds it is killed and the campaign fails. Without
    /// this, a deadlocked run is indistinguishable from a slow one and blocks
    /// the campaign forever.
    #[arg(long, env = "RUN_TIMEOUT", default_value_t = 600)]
    pub run_timeout: u64,
}

/// The machine's parallelism, used only as the *default* for `--cpu`.
fn default_cpu() -> usize {
    available_parallelism().map(|n| n.get()).unwrap_or(1)
}

/// A pinned ISA baseline per architecture. Empty means "pass no `-march`".
fn default_march() -> String {
    match std::env::consts::ARCH {
        "x86_64" => "x86-64-v3",
        "aarch64" => "armv8.2-a",
        _ => "",
    }
    .to_owned()
}

fn parse_march(value: &str) -> Result<String, String> {
    if value.eq_ignore_ascii_case("native") {
        return Err(
            "`-march=native` is forbidden: the CPU model varies between \
             runs and the ISA baseline would vary with it. Pin an explicit \
             baseline, e.g. `x86-64-v3`."
                .to_owned(),
        );
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_march_rejects_native() {
        assert!(parse_march("native").is_err());
        assert!(parse_march("NATIVE").is_err());
    }

    #[test]
    fn parse_march_accepts_a_pinned_baseline() {
        assert_eq!(parse_march("x86-64-v3").unwrap(), "x86-64-v3");
    }
}
