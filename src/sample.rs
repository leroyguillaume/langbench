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

use crate::machine::Machine;
use crate::mode::FpMode;

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
    /// The cgroup's memory high-water mark, read by the container from its own
    /// `memory.peak`. `None` on a kernel that exposes neither that nor the cgroup
    /// v1 file — an absence, never a zero.
    ///
    /// It is the whole container: the process tree, the page cache it faulted in,
    /// the tmpfs it wrote. Not the RSS of one process, and deliberately not — the
    /// question is what this backend needed in order to run.
    #[serde(default)]
    pub peak_bytes: Option<u64>,
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
    pub workload: String,
    /// The implementation is its (language, compiler, interpreter) triple —
    /// there is no separate name, and no directory path, to stand in for it.
    ///
    /// All of it is copied from `bench.yaml` onto every line. That is repetition,
    /// and it is the point: a sample must say what produced it without a second
    /// file to join against. The manifest can be edited, the directory renamed,
    /// the backend deleted — these lines still describe the campaign that ran.
    pub language: String,
    /// `None` for a backend that compiles nothing ahead of the run.
    pub compiler: Option<String>,
    /// `None` for a backend that ships machine code and no runtime.
    pub interpreter: Option<String>,
    pub description: String,
    pub comments: Option<String>,
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
    /// The container's peak memory, from its own cgroup. See
    /// [`ContainerRecord::peak_bytes`].
    #[serde(default)]
    pub peak_bytes: Option<u64>,
    /// Bytes of the one source file the manifest declares. A property of the
    /// implementation, read off disk at discovery — not of the run, and not of the
    /// image.
    ///
    /// It measures the *language*, and it is honest about that: two backends that
    /// compile the same file report the same number, which is exactly what a
    /// `c-gcc` / `c-clang` pair should say about the code somebody wrote once.
    ///
    /// `Option` for one reason only: a campaign recorded before this field existed
    /// carries none, and a report of it says so rather than inventing a zero.
    #[serde(default)]
    pub source_bytes: Option<u64>,
    pub checksum: Option<u64>,
    pub binary_bytes: Option<u64>,
    pub binary_stripped_bytes: Option<u64>,
    pub text_bytes: Option<u64>,
}

/// The identity of a backend, flattened into one token: `c-gcc`,
/// `python-cpython`, `python-cython-cpython`.
///
/// Derived from the triple and never declared, so it cannot drift from it. It is
/// what tags an image and what names a row; two backends collide here exactly
/// when they *are* the same backend.
pub fn backend_slug(language: &str, compiler: Option<&str>, interpreter: Option<&str>) -> String {
    [Some(language), compiler, interpreter]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("-")
}

impl Sample {
    /// This sample's backend, as one token. See `backend_slug`.
    pub fn backend(&self) -> String {
        backend_slug(
            &self.language,
            self.compiler.as_deref(),
            self.interpreter.as_deref(),
        )
    }

    /// Container startup plus runtime init: the tax the JVM and CPython pay.
    /// Saturating, because the two clocks are independent and a fast run can
    /// report a few nanoseconds more than the wall-clock resolution allows.
    pub fn startup_ns(&self) -> u64 {
        self.wall_ns.saturating_sub(self.elapsed_ns)
    }

    pub fn cpu_usec(&self) -> u64 {
        self.user_usec + self.system_usec
    }

    /// How many cores this run actually kept busy, in thousandths of a core.
    ///
    /// CPU time over compute time — and the single number that separates *this
    /// backend is slow* from *this backend cannot use the machine*. Two rows with
    /// the same wall-clock and 1.0 versus 7.8 cores are not two slow backends;
    /// one of them is a GIL. The harness hands every kernel the same thread count
    /// ([`Self::cpu`]), so this is read against that.
    ///
    /// **It can exceed the thread count, and that is a result, not an overflow.**
    /// The numerator is every microsecond of CPU the container burned; the
    /// denominator is only the span the program timed itself over. A JIT compiling
    /// on one thread while the kernel computes on eight is spending CPU that the
    /// hot loop's own clock never sees — and a reader comparing a JVM to a static
    /// binary deserves to see that CPU rather than have it quietly normalised away.
    ///
    /// Per sample, never as a ratio of two minima: those come from different
    /// rounds and describe a run that never happened. The same rule as
    /// [`Self::startup_ns`].
    ///
    /// `None` when the program reported no elapsed time — there is nothing to
    /// divide by, and a zero denominator is not infinite parallelism.
    pub fn cores_milli(&self) -> Option<u64> {
        if self.elapsed_ns == 0 {
            return None;
        }
        // `u128`, because the numerator scales a microsecond count by a billion:
        // a minute of CPU time would already overflow a `u64` here, and it is a
        // perfectly ordinary campaign that spends one.
        let cpu_ns = u128::from(self.cpu_usec()) * 1_000;
        let milli = cpu_ns * 1_000 / u128::from(self.elapsed_ns);
        Some(u64::try_from(milli).unwrap_or(u64::MAX))
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

/// Where a backend died. A backend that never built and a backend that built and
/// then crashed are two different bugs, and the reader has to know which.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    /// `docker build` failed: the image does not exist.
    Prepare,
    /// The container ran and did not produce a valid record: a crash, a hang past
    /// the timeout, unreadable stdout, or a checksum that diverges from strict.
    Measure,
}

impl Stage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prepare => "prepare",
            Self::Measure => "measure",
        }
    }
}

