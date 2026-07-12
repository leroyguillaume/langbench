//! Human-facing Markdown report, rendered from `templates/report.md.liquid`.
//!
//! Formatting, and nothing else. The arithmetic lives in [`crate::analysis`],
//! which the WebAssembly build calls too — so the table in `report.md` and the
//! table on the website are two renderings of one computation rather than two
//! computations that happen to agree today.
//!
//! Everything here is derived from the raw samples and can be recomputed from
//! `samples.ndjson`. Aggregates never replace the samples.

use anyhow::{Context, Result};
use serde::Serialize;

use crate::analysis::{self, Aggregate, Analysis, Options, analyze};
use crate::machine::Field;
use crate::sample::{Campaign, Recording, Stage};
use crate::stats::Summary;

/// Embedded, so the binary renders a report without a working directory. A
/// campaign can override it with `--template`.
pub const DEFAULT_TEMPLATE: &str = include_str!("../templates/report.md.liquid");

#[derive(Debug, Serialize)]
pub struct ReportData {
    pub campaign: Campaign,
    pub machine_fields: Vec<Field>,
    pub warnings: Vec<String>,
    pub algos: Vec<AlgoReport>,
    /// Every backend the campaign measured, described once, at the end of the
    /// report. The tables repeat a backend per FP mode; a description repeated
    /// three times is a description nobody reads, and one sitting between the
    /// numbers and the reader is a description in the way. Straight from the
    /// `bench.yaml` the samples carry.
    pub backends: Vec<Backend>,
    /// Every backend the campaign lost, and to what. Empty on a clean campaign,
    /// and Liquid tests it for size — the section does not exist when there is
    /// nothing to confess.
    pub failures: Vec<FailureRow>,
}

/// One quarantined `(backend, mode)`, formatted.
///
/// A failed backend has no row in the tables above, and a row that is absent
/// reads exactly like a backend nobody ever wrote. This is where the report says
/// which ones it lost — a benchmark that quietly omits what did not work is a
/// benchmark that flatters itself.
#[derive(Debug, Serialize)]
pub struct FailureRow {
    pub algo: String,
    pub backend: String,
    pub language: String,
    pub compiler: String,
    pub interpreter: String,
    pub mode: String,
    /// `build` — the image never built — or `round 3 of the run phase`: where it
    /// was when it went.
    pub stage: String,
    /// The error and its full context chain, on one line: a Markdown table cell
    /// has no room for the newlines a compiler is fond of.
    pub error: String,
}

#[derive(Debug, Serialize)]
pub struct AlgoReport {
    pub algo: String,
    /// The reference every strict-mode row of *this* algorithm agreed on. It is
    /// a property of `(algo, grid size, max_iter)`, never of the campaign.
    pub strict_checksum: String,
    pub rows: Vec<Row>,
}

/// One backend's identity card, as its manifest declared it.
#[derive(Debug, Serialize)]
pub struct Backend {
    /// `mandelbrot-python-cython-cpython`: the heading of this backend's section,
    /// and therefore the anchor every row of the tables links to. Computed in
    /// `analysis`, so the link and its target cannot disagree.
    pub id: String,
    pub algo: String,
    pub backend: String,
    pub language: String,
    pub compiler: String,
    pub interpreter: String,
    pub description: String,
    /// Empty when the manifest declared none — Liquid tests it for truthiness.
    pub comments: String,
}

#[derive(Debug, Serialize)]
pub struct Row {
    /// The (language, compiler, interpreter) triple as one token. Not a column:
    /// the table spells the three out.
    pub backend: String,
    /// Anchor of this row's entry in **Backends**, so a reader who does not know
    /// what `cython` is can find out without leaving the report.
    pub backend_id: String,
    pub language: String,
    /// `n/a` when the backend compiles nothing ahead of the run — a fact about
    /// it, not a hole in the data.
    pub compiler: String,
    /// `n/a` when the backend ships machine code and no runtime.
    pub interpreter: String,
    pub mode: String,
    pub run_min: String,
    pub run_dispersion: String,
    pub run_samples: usize,
    pub compute_min: String,
    pub startup: String,
    pub cpu_time: String,
    /// Cores kept busy, against the thread count the harness handed out. It can
    /// exceed that count: a runtime's JIT and GC threads burn CPU the hot loop's
    /// own clock never sees. See [`crate::sample::Sample::cores_milli`].
    pub cores: String,
    pub memory: String,
    /// `n/a` on a host with no readable RAPL counter, which the machine table
    /// above says in as many words.
    pub energy: String,
    pub build_min: String,
    pub build_dispersion: String,
    pub source: String,
    pub binary: String,
    pub text: String,
    pub checksum_delta: String,
}

