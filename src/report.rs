//! Human-facing Markdown report, rendered from `templates/report.md.liquid`.
//!
//! Everything here is derived from the raw samples and can be recomputed from
//! `samples.ndjson`. Aggregates never replace the samples.

use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::cli::FpMode;
use crate::machine::Field;
use crate::sample::{Campaign, Phase, Recording, Sample};
use crate::stats::{Summary, summarize};

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
    /// and therefore the anchor every row of the tables links to. Computed here
    /// rather than in the template, so the link and its target cannot disagree.
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

/// The anchor of a backend's section, and the heading it is generated from.
///
/// Markdown renderers derive an anchor from the heading text, and they do not all
/// derive it the same way. So the heading *is* the anchor: lowercase, no spaces,
/// no punctuation — nothing for a renderer to reinterpret.
fn backend_id(algo: &str, backend: &str) -> String {
    format!("{algo}-{backend}")
}

#[derive(Debug, Serialize)]
pub struct Row {
    /// The value `run_min` was formatted from, kept to sort rows fastest-first.
    /// Not rendered: the template has the formatted string. `None` — a row with
    /// no measured run — sorts last, having no speed to be ranked on.
    #[serde(skip)]
    pub run_min_ns: Option<u64>,
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
    pub build_min: String,
    pub build_dispersion: String,
    pub binary: String,
    pub text: String,
    pub checksum_delta: String,
}

/// Samples grouped by (algorithm, backend, FP mode).
#[derive(Default)]
struct Bucket {
    language: String,
    compiler: Option<String>,
    interpreter: Option<String>,
    description: String,
    comments: Option<String>,
    run_wall: Vec<u64>,
    run_elapsed: Vec<u64>,
    run_startup: Vec<u64>,
    run_cpu_usec: Vec<u64>,
    /// The compiler's own elapsed time, reported by the entrypoint from inside
    /// the container — never the `docker run` wall-clock. Container creation
    /// costs ~110 ms here, which is several times a `gcc` invocation on a
    /// single kernel: timing the wall would rank Docker, not the compilers.
    /// The run row keeps its wall-clock because a runtime's startup is a result;
    /// a container's is an artefact of how we chose to isolate the build.
    build_elapsed: Vec<u64>,
    checksum: Option<u64>,
    binary_bytes: Option<u64>,
    text_bytes: Option<u64>,
}