/// A backend that left the campaign, and what it left on.
///
/// A failure is **not** a sample: it carries no timing, and nothing aggregates
/// it. It is written to the same file for the same reason the samples are — the
/// renderings are pure functions of `samples.ndjson`, so a report that names the
/// backends that broke can only get that fact from here. A row that is missing
/// from a table looks exactly like a backend nobody wrote, and that is the one
/// thing a benchmark must never let a reader believe.
///
/// Like a sample, it copies its manifest fields onto itself: it must say what
/// broke without a second file to join against.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Failure {
    pub workload: String,
    pub language: String,
    pub compiler: Option<String>,
    pub interpreter: Option<String>,
    pub description: String,
    pub comments: Option<String>,
    pub mode: FpMode,
    pub stage: Stage,
    pub phase: Option<Phase>,
    /// The round it died in. `None` when the image never built — there was no
    /// round yet.
    pub round: Option<u32>,
    /// The error, rendered with its full `anyhow` context chain. A string, and
    /// deliberately so: it is a message for a human, and the harness never
    /// branches on it.
    pub error: String,
}

impl Failure {
    /// This failure's backend, as one token. See `backend_slug`.
    pub fn backend(&self) -> String {
        backend_slug(
            &self.language,
            self.compiler.as_deref(),
            self.interpreter.as_deref(),
        )
    }
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
    Failure(&'a Failure),
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
    Failure(Failure),
}

/// Everything one campaign recorded: its context, every measured invocation, and
/// every backend it lost on the way.
///
/// This is what `langbench csv` and `langbench md` consume. Both are pure
/// functions of this value, which is why the campaign never renders anything
/// itself.
#[derive(Debug)]
pub struct Recording {
    pub machine: Machine,
    pub campaign: Campaign,
    pub samples: Vec<Sample>,
    /// Empty on a campaign where everything worked, which is not the common case
    /// the moment a new backend lands.
    pub failures: Vec<Failure>,
}

/// Read back a `samples.ndjson` written by a campaign.
///
/// A campaign killed mid-round leaves a truncated last line; that is a fact
/// about the run, not a reason to lose the samples that precede it, so the
/// final line is dropped with a warning rather than failing the command.
pub fn load(path: &Path) -> Result<Recording> {
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    parse(&raw).with_context(|| format!("reading {}", path.display()))
}

/// The same campaign, from bytes rather than from a path.
///
/// The website has no filesystem: it fetches `samples.ndjson` over HTTP and
/// parses it *here*, in Rust, precisely so that a `JSON.parse` in JavaScript
/// never gets to round a 64-bit checksum to the nearest double. See
/// `crate::wasm`.
pub fn parse(raw: &str) -> Result<Recording> {
    let mut lines = raw
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty());

    let (_, first) = lines.next().context("the campaign is empty")?;
    let OwnedRecord::Header { machine, campaign } = parse_record(first, 1)? else {
        anyhow::bail!(
            "the campaign does not start with a header record; it was not written by \
             `langbench run`",
        );
    };

    let mut samples = Vec::new();
    let mut failures = Vec::new();
    let mut pending: Option<(usize, &str)> = None;
    for (index, line) in lines {
        // Only the *last* line can be a torn write: hold each one back until the
        // next arrives, so a truncation in the middle is still an error.
        if let Some((index, line)) = pending.take() {
            match parse_record(line, index + 1)? {
                OwnedRecord::Sample(sample) => samples.push(sample),
                OwnedRecord::Failure(failure) => failures.push(failure),
                OwnedRecord::Header { .. } => anyhow::bail!(
                    "line {}: a second header record; two campaigns were appended to one file",
                    index + 1,
                ),
            }
        }
        pending = Some((index, line));
    }
    if let Some((index, line)) = pending {
        match parse_record(line, index + 1) {
            Ok(OwnedRecord::Sample(sample)) => samples.push(sample),
            Ok(OwnedRecord::Failure(failure)) => failures.push(failure),
            Ok(OwnedRecord::Header { .. }) => anyhow::bail!(
                "line {}: a second header record; two campaigns were appended to one file",
                index + 1,
            ),
            Err(error) => tracing::warn!(
                "the last line is truncated and was dropped ({error:#}). The campaign was most \
                 likely interrupted mid-write; every earlier sample is intact.",
            ),
        }
    }