/// The report is the default analysis, formatted. It has no options of its own:
/// `report.md` is the campaign as the methodology defines it — min-of-N, warmup
/// rounds recorded but never aggregated.
pub fn build(recording: &Recording) -> ReportData {
    let analysis = analyze(recording, Options::default());
    format(analysis)
}

fn format(analysis: Analysis) -> ReportData {
    ReportData {
        campaign: analysis.campaign,
        machine_fields: analysis.machine_fields,
        warnings: analysis.warnings,
        algos: analysis
            .algos
            .into_iter()
            .map(|algo| AlgoReport {
                algo: algo.algo,
                strict_checksum: algo
                    .strict_checksum
                    .map_or_else(|| "n/a".to_owned(), |checksum| checksum.to_string()),
                rows: algo.aggregates.iter().map(row).collect(),
            })
            .collect(),
        backends: analysis
            .backends
            .into_iter()
            .map(|backend| Backend {
                id: backend.id,
                algo: backend.algo,
                backend: backend.backend,
                language: backend.language,
                compiler: opt(backend.compiler.as_deref()),
                interpreter: opt(backend.interpreter.as_deref()),
                description: backend.description,
                comments: backend.comments.unwrap_or_default(),
            })
            .collect(),
        failures: analysis.failures.iter().map(failure_row).collect(),
    }
}

fn failure_row(failure: &analysis::Failure) -> FailureRow {
    FailureRow {
        algo: failure.algo.clone(),
        backend: failure.backend.clone(),
        language: failure.language.clone(),
        compiler: opt(failure.compiler.as_deref()),
        interpreter: opt(failure.interpreter.as_deref()),
        mode: failure.mode.to_string(),
        stage: match (failure.stage, failure.phase, failure.round) {
            (Stage::Prepare, _, _) => "build".to_owned(),
            (Stage::Measure, Some(phase), Some(round)) => {
                // Rounds are counted from zero inside the harness and from one for
                // a reader: the campaign log says "round 1 of 10", and this must
                // agree with it.
                format!("{} round {}", phase.as_str(), round + 1)
            }
            (Stage::Measure, _, _) => "run".to_owned(),
        },
        error: one_line(&failure.error),
    }
}

/// A Markdown table cell is one line, and a compiler's diagnostics are not. The
/// pipes go too — they would end the cell early and shear the row in half.
fn one_line(error: &str) -> String {
    error
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
        .replace('|', "\\|")
}

fn row(aggregate: &Aggregate) -> Row {
    Row {
        backend: aggregate.backend.clone(),
        backend_id: aggregate.backend_id.clone(),
        language: aggregate.language.clone(),
        compiler: opt(aggregate.compiler.as_deref()),
        interpreter: opt(aggregate.interpreter.as_deref()),
        mode: aggregate.mode.to_string(),
        run_min: min_ms(aggregate.run_wall),
        run_dispersion: dispersion(aggregate.run_wall),
        run_samples: aggregate.run_wall.map_or(0, |summary| summary.n),
        compute_min: min_ms(aggregate.run_elapsed),
        startup: min_ms(aggregate.run_startup),
        cpu_time: aggregate.run_cpu_usec.map_or_else(
            || "n/a".to_owned(),
            |summary| format!("{:.2} s", summary.median as f64 / 1e6),
        ),
        // The median, not the minimum — the one number on this row that is neither.
        // See `Aggregate::run_cores`.
        cores: aggregate.run_cores.map_or_else(
            || "n/a".to_owned(),
            |summary| format!("{:.1} / {}", summary.median as f64 / 1e3, aggregate.cpu),
        ),
        memory: mebibytes(aggregate.run_peak_bytes.map(|summary| summary.min)),
        energy: aggregate.run_energy_uj.map_or_else(
            || "n/a".to_owned(),
            |summary| format!("{:.1} J", summary.min as f64 / 1e6),
        ),
        build_min: min_ms(aggregate.build_elapsed),
        build_dispersion: dispersion(aggregate.build_elapsed),
        source: bytes(aggregate.source_bytes),
        binary: bytes(aggregate.binary_bytes),
        text: bytes(aggregate.text_bytes),
        checksum_delta: delta(aggregate.checksum_delta),
    }
}

