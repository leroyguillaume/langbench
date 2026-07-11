use std::fmt;
use std::path::PathBuf;
use std::thread::available_parallelism;

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

/// Floating-point semantics the kernels are compiled under.
///
/// The axis is FP semantics, not "optimization on/off": every mode is `-O3`.
/// See `METHODOLOGY.md#floating-point-modes`.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum FpMode {
    /// `-ffp-contract=off`, no fast-math. Bit-reproducible IEEE 754.
    Strict,
    /// FMA contraction allowed: bit-different, but more accurate.
    Fma,
    /// `-ffast-math`: reassociation allowed, precision sold for speed.
    Fast,
}

impl FpMode {
    pub const ALL: [Self; 3] = [Self::Strict, Self::Fma, Self::Fast];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::Fma => "fma",
            Self::Fast => "fast",
        }
    }

    /// Parse a mode as it is spelled in a `langbench.fp_modes` label.
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.as_str() == value)
    }
}

impl fmt::Display for FpMode {
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
    Csv(RenderArgs),

    /// Render a campaign's samples as a Markdown report.
    Md(MarkdownArgs),

    /// Describe this machine, and why it may be a poor benchmark target.
    ///
    /// Prints exactly what a campaign would record in its header, so you can
    /// check a host before spending an hour measuring on it.
    Machine,
}

/// The one file a campaign writes, and the one file a rendering reads. `OUTPUT`
/// names it on both sides, so a `run` and the `md` that follows agree without
/// being told twice.
pub const DEFAULT_OUTPUT: &str = "samples.ndjson";

/// Reading a campaign back. Rendering is never part of measuring: the samples
/// are the source of truth, and every artifact is recomputed from them.
///
/// Renderings go to stdout. Redirect them — `langbench md > report.md` — rather
/// than growing a second, opposite meaning for `--output`.
#[derive(Args, Debug)]
pub struct RenderArgs {
    /// The `samples.ndjson` a campaign wrote.
    #[arg(
        value_name = "SAMPLES",
        env = "OUTPUT",
        default_value_os_t = PathBuf::from(DEFAULT_OUTPUT),
    )]
    pub samples: PathBuf,
}

#[derive(Args, Debug)]
pub struct MarkdownArgs {
    #[command(flatten)]
    pub render: RenderArgs,

    /// A Liquid template to render instead of the built-in one.
    ///
    /// It receives exactly the same variables; `langbench md --print-template`
    /// writes the built-in one out as a starting point.
    #[arg(long, env = "TEMPLATE", conflicts_with = "print_template")]
    pub template: Option<PathBuf>,

    /// Print the built-in template on stdout and exit, measuring nothing.
    #[arg(long)]
    pub print_template: bool,
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
    #[arg(long, short, env = "OUTPUT", default_value_os_t = PathBuf::from(DEFAULT_OUTPUT))]
    pub output: PathBuf,

    /// Root of the `<algo>/<language>-<compiler>/Dockerfile` tree.
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

    #[test]
    fn fp_mode_display_matches_serialization() {
        for mode in FpMode::ALL {
            let json = serde_json::to_string(&mode).unwrap();
            assert_eq!(json, format!("\"{mode}\""));
        }
    }
}