pub fn build(recording: &Recording) -> ReportData {
    let samples = &recording.samples;
    let references = strict_references(samples);

    // Insertion order comes from the first round, which is the schedule order.
    let mut order: Vec<(String, String, String)> = Vec::new();
    let mut buckets: HashMap<(String, String, String), Bucket> = HashMap::new();

    for sample in samples {
        let key = (
            sample.algo.clone(),
            sample.backend(),
            sample.mode.to_string(),
        );
        let bucket = buckets.entry(key.clone()).or_insert_with(|| {
            order.push(key);
            Bucket {
                language: sample.language.clone(),
                compiler: sample.compiler.clone(),
                interpreter: sample.interpreter.clone(),
                description: sample.description.clone(),
                comments: sample.comments.clone(),
                ..Bucket::default()
            }
        });

        // Constants of the image: take them wherever they first appear.
        bucket.checksum = bucket.checksum.or(sample.checksum);
        bucket.binary_bytes = bucket.binary_bytes.or(sample.binary_bytes);
        bucket.text_bytes = bucket.text_bytes.or(sample.text_bytes);

        // Warmup samples are recorded, never aggregated.
        if sample.warmup {
            continue;
        }
        match sample.phase {
            Phase::Build => bucket.build_elapsed.push(sample.elapsed_ns),
            Phase::Run => {
                bucket.run_wall.push(sample.wall_ns);
                bucket.run_elapsed.push(sample.elapsed_ns);
                bucket.run_startup.push(sample.startup_ns());
                bucket.run_cpu_usec.push(sample.cpu_usec());
            }
        }
    }

    let mut algos: Vec<AlgoReport> = Vec::new();
    let mut backends: Vec<Backend> = Vec::new();
    for key in &order {
        let (algo, backend, mode) = key;
        let bucket = &buckets[key];
        let reference = references.get(algo).copied();
        let row = Row {
            run_min_ns: summarize(&bucket.run_wall).map(|summary| summary.min),
            backend: backend.clone(),
            backend_id: backend_id(algo, backend),
            language: bucket.language.clone(),
            compiler: opt(bucket.compiler.as_deref()),
            interpreter: opt(bucket.interpreter.as_deref()),
            mode: mode.clone(),
            run_min: min_ms(summarize(&bucket.run_wall)),
            run_dispersion: dispersion(summarize(&bucket.run_wall)),
            run_samples: bucket.run_wall.len(),
            compute_min: min_ms(summarize(&bucket.run_elapsed)),
            startup: min_ms(summarize(&bucket.run_startup)),
            cpu_time: summarize(&bucket.run_cpu_usec).map_or_else(
                || "n/a".to_owned(),
                |summary| format!("{:.2} s", summary.median as f64 / 1e6),
            ),
            build_min: min_ms(summarize(&bucket.build_elapsed)),
            build_dispersion: dispersion(summarize(&bucket.build_elapsed)),
            binary: bytes(bucket.binary_bytes),
            text: bytes(bucket.text_bytes),
            checksum_delta: delta(bucket.checksum, reference),
        };

        // One card per backend, not one per (backend, mode): the three modes are
        // three experiments on the same thing, and the thing is what a card
        // describes.
        let id = backend_id(algo, backend);
        if !backends.iter().any(|known: &Backend| known.id == id) {
            backends.push(Backend {
                id,
                algo: algo.clone(),
                backend: backend.clone(),
                language: bucket.language.clone(),
                compiler: opt(bucket.compiler.as_deref()),
                interpreter: opt(bucket.interpreter.as_deref()),
                description: bucket.description.clone(),
                comments: bucket.comments.clone().unwrap_or_default(),
            });
        }

        match algos.iter_mut().find(|report| &report.algo == algo) {
            Some(report) => report.rows.push(row),
            None => algos.push(AlgoReport {
                algo: algo.clone(),
                strict_checksum: reference
                    .map_or_else(|| "n/a".to_owned(), |checksum| checksum.to_string()),
                rows: vec![row],
            }),
        }
    }

    // Fastest first, on the same statistic the table headlines: the minimum wall
    // clock. `sort_by_key` is stable, so rows the campaign could not measure keep
    // their schedule order at the bottom instead of being shuffled among
    // themselves.
    for report in &mut algos {
        report
            .rows
            .sort_by_key(|row| (row.run_min_ns.is_none(), row.run_min_ns));
    }

    ReportData {
        campaign: recording.campaign.clone(),
        machine_fields: recording.machine.fields(),
        warnings: recording.machine.warnings(),
        algos,
        backends,
    }
}

/// The value every strict-mode run of a given algorithm agreed on, keyed by
/// algorithm.
///
/// The campaign already refused to record a divergent one — `Runner::verify`
/// aborts on the spot — so any strict sample of an algorithm carries its
/// reference and the first one is as good as the last. The reference is per
/// algorithm because the checksum is a property of `(algo, grid size,
/// max_iter)`: measuring one algorithm's delta against another's would be
/// meaningless. See `METHODOLOGY.md#the-strict-mode-invariant`.
fn strict_references(samples: &[Sample]) -> HashMap<String, u64> {
    let mut references = HashMap::new();
    for sample in samples {
        if sample.mode != FpMode::Strict {
            continue;
        }
        if let Some(checksum) = sample.checksum {
            references.entry(sample.algo.clone()).or_insert(checksum);
        }
    }
    references
}

pub fn render(data: &ReportData, template: &str) -> Result<String> {
    let template = liquid::ParserBuilder::with_stdlib()
        .build()
        .context("building the Liquid parser")?
        .parse(template)
        .context("parsing the report template")?;
    let globals = liquid::to_object(data).context("serializing the report data")?;
    template.render(&globals).context("rendering the report")
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

/// A relaxed mode's distance from the strict reference: the precision sold for
/// the speed gained.
fn delta(checksum: Option<u64>, reference: Option<u64>) -> String {
    match (checksum, reference) {
        (Some(checksum), Some(reference)) => match i128::from(checksum) - i128::from(reference) {
            0 => "0".to_owned(),
            delta => format!("{delta:+}"),
        },
        _ => "n/a".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::Machine;

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
        assert_eq!(markdown, "mandelbrot:2000.0 ms");
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
}