pub fn render(data: &ReportData, template: &str) -> Result<String> {
    let template = liquid::ParserBuilder::with_stdlib()
        .build()
        .context("building the Liquid parser")?
        .parse(template)
        .context("parsing the report template")?;
    let globals = liquid::to_object(data).context("serializing the report data")?;
    let rendered = template.render(&globals).context("rendering the report")?;

    // A rendered report ends with exactly one newline. Where a Liquid loop happens
    // to leave a blank line is an accident of the template, and a report the repo's
    // own `end-of-file-fixer` wants to rewrite is a report the campaign cannot
    // commit. Normalising here rather than in `report.md.liquid` means a
    // `--template` of your own inherits the guarantee instead of rediscovering it.
    Ok(format!("{}\n", rendered.trim_end()))
}

/// An absent half of the triple is a fact about the backend — a compiled binary
/// has no interpreter — so it is rendered, never blanked. A blank cell reads as a
/// rendering bug.
fn opt(value: Option<&str>) -> String {
    value.unwrap_or("n/a").to_owned()
}

/// The unit belongs to the value, not to the template: `n/a ms` is nonsense.
fn min_ms(summary: Option<Summary>) -> String {
    summary.map_or_else(
        || "n/a".to_owned(),
        |summary| format!("{:.1} ms", summary.min as f64 / 1e6),
    )
}

/// Below three samples the median absolute deviation is structurally zero — the
/// lower median of `[0, d]` is `0` — so reporting it would claim a precision
/// the campaign never had.
fn dispersion(summary: Option<Summary>) -> String {
    match summary {
        Some(summary) if summary.n >= 3 => format!("{:.2}%", summary.mad_pct),
        Some(summary) => format!("n/a (n={})", summary.n),
        None => "n/a".to_owned(),
    }
}

fn bytes(value: Option<u64>) -> String {
    match value {
        None => "n/a".to_owned(),
        Some(bytes) if bytes < 1024 => format!("{bytes} B"),
        Some(bytes) => format!("{:.1} KiB", bytes as f64 / 1024.0),
    }
}

/// A whole container's memory, which is megabytes and not kilobytes: a JVM's peak
/// in KiB is a six-digit number nobody can read at a glance.
fn mebibytes(value: Option<u64>) -> String {
    value.map_or_else(
        || "n/a".to_owned(),
        |bytes| format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0)),
    )
}

