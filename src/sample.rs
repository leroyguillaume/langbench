//! Raw samples, one NDJSON line per measured invocation.
//!
//! `samples.ndjson` is the **only** thing a campaign writes, and it is the only
//! thing that cannot be recomputed. Every other artifact — the CSV, the report —
//! is a rendering of these lines, produced after the fact by `langbench csv` and
//! `langbench md`. Aggregates never replace the samples; a discarded sample is
//! gone forever. See `METHODOLOGY.md#sampling`.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::cli::FpMode;
use crate::machine::Machine;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Serialize)]
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

/// A line of `samples.ndjson`, as written.
#[derive(Serialize)]
#[serde(tag = "record", rename_all = "lowercase")]
enum Record<'a> {
    Header {
        machine: &'a Machine,
        campaign: &'a Campaign,
    },
    Sample(&'a Sample),
}

/// The same line, as read back. Owned, because nothing borrows the file.
///
/// The machine is boxed: it dwarfs a sample, and every line but the first is a
/// sample.
#[derive(Deserialize)]
#[serde(tag = "record", rename_all = "lowercase")]
enum OwnedRecord {
    Header {
        machine: Box<Machine>,
        campaign: Campaign,
    },
    Sample(Sample),
}

/// Everything one campaign recorded: its context, and every measured invocation.
///
/// This is what `langbench csv` and `langbench md` consume. Both are pure
/// functions of this value, which is why the campaign never renders anything
/// itself.
#[derive(Debug)]
pub struct Recording {
    pub machine: Machine,
    pub campaign: Campaign,
    pub samples: Vec<Sample>,
}

/// Read back a `samples.ndjson` written by a campaign.
///
/// A campaign killed mid-round leaves a truncated last line; that is a fact
/// about the run, not a reason to lose the samples that precede it, so the
/// final line is dropped with a warning rather than failing the command.
pub fn load(path: &Path) -> Result<Recording> {
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut lines = raw
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty());

    let (_, first) = lines
        .next()
        .with_context(|| format!("{} is empty", path.display()))?;
    let OwnedRecord::Header { machine, campaign } = parse_record(first, 1, path)? else {
        anyhow::bail!(
            "{} does not start with a header record; it was not written by `langbench run`",
            path.display(),
        );
    };

    let mut samples = Vec::new();
    let mut pending: Option<(usize, &str)> = None;
    for (index, line) in lines {
        // Only the *last* line can be a torn write: hold each one back until the
        // next arrives, so a truncation in the middle is still an error.
        if let Some((index, line)) = pending.take() {
            match parse_record(line, index + 1, path)? {
                OwnedRecord::Sample(sample) => samples.push(sample),
                OwnedRecord::Header { .. } => anyhow::bail!(
                    "{}:{}: a second header record; two campaigns were appended to one file",
                    path.display(),
                    index + 1,
                ),
            }
        }
        pending = Some((index, line));
    }
    if let Some((index, line)) = pending {
        match parse_record(line, index + 1, path) {
            Ok(OwnedRecord::Sample(sample)) => samples.push(sample),
            Ok(OwnedRecord::Header { .. }) => anyhow::bail!(
                "{}:{}: a second header record; two campaigns were appended to one file",
                path.display(),
                index + 1,
            ),
            Err(error) => tracing::warn!(
                "{}: the last line is truncated and was dropped ({error:#}). The campaign was \
                 most likely interrupted mid-write; every earlier sample is intact.",
                path.display(),
            ),
        }
    }

    Ok(Recording {
        machine: *machine,
        campaign,
        samples,
    })
}

fn parse_record(line: &str, number: usize, path: &Path) -> Result<OwnedRecord> {
    serde_json::from_str(line).with_context(|| format!("{}:{number}", path.display()))
}

/// Columns of the CSV rendering, in the order `Sample` declares them.
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

/// The flat view of the samples a spreadsheet or a dataframe can read.
///
/// The header record has no room here — a CSV has one shape, and the machine and
/// campaign context does not fit it. That context stays in the NDJSON, which is
/// why this is a rendering and not the source of truth.
pub fn to_csv(samples: &[Sample]) -> String {
    let mut out = String::new();
    out.push_str(&CSV_COLUMNS.join(","));
    out.push('\n');
    for sample in samples {
        out.push_str(&sample.csv_row());
        out.push('\n');
    }
    out
}

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
/// The campaign writes this and nothing else. Rendering — CSV, Markdown — happens
/// afterwards, from this file, so a report can never be produced from anything a
/// run did not actually record.
pub struct SampleWriter {
    ndjson: BufWriter<File>,
}

impl SampleWriter {
    pub fn create(path: &Path) -> Result<Self> {
        let file = File::create(path).with_context(|| format!("creating {}", path.display()))?;
        Ok(Self {
            ndjson: BufWriter::new(file),
        })
    }

    /// The campaign's context — machine, grid size, `-march` — recorded once, so
    /// the file explains itself without the command line that produced it.
    pub fn write_header(&mut self, machine: &Machine, campaign: &Campaign) -> Result<()> {
        self.write_record(&Record::Header { machine, campaign })
    }

