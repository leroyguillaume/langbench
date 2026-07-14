use std::fmt;
use std::path::PathBuf;
use std::thread::available_parallelism;

use clap::{Args, Parser, Subcommand, ValueEnum};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::mode::Mode;

/// A CPU architecture a backend can be built and measured on.
///
/// The two this project supports, because they are the two the architecture rule is
/// written for. See `site/src/content/methodology.md#the-architecture`.
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
pub enum Architecture {
    /// 64-bit x86. `-march=x86-64-v3` and friends.
    X86_64,
    /// 64-bit ARM. `-march=armv8.2-a` and friends.
    Aarch64,
}

impl Architecture {
    pub const ALL: [Self; 2] = [Self::X86_64, Self::Aarch64];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::Aarch64 => "aarch64",
        }
    }

    /// Parse an architecture as a `bench.yaml` spells it.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|architecture| architecture.as_str() == value)
    }

    /// The architecture the harness is running on — which is the architecture the
    /// containers will run on, since the harness never builds for another one.
    /// Cross-building would mean measuring under emulation, and this project does
    /// not do that.
    ///
    /// `None` on anything else: the architecture rule only knows these two, and a campaign
    /// on a third has no baseline to pin.
    pub fn current() -> Option<Self> {
        Self::parse(std::env::consts::ARCH)
    }
}

impl fmt::Display for Architecture {
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

/// Three resources, and the verbs that act on them.
///
/// `workload` and `implementation` read the tree — what could be measured.
/// `sample` reads a campaign — what was. Neither family can be folded into the
/// other: a workload does not convert, and a campaign is not on disk until one has
/// run. The two that are left over, `validate` and `machine`, belong to no resource
/// because they are about *everything* — every manifest at once, and the host under
/// all of them.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// The work: list the workloads, or run a campaign on one.
    #[command(subcommand)]
    Workload(WorkloadCommand),

    /// The backends that do the work, for a given workload.
    #[command(subcommand, alias = "impl")]
    Implementation(ImplementationCommand),

    /// The samples a campaign recorded. Never measures anything.
    #[command(subcommand)]
    Sample(SampleCommand),

    /// Check every manifest on disk, and report all of them at once.
    ///
    /// Every failure a campaign would hit at discovery time — a misspelled key, an
    /// unknown mode, a backend that neither compiles nor interprets, two
    /// manifests declaring the same identity, a `bench.yaml` no workload lists —
    /// without building anything.
    ///
    /// It takes the whole tree, and not the files that changed: two backends collide
    /// with *each other*, and an undeclared manifest can only be seen by someone
    /// holding both the tree and the declarations.
    Validate(ValidateArgs),

    /// Describe this machine, and why it may be a poor benchmark target.
    ///
    /// Prints exactly what a campaign would record in its header, so you can
    /// check a host before spending an hour measuring on it.
    Machine,
}

#[derive(Debug, Subcommand)]
pub enum WorkloadCommand {
    /// Every workload declared on disk: its id, what it is, how it is sized.
    List(ListArgs),

    /// Build every implementation of a workload and measure it.
    ///
    /// Writes `samples.ndjson` and nothing else — one machine, one workload, one
    /// campaign. Read it on the website, or convert it with `langbench sample
    /// convert`.
    Run(Box<RunArgs>),

    /// Write the JSON Schema of a `workload.yaml`.
    Jsonschema(JsonSchemaArgs),
}

#[derive(Debug, Subcommand)]
pub enum ImplementationCommand {
    /// Every implementation a workload declares, and what each one is.
    List(ImplementationListArgs),

    /// Write the JSON Schema of a `bench.yaml`.
    ///
    /// Generated from the struct the harness deserializes, so it cannot drift
    /// from what the campaign actually accepts. Point an editor at it and get
    /// completion and validation as you type a manifest.
    Jsonschema(JsonSchemaArgs),
}

#[derive(Debug, Subcommand)]
pub enum SampleCommand {
    /// Convert a campaign's samples into another format, for a spreadsheet or a
    /// dataframe.
    ///
    /// A pure function of `samples.ndjson`: it reads the file a campaign wrote and
    /// writes the same rows in another shape. Nothing is aggregated, nothing is
    /// dropped, nothing is measured — the human rendering is the website, which
    /// recomputes every statistic from these same samples with the harness's own
    /// code.
    Convert(ConvertArgs),
}