/// A relaxed mode's distance from the strict reference: the precision sold for
/// the speed gained.
fn delta(delta: Option<i128>) -> String {
    match delta {
        None => "n/a".to_owned(),
        Some(0) => "0".to_owned(),
        Some(delta) => format!("{delta:+}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::Machine;
    use crate::mode::FpMode;
    use crate::sample::{Phase, Sample};

    fn implementations(data: &ReportData) -> Vec<&str> {
        data.algos[0]
            .rows
            .iter()
            .map(|row| row.backend.as_str())
            .collect()
    }

    fn recording(samples: Vec<Sample>) -> Recording {
        Recording {
            machine: Machine::default(),
            campaign: campaign(),
            samples,
            failures: Vec::new(),
        }
    }

    /// `backend` is spelled as its slug — `c-gcc`, `python-cpython` — and split
    /// back into the triple the sample actually carries.
    fn sample(backend: &str, mode: FpMode, phase: Phase, warmup: bool, wall: u64) -> Sample {
        let (language, compiler) = backend.split_once('-').expect("<language>-<compiler>");
        Sample {
            algo: "mandelbrot".to_owned(),
            language: language.to_owned(),
            compiler: Some(compiler.to_owned()),
            interpreter: None,
            description: format!("{backend}, as the fixture declares it"),
            comments: None,
            mode,
            phase,
            round: 0,
            warmup,
            cpu: 8,
            wall_ns: wall,
            elapsed_ns: wall / 2,
            user_usec: 1_000,
            system_usec: 0,
            energy_uj: Some(9_400_000),
            peak_bytes: Some(12_582_912),
            source_bytes: Some(2_048),
            checksum: Some(42),
            binary_bytes: Some(2048),
            binary_stripped_bytes: None,
            text_bytes: Some(1024),
        }
    }

    fn campaign() -> Campaign {
        Campaign {
            langbench_version: "0.1.0".to_owned(),
            timestamp: "2026-07-09T12:00:00Z".to_owned(),
            cpu: 8,
            grid_size: 4096,
            max_iter: 1000,
            rounds: 30,
            build_rounds: 5,
            warmup_rounds: 2,
            march: "x86-64-v3".to_owned(),
            modes: vec!["strict".to_owned()],
        }
    }

    #[test]
    fn warmup_samples_never_enter_the_aggregates() {
        let samples = vec![
            sample("c-gcc", FpMode::Strict, Phase::Run, true, 9_000_000_000),
            sample("c-gcc", FpMode::Strict, Phase::Run, false, 2_000_000_000),
        ];
        let data = build(&recording(samples));
        let row = &data.algos[0].rows[0];
        assert_eq!(row.run_samples, 1);
        assert_eq!(row.run_min, "2000.0 ms");
    }

    #[test]
    fn build_and_run_phases_land_in_separate_columns() {
        let samples = vec![
            sample("c-gcc", FpMode::Strict, Phase::Build, false, 800_000_000),
            sample("c-gcc", FpMode::Strict, Phase::Run, false, 2_000_000_000),
        ];
        let data = build(&recording(samples));
        let row = &data.algos[0].rows[0];
        // `sample()` halves the wall to get the elapsed, and the build column
        // reports the elapsed. See the next test.
        assert_eq!(row.build_min, "400.0 ms");
        assert_eq!(row.run_min, "2000.0 ms");
    }

    /// The build column times the compiler, not Docker.
    ///
    /// A `docker run` costs ~110 ms of container creation here, several times a
    /// `gcc` invocation on a single-file kernel. Reporting the wall-clock would
    /// bury the compilers under a constant that says nothing about them, and
    /// would flatter a slow compiler by the same 110 ms it charges a fast one.
    #[test]
    fn the_build_column_excludes_container_creation() {
        let mut build_sample = sample("c-gcc", FpMode::Strict, Phase::Build, false, 0);
        build_sample.wall_ns = 142_000_000; // what `docker run` took
        build_sample.elapsed_ns = 30_000_000; // what gcc took

        let data = build(&recording(vec![build_sample]));
        assert_eq!(data.algos[0].rows[0].build_min, "30.0 ms");
    }

    #[test]
    fn an_implementation_with_no_build_phase_reports_not_available() {
        let samples = vec![sample(
            "py-cpython",
            FpMode::Strict,
            Phase::Run,
            false,
            1_000_000,
        )];
        let data = build(&recording(samples));
        assert_eq!(data.algos[0].rows[0].build_min, "n/a");
    }

    #[test]
    fn a_relaxed_mode_reports_its_distance_from_the_strict_reference() {
        // The reference is read back out of the samples, not handed in: a report
        // rendered from a file has nothing else to go on. `sample()` checksums 42.
        let reference = sample("c-gcc", FpMode::Strict, Phase::Run, false, 1_000_000);
        let mut divergent = sample("c-gcc", FpMode::Fast, Phase::Run, false, 1_000_000);
        divergent.checksum = Some(40);
        let data = build(&recording(vec![reference, divergent]));
        assert_eq!(data.algos[0].rows[1].checksum_delta, "-2");
    }

    #[test]
    fn a_campaign_without_a_strict_mode_has_no_reference_to_measure_against() {
        let mut relaxed = sample("c-gcc", FpMode::Fast, Phase::Run, false, 1_000_000);
        relaxed.checksum = Some(40);
        let data = build(&recording(vec![relaxed]));
        assert_eq!(data.algos[0].strict_checksum, "n/a");
        assert_eq!(data.algos[0].rows[0].checksum_delta, "n/a");
    }

    #[test]
    fn each_algorithm_measures_its_delta_against_its_own_reference() {
        // The checksum is a property of (algo, grid size, max_iter). Measuring
        // the second algorithm against the first one's reference would report a
        // huge bogus delta on the column that gates correctness.
        let first = sample("c-gcc", FpMode::Strict, Phase::Run, false, 1_000_000);

        let mut second = sample("c-gcc", FpMode::Strict, Phase::Run, false, 1_000_000);
        second.algo = "nbody".to_owned();
        second.checksum = Some(1_000);

        let mut relaxed = sample("c-gcc", FpMode::Fast, Phase::Run, false, 1_000_000);
        relaxed.algo = "nbody".to_owned();
        relaxed.checksum = Some(997);

        let data = build(&recording(vec![first, second, relaxed]));
        assert_eq!(data.algos[0].strict_checksum, "42");
        assert_eq!(data.algos[1].strict_checksum, "1000");
        assert_eq!(data.algos[1].rows[0].checksum_delta, "0");
        assert_eq!(data.algos[1].rows[1].checksum_delta, "-3");
    }

    /// A backend that broke has no row in the tables, and a row that is absent
    /// reads exactly like a backend nobody ever wrote. The report says which ones
    /// it lost, and to what.
    #[test]
    fn the_report_names_the_backends_the_campaign_lost() {
        let mut recording = recording(vec![sample(
            "c-gcc",
            FpMode::Strict,
            Phase::Run,
            false,
            2_000_000_000,
        )]);
        recording.failures = vec![crate::sample::Failure {
            algo: "mandelbrot".to_owned(),
            language: "rust".to_owned(),
            compiler: Some("llvm".to_owned()),
            interpreter: None,
            description: "Rust, as the fixture declares it.".to_owned(),
            comments: None,
            mode: FpMode::Strict,
            stage: Stage::Measure,
            phase: Some(Phase::Run),
            round: Some(2),
            // Multi-line and full of pipes, like every real one: a Markdown cell is
            // one line, and an unescaped pipe shears the row in half.
            error: "`docker run` failed\nSegmentation fault | core dumped".to_owned(),
        }];

        let data = build(&recording);
        assert_eq!(data.failures.len(), 1);
        assert_eq!(data.failures[0].backend, "rust-llvm");
        // Zero-based inside the harness, one-based for a reader — the campaign log
        // says "round 3 of 10", and this has to agree with it.
        assert_eq!(data.failures[0].stage, "run round 3");
        assert_eq!(
            data.failures[0].error,
            "`docker run` failed · Segmentation fault \\| core dumped",
        );

        let markdown = render(&data, DEFAULT_TEMPLATE).unwrap();
        assert!(markdown.contains("## What did not finish"), "{markdown}");
        assert!(markdown.contains("rust"), "{markdown}");
    }

    /// A clean campaign does not get a section confessing to nothing.
    #[test]
    fn a_campaign_that_lost_nothing_has_no_failures_section() {
        let data = build(&recording(vec![sample(
            "c-gcc",
            FpMode::Strict,
            Phase::Run,
            false,
            2_000_000_000,
        )]));
        let markdown = render(&data, DEFAULT_TEMPLATE).unwrap();
        assert!(!markdown.contains("What did not finish"), "{markdown}");
    }

    #[test]
    fn a_custom_template_replaces_the_built_in_one() {
        let data = build(&recording(vec![sample(
            "c-gcc",
            FpMode::Strict,
            Phase::Run,
            false,
            2_000_000_000,
        )]));
        let markdown = render(
            &data,
            "{% for algo in algos %}{{ algo.algo }}:{% for row in algo.rows %}{{ row.run_min }}{% endfor %}{% endfor %}",
        )
        .unwrap();
        // The trailing newline is the renderer's, not the template's: a report is a
        // text file, and a text file ends with one.
        assert_eq!(markdown, "mandelbrot:2000.0 ms\n");
    }

    #[test]
    fn a_broken_template_names_the_template_not_the_data() {
        let data = build(&recording(vec![]));
        let error = render(&data, "{% for %}").unwrap_err();
        assert!(error.to_string().contains("template"), "{error}");
    }

    /// Startup is a gap observed within one run, never the difference of two
    /// minima drawn from different rounds — that describes a run that never
    /// happened, and on a noisy host it can exceed every gap actually measured.
    #[test]
    fn startup_is_the_smallest_gap_of_a_single_run_not_the_difference_of_the_minima() {
        let mut fast_wall = sample("c-gcc", FpMode::Strict, Phase::Run, false, 0);
        fast_wall.wall_ns = 350_000_000;
        fast_wall.elapsed_ns = 240_000_000; // gap: 110 ms

        let mut fast_compute = sample("c-gcc", FpMode::Strict, Phase::Run, false, 0);
        fast_compute.wall_ns = 400_000_000;
        fast_compute.elapsed_ns = 230_000_000; // gap: 170 ms

        let data = build(&recording(vec![fast_wall, fast_compute]));
        let row = &data.algos[0].rows[0];
        assert_eq!(row.run_min, "350.0 ms");
        assert_eq!(row.compute_min, "230.0 ms");
        // The difference of the two minima would be 120 ms, a run nobody observed.
        assert_eq!(row.startup, "110.0 ms");
    }

    #[test]
    fn dispersion_below_three_samples_is_not_reported_as_zero() {
        // The lower median of `[0, d]` is `0`, so a two-sample MAD is
        // structurally zero and would claim a precision we never measured.
        let samples = vec![
            sample("c-gcc", FpMode::Strict, Phase::Run, false, 1_000_000),
            sample("c-gcc", FpMode::Strict, Phase::Run, false, 5_000_000),
        ];
        let data = build(&recording(samples));
        assert_eq!(data.algos[0].rows[0].run_dispersion, "n/a (n=2)");
    }

    #[test]
    fn a_strict_mode_row_shows_a_bare_zero_delta() {
        let samples = vec![sample(
            "c-gcc",
            FpMode::Strict,
            Phase::Run,
            false,
            1_000_000,
        )];
        let data = build(&recording(samples));
        assert_eq!(data.algos[0].rows[0].checksum_delta, "0");
    }

    /// The sort is stable, so the schedule order survives as the tiebreak.
    #[test]
    fn rows_of_equal_speed_keep_the_schedule_order() {
        let samples = vec![
            sample("c-gcc", FpMode::Strict, Phase::Run, false, 1_000_000),
            sample("rust-llvm", FpMode::Strict, Phase::Run, false, 1_000_000),
            sample("c-gcc", FpMode::Strict, Phase::Run, false, 1_000_000),
        ];
        let data = build(&recording(samples));
        assert_eq!(implementations(&data), ["c-gcc", "rust-llvm"]);
    }

    /// A backend is described once, at the end, however many modes it was built
    /// under — and every row's link lands on a heading that exists. A dead anchor
    /// is worse than no link: it looks like the explanation is missing rather
    /// than misfiled.
    #[test]
    fn every_row_links_to_a_backend_section_that_exists() {
        let samples = vec![
            sample("c-gcc", FpMode::Strict, Phase::Run, false, 1_000_000),
            sample("c-gcc", FpMode::Fast, Phase::Run, false, 1_000_000),
            sample(
                "python-cpython",
                FpMode::Strict,
                Phase::Run,
                false,
                9_000_000,
            ),
        ];
        let data = build(&recording(samples));

        // Once per backend, not once per (backend, mode): `c-gcc` has two rows.
        assert_eq!(data.backends.len(), 2);

        let markdown = render(&data, DEFAULT_TEMPLATE).unwrap();
        for row in &data.algos[0].rows {
            assert!(
                markdown.contains(&format!("](#{})", row.backend_id)),
                "row `{}` has no link",
                row.backend,
            );
            assert!(
                markdown.contains(&format!("### {}\n", row.backend_id)),
                "`{}` links to a heading that does not exist",
                row.backend_id,
            );
        }
    }

    #[test]
    fn rows_are_sorted_fastest_first() {
        let samples = vec![
            sample("c-gcc", FpMode::Strict, Phase::Run, false, 3_000_000),
            sample(
                "python-cpython",
                FpMode::Strict,
                Phase::Run,
                false,
                9_000_000,
            ),
            sample("rust-llvm", FpMode::Strict, Phase::Run, false, 1_000_000),
        ];
        let data = build(&recording(samples));
        assert_eq!(
            implementations(&data),
            ["rust-llvm", "c-gcc", "python-cpython"],
        );
    }

    /// The ranking is on the minimum, the statistic the table headlines — not on
    /// the order the samples happened to arrive in.
    #[test]
    fn the_ranking_is_on_the_minimum_not_the_first_sample() {
        let samples = vec![
            sample("c-gcc", FpMode::Strict, Phase::Run, false, 1_000_000),
            sample("c-gcc", FpMode::Strict, Phase::Run, false, 9_000_000),
            sample("rust-llvm", FpMode::Strict, Phase::Run, false, 2_000_000),
            sample("rust-llvm", FpMode::Strict, Phase::Run, false, 2_000_000),
        ];
        let data = build(&recording(samples));
        assert_eq!(implementations(&data), ["c-gcc", "rust-llvm"]);
    }

    /// A build-only row has no run to be ranked on. It sorts last rather than
    /// ahead of every measured row, which a `None`-is-zero sort would do.
    #[test]
    fn a_row_without_a_measured_run_sorts_last() {
        let samples = vec![
            sample("c-gcc", FpMode::Strict, Phase::Build, false, 1_000_000),
            sample("rust-llvm", FpMode::Strict, Phase::Run, false, 5_000_000),
        ];
        let data = build(&recording(samples));
        assert_eq!(implementations(&data), ["rust-llvm", "c-gcc"]);
    }

    #[test]
    fn the_template_renders_with_real_data() {
        let samples = vec![sample(
            "c-gcc",
            FpMode::Strict,
            Phase::Run,
            false,
            2_000_000_000,
        )];
        let data = build(&recording(samples));
        let markdown = render(&data, DEFAULT_TEMPLATE).unwrap();
        assert!(markdown.contains("mandelbrot"));
        assert!(markdown.contains("c-gcc"));
        assert!(markdown.contains("2000.0"));
    }

    #[test]
    fn every_column_of_the_results_table_is_documented() {
        let samples = vec![sample(
            "c-gcc",
            FpMode::Strict,
            Phase::Run,
            false,
            2_000_000_000,
        )];
        let markdown = render(&build(&recording(samples)), DEFAULT_TEMPLATE).unwrap();

        let header = markdown
            .lines()
            .find(|line| line.starts_with("| Language |"))
            .expect("the results table has a header row");
        let columns: Vec<&str> = header
            .split('|')
            .map(str::trim)
            .filter(|cell| !cell.is_empty())
            .collect();

        // Guard against a vacuous pass: an empty column list would assert nothing.
        assert!(columns.len() > 10, "parsed {} columns", columns.len());
        for column in columns {
            assert!(
                markdown.contains(&format!("### {column}\n")),
                "column `{column}` has no section in the column reference",
            );
        }
    }

    #[test]
    fn the_report_surfaces_the_hosts_warnings() {
        let data = build(&recording(vec![]));
        let markdown = render(&data, DEFAULT_TEMPLATE).unwrap();
        assert!(markdown.contains("Linux"));
        assert!(markdown.contains("n/a"));
    }

    // The campaign commits what this renders, and `pre-commit` gates that commit.
    // A report that ends in a blank line is one the bot cannot push.
    #[test]
    fn a_rendered_report_ends_with_exactly_one_newline() {
        let markdown = render(&build(&recording(vec![])), DEFAULT_TEMPLATE).unwrap();
        assert!(markdown.ends_with('\n'));
        assert!(!markdown.ends_with("\n\n"));

        // Whatever slack a template leaves at its end, however much of it.
        let sloppy = render(&build(&recording(vec![])), "# report  \n\n\n").unwrap();
        assert_eq!(sloppy, "# report\n");
    }
}