    pub fn write_sample(&mut self, sample: &Sample) -> Result<()> {
        self.write_record(&Record::Sample(sample))
    }

    fn write_record(&mut self, record: &Record<'_>) -> Result<()> {
        serde_json::to_writer(&mut self.ndjson, record).context("serializing record")?;
        self.ndjson.write_all(b"\n")?;
        self.ndjson.flush().context("flushing samples.ndjson")
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
    use tempfile::TempDir;

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

    /// Write a campaign the way `run` does, then read it back the way `csv` and
    /// `md` do. The two commands only exist because this round-trip holds.
    fn round_trip(campaign: &Campaign, samples: &[Sample]) -> Result<Recording> {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("samples.ndjson");
        let mut writer = SampleWriter::create(&path).unwrap();
        writer.write_header(&Machine::default(), campaign).unwrap();
        for sample in samples {
            writer.write_sample(sample).unwrap();
        }
        drop(writer);
        load(&path)
    }

    fn campaign() -> Campaign {
        Campaign {
            langbench_version: "0.1.0".to_owned(),
            timestamp: "2026-07-11T12:00:00Z".to_owned(),
            cpu: 8,
            grid_size: 2048,
            max_iter: 1000,
            rounds: 10,
            build_rounds: 3,
            warmup_rounds: 1,
            march: "x86-64-v3".to_owned(),
            modes: vec!["strict".to_owned()],
        }
    }

    #[test]
    fn a_campaign_survives_the_round_trip_through_the_file() {
        let samples = [
            sample(Phase::Run, Some(448_356_792)),
            sample(Phase::Build, None),
        ];
        let recording = round_trip(&campaign(), &samples).unwrap();

        assert_eq!(recording.campaign.grid_size, 2048);
        assert_eq!(recording.campaign.march, "x86-64-v3");
        assert_eq!(recording.samples.len(), 2);
        assert_eq!(recording.samples[0].mode, FpMode::Strict);
        assert_eq!(recording.samples[0].phase, Phase::Run);
        assert_eq!(recording.samples[0].checksum, Some(448_356_792));
        assert_eq!(recording.samples[1].checksum, None);
    }

    #[test]
    fn a_checksum_beyond_two_to_the_fifty_three_survives_the_file_too() {
        // The same 2^53 trap as on the container's stdout: a `f64` on either side
        // of the file would silently round the correctness gate.
        let mut huge = sample(Phase::Run, Some(9_007_199_254_740_993));
        huge.wall_ns = 9_007_199_254_740_993;
        let recording = round_trip(&campaign(), &[huge]).unwrap();
        assert_eq!(recording.samples[0].checksum, Some(9_007_199_254_740_993));
        assert_eq!(recording.samples[0].wall_ns, 9_007_199_254_740_993);
    }

    #[test]
    fn an_interrupted_campaign_keeps_every_sample_before_the_torn_line() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("samples.ndjson");
        let mut writer = SampleWriter::create(&path).unwrap();
        writer
            .write_header(&Machine::default(), &campaign())
            .unwrap();
        writer.write_sample(&sample(Phase::Run, Some(7))).unwrap();
        drop(writer);

        // A process killed mid-`write` leaves exactly this.
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        file.write_all(br#"{"record":"sample","algo":"mandelb"#)
            .unwrap();
        drop(file);

        let recording = load(&path).unwrap();
        assert_eq!(recording.samples.len(), 1);
        assert_eq!(recording.samples[0].checksum, Some(7));
    }

    #[test]
    fn a_truncation_that_is_not_the_last_line_is_still_an_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("samples.ndjson");
        let mut writer = SampleWriter::create(&path).unwrap();
        writer
            .write_header(&Machine::default(), &campaign())
            .unwrap();
        drop(writer);
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        file.write_all(b"{\"record\":\"sample\",\"algo\":\"mandelb\n")
            .unwrap();
        file.write_all(b"{\"record\":\"sample\",\"algo\":\"mandelb\n")
            .unwrap();
        drop(file);

        assert!(load(&path).is_err());
    }

    #[test]
    fn a_file_that_does_not_open_on_a_header_is_refused_rather_than_half_read() {
        // Well-formed samples, but no header: the campaign's context is missing,
        // and a report without a machine is a report about nothing.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("samples.ndjson");
        let headless =
            serde_json::to_string(&Record::Sample(&sample(Phase::Run, Some(1)))).unwrap();
        std::fs::write(&path, format!("{headless}\n")).unwrap();

        let error = load(&path).unwrap_err();
        assert!(error.to_string().contains("header"), "{error}");
    }

    #[test]
    fn the_csv_rendering_starts_with_the_column_header() {
        let csv = to_csv(&[sample(Phase::Run, Some(1))]);
        let mut lines = csv.lines();
        assert_eq!(lines.next().unwrap(), CSV_COLUMNS.join(","));
        assert_eq!(lines.next().unwrap().split(',').count(), CSV_COLUMNS.len());
        assert_eq!(lines.next(), None);
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
