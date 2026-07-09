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

/// Columns of `samples.csv`, in the order `Sample` declares them.
const CSV_COLUMNS: &[&str] = &[
    "algo",
    "implementation",
    "language",
    "compiler",
    "mode",
    "phase",
    "round",
    "warmup",
    "cpu",
    "wall_ns",
    "elapsed_ns",
    "user_usec",
    "system_usec",
    "checksum",
    "binary_bytes",
    "binary_stripped_bytes",
    "text_bytes",
];

impl Sample {
    /// A flat view of the same record `samples.ndjson` carries.
    ///
    /// Missing values are **empty**, never `n/a`: a numeric column that
    /// sometimes holds a word breaks every parser that reads it.
    fn csv_row(&self) -> String {
        let columns = [
            escape(&self.algo),
            escape(&self.implementation),
            escape(&self.language),
            escape(&self.compiler),
            self.mode.to_string(),
            self.phase.as_str().to_owned(),
            self.round.to_string(),
            self.warmup.to_string(),
            self.cpu.to_string(),
            self.wall_ns.to_string(),
            self.elapsed_ns.to_string(),
            self.user_usec.to_string(),
            self.system_usec.to_string(),
            optional(self.checksum),
            optional(self.binary_bytes),
            optional(self.binary_stripped_bytes),
            optional(self.text_bytes),
        ];
        columns.join(",")
    }
}

fn optional(value: Option<u64>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn escape(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

/// Appends and **flushes** one line per sample, so an interrupted campaign keeps
/// every sample it completed.
///
/// Two renderings of the same records: `samples.ndjson` is the source of truth
/// and carries the machine and campaign header; `samples.csv` is the flat view a
/// spreadsheet or a dataframe can read directly. Both are written in lockstep.
pub struct SampleWriter {
    ndjson: BufWriter<File>,
    csv: BufWriter<File>,
}

impl SampleWriter {
    pub fn create(dir: &Path) -> Result<Self> {
        Ok(Self {
            ndjson: create(&dir.join("samples.ndjson"))?,
            csv: create(&dir.join("samples.csv"))?,
        })
    }

    /// The header lands in the NDJSON only: a CSV has no room for it. The
    /// campaign's context — machine, grid size, `-march` — lives there.
    pub fn write_header(&mut self, machine: &Machine, campaign: &Campaign) -> Result<()> {
        self.write_ndjson(&Record::Header { machine, campaign })?;
        writeln!(self.csv, "{}", CSV_COLUMNS.join(",")).context("writing the CSV header")?;
        self.csv.flush().context("flushing samples.csv")
    }

    pub fn write_sample(&mut self, sample: &Sample) -> Result<()> {
        self.write_ndjson(&Record::Sample(sample))?;
        writeln!(self.csv, "{}", sample.csv_row()).context("writing a CSV row")?;
        self.csv.flush().context("flushing samples.csv")
    }

    fn write_ndjson(&mut self, record: &Record<'_>) -> Result<()> {
        serde_json::to_writer(&mut self.ndjson, record).context("serializing record")?;
        self.ndjson.write_all(b"\n")?;
        self.ndjson.flush().context("flushing samples.ndjson")
    }
}

fn create(path: &Path) -> Result<BufWriter<File>> {
    let file = File::create(path).with_context(|| format!("creating {}", path.display()))?;
    Ok(BufWriter::new(file))
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

    fn sample(phase: Phase, checksum: Option<u64>) -> Sample {
        Sample {
            algo: "mandelbrot".to_owned(),
            implementation: "c-gcc".to_owned(),
            language: "c".to_owned(),
            compiler: "gcc".to_owned(),
            mode: FpMode::Strict,
            phase,
            round: 3,
            warmup: false,
            cpu: 8,
            wall_ns: 313_600_000,
            elapsed_ns: 213_300_000,
            user_usec: 860_000,
            system_usec: 4_000,
            checksum,
            binary_bytes: Some(70_984),
            binary_stripped_bytes: Some(67_568),
            text_bytes: Some(1_340),
        }
    }

    #[test]
    fn a_csv_row_has_exactly_one_field_per_declared_column() {
        let row = sample(Phase::Run, Some(448_356_792)).csv_row();
        assert_eq!(row.split(',').count(), CSV_COLUMNS.len());
    }

    #[test]
    fn a_csv_row_carries_raw_values_in_column_order() {
        let row = sample(Phase::Run, Some(448_356_792)).csv_row();
        assert_eq!(
            row,
            "mandelbrot,c-gcc,c,gcc,strict,run,3,false,8,313600000,213300000,860000,4000,\
             448356792,70984,67568,1340",
        );
    }

    #[test]
    fn a_missing_value_is_an_empty_field_never_the_word_not_available() {
        // A build record has no checksum. `n/a` in a numeric column breaks every
        // parser that reads it.
        let row = sample(Phase::Build, None).csv_row();
        assert!(row.contains(",,"), "empty checksum field: {row}");
        assert!(!row.contains("n/a"));
        assert_eq!(row.split(',').count(), CSV_COLUMNS.len());
    }

    #[test]
    fn a_field_containing_a_separator_is_quoted() {
        let mut awkward = sample(Phase::Run, Some(1));
        awkward.implementation = "c-gcc,-O2".to_owned();
        assert!(awkward.csv_row().contains("\"c-gcc,-O2\""));
    }

    #[test]
    fn a_field_containing_a_quote_doubles_it() {
        assert_eq!(escape("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(escape("plain"), "plain");
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