    Ok(Recording {
        machine: *machine,
        campaign,
        samples,
        failures,
    })
}

fn parse_record(line: &str, number: usize) -> Result<OwnedRecord> {
    serde_json::from_str(line).with_context(|| format!("line {number}"))
}

/// Columns of the CSV rendering, in the order `Sample` declares them.
const CSV_COLUMNS: &[&str] = &[
    "workload",
    "language",
    "compiler",
    "interpreter",
    "description",
    "comments",
    "mode",
    "phase",
    "round",
    "warmup",
    "cpu",
    "wall_ns",
    "elapsed_ns",
    "user_usec",
    "system_usec",
    "peak_bytes",
    "source_bytes",
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
            escape(&self.workload),
            escape(&self.language),
            escape(self.compiler.as_deref().unwrap_or_default()),
            escape(self.interpreter.as_deref().unwrap_or_default()),
            escape(&self.description),
            escape(self.comments.as_deref().unwrap_or_default()),
            self.mode.to_string(),
            self.phase.as_str().to_owned(),
            self.round.to_string(),
            self.warmup.to_string(),
            self.cpu.to_string(),
            self.wall_ns.to_string(),
            self.elapsed_ns.to_string(),
            self.user_usec.to_string(),
            self.system_usec.to_string(),
            optional(self.peak_bytes),
            optional(self.source_bytes),
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

    /// A backend that left the campaign. Written where it happened, flushed like
    /// a sample: an interrupted campaign keeps the record of what had already
    /// broken, and the renderings can name it.
    pub fn write_failure(&mut self, failure: &Failure) -> Result<()> {
        self.write_record(&Record::Failure(failure))
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
            workload: "mandelbrot".to_owned(),
            language: "c".to_owned(),
            compiler: Some("gcc".to_owned()),
            interpreter: None,
            description: "The reference C kernel.".to_owned(),
            comments: None,
            mode: FpMode::Strict,
            phase,
            round: 3,
            warmup: false,
            cpu: 8,
            wall_ns: 313_600_000,
            elapsed_ns: 213_300_000,
            user_usec: 860_000,
            system_usec: 4_000,
            peak_bytes: Some(12_582_912),
            source_bytes: Some(2_048),
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
        file.write_all(br#"{"record":"sample","workload":"mandelb"#)
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
        file.write_all(b"{\"record\":\"sample\",\"workload\":\"mandelb\n")
            .unwrap();
        file.write_all(b"{\"record\":\"sample\",\"workload\":\"mandelb\n")
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

        // `{:#}` walks the chain: `load` names the file, `parse` says what is
        // wrong with it.
        let error = format!("{:#}", load(&path).unwrap_err());
        assert!(error.contains("header"), "{error}");
        assert!(error.contains("samples.ndjson"), "{error}");
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
            "mandelbrot,c,gcc,,The reference C kernel.,,strict,run,3,false,8,313600000,\
             213300000,860000,4000,12582912,2048,448356792,70984,67568,1340",
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

    /// Not hypothetical: `description` and `comments` are prose lifted out of a
    /// `bench.yaml`, and prose has commas in it.
    #[test]
    fn a_field_containing_a_separator_is_quoted() {
        let mut awkward = sample(Phase::Run, Some(1));
        awkward.description = "Fast, in theory.".to_owned();
        assert!(awkward.csv_row().contains("\"Fast, in theory.\""));
    }

    #[test]
    fn a_field_containing_a_quote_doubles_it() {
        assert_eq!(escape("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(escape("plain"), "plain");
    }

    #[test]
    fn startup_is_the_gap_between_the_two_clocks() {
        let mut measured = sample(Phase::Run, Some(1));
        measured.wall_ns = 4_300_000_000;
        measured.elapsed_ns = 4_100_000_000;
        measured.user_usec = 1;
        measured.system_usec = 2;

        assert_eq!(measured.startup_ns(), 200_000_000);
        assert_eq!(measured.cpu_usec(), 3);
    }

    /// Eight cores, saturated for the whole compute span: 8000 thousandths.
    #[test]
    fn cores_are_the_cpu_time_over_the_compute_time() {
        let mut measured = sample(Phase::Run, Some(1));
        measured.cpu = 8;
        measured.elapsed_ns = 1_000_000_000; // 1 s of compute
        measured.user_usec = 8_000_000; // 8 s of CPU
        measured.system_usec = 0;
        assert_eq!(measured.cores_milli(), Some(8_000));
    }

    /// The GIL, in one number: one core busy however many threads it was handed.
    #[test]
    fn a_backend_that_cannot_use_the_machine_says_so_in_one_core() {
        let mut measured = sample(Phase::Run, Some(1));
        measured.cpu = 8;
        measured.elapsed_ns = 10_000_000_000; // 10 s of compute
        measured.user_usec = 10_000_000; // 10 s of CPU: one core
        measured.system_usec = 0;
        assert_eq!(measured.cores_milli(), Some(1_000));
    }

    /// Not an overflow: the CPU clock counts the runtime's JIT and GC threads,
    /// and the compute clock does not count the span they ran in. A JVM burning
    /// more CPU than its hot loop's wall-clock explains is the result.
    #[test]
    fn cores_may_exceed_the_thread_count_and_that_is_a_result() {
        let mut measured = sample(Phase::Run, Some(1));
        measured.cpu = 4;
        measured.elapsed_ns = 1_000_000_000;
        measured.user_usec = 5_500_000; // 5.5 core-seconds in a 1 s window
        measured.system_usec = 0;
        assert_eq!(measured.cores_milli(), Some(5_500));
    }

    /// A zero denominator is not infinite parallelism.
    #[test]
    fn a_run_that_reported_no_compute_time_has_no_core_count() {
        let mut measured = sample(Phase::Run, Some(1));
        measured.elapsed_ns = 0;
        assert_eq!(measured.cores_milli(), None);
    }

    /// A campaign long enough to spend a minute of CPU overflows the naive `u64`
    /// arithmetic — and a perfectly ordinary campaign spends one.
    #[test]
    fn a_long_run_does_not_overflow_the_core_count() {
        let mut measured = sample(Phase::Run, Some(1));
        measured.elapsed_ns = 600_000_000_000; // 10 minutes of compute
        measured.user_usec = 4_800_000_000; // 80 core-minutes
        measured.system_usec = 0;
        assert_eq!(measured.cores_milli(), Some(8_000));
    }

    #[test]
    fn a_container_reports_its_peak_memory() {
        let record = parse_container_stdout(
            r#"{"phase":"run","checksum":7,"elapsed_ns":1,"user_usec":1,"system_usec":1,"peak_bytes":12582912}"#,
        )
        .unwrap();
        assert_eq!(record.peak_bytes, Some(12_582_912));
    }

    /// A kernel that exposes no `memory.peak` reports an absence, and the absence
    /// travels. A zero would read as a backend that needed no memory at all.
    #[test]
    fn a_kernel_with_no_cgroup_peak_reports_nothing_rather_than_zero() {
        let record = parse_container_stdout(
            r#"{"phase":"run","checksum":7,"elapsed_ns":1,"user_usec":1,"system_usec":1,"peak_bytes":null}"#,
        )
        .unwrap();
        assert_eq!(record.peak_bytes, None);
    }

    /// The two fields a campaign recorded before they existed. An old
    /// `samples.ndjson` still renders, and its new columns say `n/a` — which is
    /// the truth about it.
    #[test]
    fn a_campaign_recorded_before_these_metrics_existed_still_loads() {
        let mut old = sample(Phase::Run, Some(1));
        old.peak_bytes = None;
        old.source_bytes = None;
        let mut line = serde_json::to_value(&old).unwrap();
        let object = line.as_object_mut().unwrap();
        object.remove("peak_bytes");
        object.remove("source_bytes");

        let reloaded: Sample = serde_json::from_value(line).unwrap();
        assert_eq!(reloaded.peak_bytes, None);
        assert_eq!(reloaded.source_bytes, None);
    }

    /// A campaign written while the harness still recorded energy carries an
    /// `energy_uj` the struct no longer has. Serde ignores it, and the campaign
    /// loads: a field that left the harness must not take the samples with it.
    #[test]
    fn a_campaign_recorded_when_energy_existed_still_loads() {
        let mut line = serde_json::to_value(sample(Phase::Run, Some(1))).unwrap();
        let object = line.as_object_mut().unwrap();
        object.insert("energy_uj".to_owned(), serde_json::json!(9_400_000u64));

        let reloaded: Sample = serde_json::from_value(line).unwrap();
        assert_eq!(reloaded.checksum, Some(1));
        assert_eq!(reloaded.peak_bytes, Some(12_582_912));
    }
}