/// What `sample convert` can write.
///
/// An enum with one variant today rather than a `--csv` flag, and the difference is
/// not cosmetic: a boolean would have to be *mandatory* — convert to what, otherwise?
/// — which is a flag you always type and that therefore says nothing. A format is a
/// value, so it is spelled as one, and the day this grows a `parquet` the command
/// line that asks for CSV today still asks for CSV.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum SampleFormat {
    /// One row per sample, the columns `samples.ndjson` carries. Raw, never aggregated.
    #[default]
    Csv,
}

impl SampleFormat {
    /// Where this format lands when nobody says otherwise. The extension is the
    /// format's business, not the caller's.
    pub fn default_output(self) -> PathBuf {
        match self {
            Self::Csv => PathBuf::from("samples.csv"),
        }
    }
}

impl fmt::Display for SampleFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Csv => f.write_str("csv"),
        }
    }
}

/// A listing is read by a person *and* by a script. `--json` because a listing that
/// is not parsable ends up re-parsed with `awk`, and then the format is load-bearing
/// without anybody having decided it was.
#[derive(Args, Debug)]
pub struct ListArgs {
    /// Root of the tree to walk for `workload.yaml` manifests.
    #[arg(long, env = "BENCHMARKS_DIR", default_value_os_t = PathBuf::from("benchmarks"))]
    pub benchmarks_dir: PathBuf,

    /// Emit JSON rather than a table.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ImplementationListArgs {
    /// The workload whose implementations to list.
    #[arg(value_name = "WORKLOAD", env = "WORKLOAD")]
    pub workload: String,

    #[command(flatten)]
    pub list: ListArgs,
}

/// The one file a campaign writes, and the one file a conversion reads.
/// `SAMPLES_OUTPUT` names it on both sides, so a `run` and the `convert` that
/// follows agree without being told twice.
pub const DEFAULT_SAMPLES_OUTPUT: &str = "samples.ndjson";

/// Where the two schemas are written. At the repo root, because that is where an
/// editor's `yaml.schemas` mapping looks for them, and a pre-commit hook keeps them
/// honest.
///
/// One per manifest, because there are two manifests: a workload declares the work,
/// an implementation declares a backend that does it. Each resource writes its own —
/// `langbench workload jsonschema`, `langbench implementation jsonschema`.
pub const DEFAULT_BENCH_SCHEMA_OUTPUT: &str = "bench.schema.json";
pub const DEFAULT_WORKLOAD_SCHEMA_OUTPUT: &str = "workload.schema.json";

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

/// Where a schema is written. The default differs per resource, so it is not here:
/// each subcommand fills it in.
#[derive(Args, Debug)]
pub struct JsonSchemaArgs {
    /// Path of the schema to write. Missing parent directories are created.
    #[arg(long, short, env = "SCHEMA_OUTPUT")]
    pub output: Option<PathBuf>,
}

impl JsonSchemaArgs {
    /// The path this schema is written to, defaulting to the one the pre-commit
    /// hook and an editor's `yaml.schemas` both expect.
    pub fn output_or(&self, default: &str) -> PathBuf {
        self.output
            .clone()
            .unwrap_or_else(|| PathBuf::from(default))
    }
}

/// Reading a campaign back. Converting is never part of measuring: the samples
/// are the source of truth, and everything else is recomputed from them.
///
/// The samples are an *input* here, so they keep the name they were written under —
/// `SAMPLES_OUTPUT`, the value `run` wrote — and the conversion names its own
/// destination separately.
#[derive(Args, Debug)]
pub struct ConvertArgs {
    /// The `samples.ndjson` a campaign wrote.
    #[arg(
        value_name = "SAMPLES",
        env = "SAMPLES_OUTPUT",
        default_value_os_t = PathBuf::from(DEFAULT_SAMPLES_OUTPUT),
    )]
    pub samples: PathBuf,

    /// What to write.
    #[arg(long, short, env = "FORMAT", default_value_t = SampleFormat::default())]
    pub format: SampleFormat,

    /// Where to write it. Defaults to the format's own name — `samples.csv`.
    /// Missing parent directories are created.
    #[arg(long, short, env = "CONVERT_OUTPUT")]
    pub output: Option<PathBuf>,
}

impl ConvertArgs {
    /// The file this conversion writes. The default follows the *format*, never the
    /// caller: a CSV that landed in `samples.parquet` would be a file that lies about
    /// itself.
    pub fn output(&self) -> PathBuf {
        self.output
            .clone()
            .unwrap_or_else(|| self.format.default_output())
    }
}

