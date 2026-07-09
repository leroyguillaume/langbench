//! Raw samples, one NDJSON line per measured invocation.
//!
//! Aggregates are recomputed at report time; a discarded sample is gone
//! forever. See `METHODOLOGY.md#sampling`.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cli::FpMode;
use crate::machine::Machine;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    /// A timed recompile from a clean tree, artifacts discarded.
    Build,
    /// A timed execution of the binary the image ships.
    Run,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Run => "run",
        }
    }
}

/// The single JSON object a container writes on stdout.
///
/// The checksum is an integer. It is a sum of 64-bit iteration counts and the
/// correctness gate for the whole harness; anything that rounds it destroys the
/// invariant. See `METHODOLOGY.md#the-strict-mode-invariant`.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ContainerRecord {
    pub elapsed_ns: u64,
    pub user_usec: u64,
    pub system_usec: u64,
    #[serde(default)]
    pub checksum: Option<u64>,
    #[serde(default)]
    pub binary_bytes: Option<u64>,
    #[serde(default)]
    pub binary_stripped_bytes: Option<u64>,
    #[serde(default)]
    pub text_bytes: Option<u64>,
}

/// One measured invocation, as written to `samples.ndjson`.
#[derive(Clone, Debug, Serialize)]
pub struct Sample {
    pub algo: String,
    pub implementation: String,
    pub language: String,
    pub compiler: String,
    pub mode: FpMode,
    pub phase: Phase,
    pub round: u32,
    /// Warmup samples are recorded and flagged, never dropped.
    pub warmup: bool,
    pub cpu: usize,
    /// External wall-clock: container create + runtime init + compute.
    pub wall_ns: u64,
    /// Self-reported by the program: compute only.
    pub elapsed_ns: u64,
    pub user_usec: u64,
    pub system_usec: u64,
    pub checksum: Option<u64>,
    pub binary_bytes: Option<u64>,
    pub binary_stripped_bytes: Option<u64>,
    pub text_bytes: Option<u64>,
}

impl Sample {
    /// Container startup plus runtime init: the tax the JVM and CPython pay.
    /// Saturating, because the two clocks are independent and a fast run can
    /// report a few nanoseconds more than the wall-clock resolution allows.
    pub fn startup_ns(&self) -> u64 {
        self.wall_ns.saturating_sub(self.elapsed_ns)
    }

    pub fn cpu_usec(&self) -> u64 {
        self.user_usec + self.system_usec
    }
}

/// Parameters of a campaign, recorded once in the header.
#[derive(Clone, Debug, Serialize)]
pub struct Campaign {
    pub langbench_version: String,
    pub timestamp: String,
    pub cpu: usize,
    pub grid_size: u32,
    pub max_iter: u32,
    pub rounds: u32,
    pub build_rounds: u32,
    pub warmup_rounds: u32,
    pub march: String,
    pub modes: Vec<String>,
}

#[derive(Serialize)]
#[serde(tag = "record", rename_all = "lowercase")]
enum Record<'a> {
    Header {
        machine: &'a Machine,
        campaign: &'a Campaign,
    },
    Sample(&'a Sample),
}

/// Appends and **flushes** one line per sample, so an interrupted campaign
/// keeps every sample it completed.
pub struct SampleWriter {
    out: BufWriter<File>,
}

impl SampleWriter {
    pub fn create(path: &Path) -> Result<Self> {
        let file = File::create(path).with_context(|| format!("creating {}", path.display()))?;
        Ok(Self {
            out: BufWriter::new(file),
        })
    }

    pub fn write_header(&mut self, machine: &Machine, campaign: &Campaign) -> Result<()> {
        self.write(&Record::Header { machine, campaign })
    }

    pub fn write_sample(&mut self, sample: &Sample) -> Result<()> {
        self.write(&Record::Sample(sample))
    }

    fn write(&mut self, record: &Record<'_>) -> Result<()> {
        serde_json::to_writer(&mut self.out, record).context("serializing record")?;
        self.out.write_all(b"\n")?;
        self.out.flush().context("flushing samples")
    }
}

/// Parse the single JSON object a container prints on stdout.
///
/// Build tools write to stderr; stdout carries exactly one line. Anything else
/// is a broken entrypoint, and we would rather fail than measure noise.
pub fn parse_container_stdout(stdout: &str) -> Result<ContainerRecord> {
    let mut lines = stdout.lines().filter(|line| !line.trim().is_empty());
    let line = lines
        .next()
        .context("container printed nothing on stdout; expected one JSON record")?;
    anyhow::ensure!(
        lines.next().is_none(),
        "container printed more than one line on stdout; build tools must write to stderr",
    );
    serde_json::from_str(line).with_context(|| format!("parsing container record: {line}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_run_record() {
        let record = parse_container_stdout(
            r#"{"phase":"run","checksum":31415926535,"elapsed_ns":4102337891,"user_usec":32418004,"system_usec":118273}"#,
        )
        .unwrap();
        assert_eq!(record.checksum, Some(31_415_926_535));
        assert_eq!(record.elapsed_ns, 4_102_337_891);
        assert_eq!(record.binary_bytes, None);
    }

    #[test]
    fn parses_a_build_record_with_binary_sizes() {
        let record = parse_container_stdout(
            r#"{"phase":"build","elapsed_ns":812004221,"user_usec":2914000,"system_usec":204000,"binary_bytes":312840,"text_bytes":41216}"#,
        )
        .unwrap();
        assert_eq!(record.checksum, None);
        assert_eq!(record.text_bytes, Some(41_216));
    }

    #[test]
    fn a_checksum_beyond_two_to_the_fifty_three_survives_the_round_trip() {
        // 2^53 + 1 is the first integer a float64 cannot represent.
        let record = parse_container_stdout(
            r#"{"elapsed_ns":1,"user_usec":1,"system_usec":1,"checksum":9007199254740993}"#,
        )
        .unwrap();
        assert_eq!(record.checksum, Some(9_007_199_254_740_993));
    }

    #[test]
    fn rejects_extra_stdout_lines() {
        let err = parse_container_stdout(
            "compiling...\n{\"elapsed_ns\":1,\"user_usec\":1,\"system_usec\":1}",
        )
        .unwrap_err();
        assert!(err.to_string().contains("more than one line"));
    }

    #[test]
    fn rejects_empty_stdout() {
        assert!(parse_container_stdout("   \n").is_err());
    }

    #[test]
    fn startup_is_the_gap_between_the_two_clocks() {
        let sample = Sample {
            algo: "mandelbrot".to_owned(),
            implementation: "c-gcc".to_owned(),
            language: "c".to_owned(),
            compiler: "gcc".to_owned(),
            mode: FpMode::Strict,
            phase: Phase::Run,
            round: 0,
            warmup: false,
            cpu: 8,
            wall_ns: 4_300_000_000,
            elapsed_ns: 4_100_000_000,
            user_usec: 1,
            system_usec: 2,
            checksum: Some(1),
            binary_bytes: None,
            binary_stripped_bytes: None,
            text_bytes: None,
        };
        assert_eq!(sample.startup_ns(), 200_000_000);
        assert_eq!(sample.cpu_usec(), 3);
    }
}