#[derive(Args, Debug)]
pub struct RunArgs {
    /// The workload to measure, by the `id` its `workload.yaml` declares.
    ///
    /// Exactly one, and required. A campaign is one machine measuring one workload:
    /// its header snapshots that workload, its samples are all of it, and its
    /// reference checksum is the answer to *that* work. Two workloads in one file
    /// would be two experiments pretending to be one.
    #[arg(value_name = "WORKLOAD", env = "WORKLOAD")]
    pub workload: String,

    /// Override a param the workload declares: `--param grid_size=256`. Repeatable.
    ///
    /// How the work is sized belongs to the workload, not to the harness — a grid
    /// and an iteration ceiling mean nothing to a workload that parses JSON. This
    /// is the escape hatch for iterating: a full-size campaign waits on the slowest
    /// backend for an hour, and that is a poor way to find out the Dockerfile has a
    /// typo.
    ///
    /// A param the workload never declared is an error. Changing any param drops the
    /// workload's declared `checksum` — it is the answer to the declared
    /// work, not to this one — and the campaign says so, then falls back to checking
    /// that its backends agree with each other.
    #[arg(long = "param", value_name = "NAME=VALUE")]
    pub params: Vec<String>,

    /// ISA targets to build and measure: a pinned baseline, the machine itself, or
    /// both.
    ///
    /// A backend only builds the modes its manifest declares. Asking for `baseline`
    /// on a JIT gets a warning and a skipped unit, not a row — there is no way to
    /// deny a JIT the machine it is running on.
    #[arg(long, env = "MODE", value_delimiter = ',', default_values_t = Mode::ALL)]
    pub mode: Vec<Mode>,

    /// Threads handed to the kernels and to the compilers.
    ///
    /// The harness resolves a default and passes it explicitly; the kernels
    /// themselves must never auto-detect. See `site/src/content/methodology.md#the-work`.
    #[arg(long, env = "CPUS", default_value_t = default_cpu())]
    pub cpu: usize,

    /// Path of the `samples.ndjson` this campaign writes: its only artifact.
    ///
    /// Missing parent directories are created.
    #[arg(long, short, env = "SAMPLES_OUTPUT", default_value_os_t = PathBuf::from(DEFAULT_SAMPLES_OUTPUT))]
    pub output: PathBuf,

    /// Root of the tree to walk for `workload.yaml` manifests.
    #[arg(long, env = "BENCHMARKS_DIR", default_value_os_t = PathBuf::from("benchmarks"))]
    pub benchmarks_dir: PathBuf,

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

    /// The pinned ISA baseline of the `baseline` mode, passed to the compilers as
    /// `-march`. Never `native`.
    ///
    /// This flag is what `baseline` *means*, and nothing else reads it: in `native`
    /// mode the harness hands out `native` regardless of what is set here. So the
    /// two are not alternatives to choose between — one is a mode, the other is that
    /// mode's value — and spelling `native` here would be asking the baseline to
    /// stop being one.
    #[arg(long, env = "MARCH", default_value_t = default_march(), value_parser = parse_march)]
    pub march: String,

    /// Size of the tmpfs mounted on the container's build directory.
    #[arg(long, env = "TMPFS_SIZE_MB", default_value_t = 2048)]
    pub tmpfs_size_mb: u64,

    /// Memory budget of every measured container, in MiB. Swap is off.
    ///
    /// Part of the measurement, not a safety rail: a garbage-collected runtime
    /// sizes its heap from what its cgroup shows it — a JVM takes a quarter of it
    /// by default — so an unpinned budget would let the *host's* RAM decide how
    /// much memory a backend decides to want, and the peak we publish would
    /// describe the bench machine. Pinned, and identical for every backend, it is
    /// a property of the backend again.
    ///
    /// It has to clear the hungriest *compiler* in the tree, not the kernels:
    /// GraalVM's `native-image` is what sets this floor, and the build-phase
    /// tmpfs is charged to the same cgroup. Changing it changes the numbers —
    /// campaigns run under different budgets are not comparable.
    /// See `site/src/content/methodology.md#how-a-run-is-measured`.
    #[arg(long, env = "MEMORY_LIMIT_MB", default_value_t = 8192)]
    pub memory_limit_mb: u64,

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

/// A pinned architecture baseline per architecture. Empty means "pass no `-march`".
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
            "`--march native` is a contradiction: this flag is the *baseline* mode's \
             value, and a baseline that varies with the CPU that built it is not a \
             baseline. Native targeting is not forbidden here — it is a mode. Ask for \
             it with `--mode native`, which builds a second image beside this one and \
             publishes both, instead of quietly turning the pinned column into an \
             unpinned one."
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
