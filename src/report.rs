//! Human-facing Markdown report, rendered from `templates/report.md.liquid`.
//!
//! Everything here is derived from the raw samples and can be recomputed from
//! `samples.ndjson`. Aggregates never replace the samples.

use std::collections::HashMap;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::machine::{Field, Machine};
use crate::sample::{Campaign, Phase, Sample};
use crate::stats::{Summary, summarize};

const TEMPLATE: &str = include_str!("../templates/report.md.liquid");

#[derive(Debug, Serialize)]
pub struct ReportData {
    pub campaign: Campaign,
    pub machine_fields: Vec<Field>,
    pub warnings: Vec<String>,
    pub strict_checksum: String,
    pub algos: Vec<AlgoReport>,
}

#[derive(Debug, Serialize)]
pub struct AlgoReport {
    pub algo: String,
    pub rows: Vec<Row>,
}

#[derive(Debug, Serialize)]
pub struct Row {
    pub implementation: String,
    pub language: String,
    pub compiler: String,
    pub mode: String,
    pub run_min_ms: String,
    pub run_dispersion: String,
    pub run_samples: usize,
    pub compute_min_ms: String,
    pub startup_ms: String,
    pub cpu_time_s: String,
    pub build_min_ms: String,
    pub build_dispersion: String,
    pub binary: String,
    pub text: String,
    pub checksum: String,
    pub checksum_delta: String,
}

/// Samples grouped by (algorithm, implementation, FP mode).
#[derive(Default)]
struct Bucket {
    language: String,
    compiler: String,
    run_wall: Vec<u64>,
    run_elapsed: Vec<u64>,
    run_startup: Vec<u64>,
    run_cpu_usec: Vec<u64>,
    build_wall: Vec<u64>,
    checksum: Option<u64>,
    binary_bytes: Option<u64>,
    text_bytes: Option<u64>,
}

pub fn build(
    machine: &Machine,
    campaign: &Campaign,
    samples: &[Sample],
    strict_checksum: Option<u64>,
) -> ReportData {
    // Insertion order comes from the first round, which is the schedule order.
    let mut order: Vec<(String, String, String)> = Vec::new();
    let mut buckets: HashMap<(String, String, String), Bucket> = HashMap::new();

    for sample in samples {
        let key = (
            sample.algo.clone(),
            sample.implementation.clone(),
            sample.mode.to_string(),
        );
        let bucket = buckets.entry(key.clone()).or_insert_with(|| {
            order.push(key);
            Bucket {
                language: sample.language.clone(),
                compiler: sample.compiler.clone(),
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
            Phase::Build => bucket.build_wall.push(sample.wall_ns),
            Phase::Run => {
                bucket.run_wall.push(sample.wall_ns);
                bucket.run_elapsed.push(sample.elapsed_ns);
                bucket.run_startup.push(sample.startup_ns());
                bucket.run_cpu_usec.push(sample.cpu_usec());
            }
        }
    }

    let mut algos: Vec<AlgoReport> = Vec::new();
    for key in &order {
        let (algo, implementation, mode) = key;
        let bucket = &buckets[key];
        let row = Row {
            implementation: implementation.clone(),
            language: bucket.language.clone(),
            compiler: bucket.compiler.clone(),
            mode: mode.clone(),
            run_min_ms: min_ms(summarize(&bucket.run_wall)),
            run_dispersion: dispersion(summarize(&bucket.run_wall)),
            run_samples: bucket.run_wall.len(),
            compute_min_ms: min_ms(summarize(&bucket.run_elapsed)),
            startup_ms: min_ms(summarize(&bucket.run_startup)),
            cpu_time_s: summarize(&bucket.run_cpu_usec).map_or_else(
                || "n/a".to_owned(),
                |s| format!("{:.2}", s.median as f64 / 1e6),
            ),
            build_min_ms: min_ms(summarize(&bucket.build_wall)),
            build_dispersion: dispersion(summarize(&bucket.build_wall)),
            binary: bytes(bucket.binary_bytes),
            text: bytes(bucket.text_bytes),
            checksum: bucket
                .checksum
                .map_or_else(|| "n/a".to_owned(), |c| c.to_string()),
            checksum_delta: delta(bucket.checksum, strict_checksum),
        };

        match algos.iter_mut().find(|report| &report.algo == algo) {
            Some(report) => report.rows.push(row),
            None => algos.push(AlgoReport {
                algo: algo.clone(),
                rows: vec![row],
            }),
        }
    }

    ReportData {
        campaign: campaign.clone(),
        machine_fields: machine.fields(),
        warnings: machine.warnings(),
        strict_checksum: strict_checksum.map_or_else(|| "n/a".to_owned(), |c| c.to_string()),
        algos,
    }
}

pub fn render(data: &ReportData) -> Result<String> {
    let template = liquid::ParserBuilder::with_stdlib()
        .build()
        .context("building the Liquid parser")?
        .parse(TEMPLATE)
        .context("parsing templates/report.md.liquid")?;
    let globals = liquid::to_object(data).context("serializing the report data")?;
    template.render(&globals).context("rendering the report")
}

fn min_ms(summary: Option<Summary>) -> String {
    summary.map_or_else(
        || "n/a".to_owned(),
        |s| format!("{:.1}", s.min as f64 / 1e6),
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
    use crate::cli::FpMode;

    fn sample(implementation: &str, mode: FpMode, phase: Phase, warmup: bool, wall: u64) -> Sample {
        Sample {
            algo: "mandelbrot".to_owned(),
            implementation: implementation.to_owned(),
            language: "c".to_owned(),
            compiler: "gcc".to_owned(),
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
        let data = build(&Machine::default(), &campaign(), &samples, Some(42));
        let row = &data.algos[0].rows[0];
        assert_eq!(row.run_samples, 1);
        assert_eq!(row.run_min_ms, "2000.0");
    }

    #[test]
    fn build_and_run_phases_land_in_separate_columns() {
        let samples = vec![
            sample("c-gcc", FpMode::Strict, Phase::Build, false, 800_000_000),
            sample("c-gcc", FpMode::Strict, Phase::Run, false, 2_000_000_000),
        ];
        let data = build(&Machine::default(), &campaign(), &samples, Some(42));
        let row = &data.algos[0].rows[0];
        assert_eq!(row.build_min_ms, "800.0");
        assert_eq!(row.run_min_ms, "2000.0");
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
        let data = build(&Machine::default(), &campaign(), &samples, Some(42));
        assert_eq!(data.algos[0].rows[0].build_min_ms, "n/a");
    }

    #[test]
    fn a_relaxed_mode_reports_its_distance_from_the_strict_reference() {
        let mut divergent = sample("c-gcc", FpMode::Fast, Phase::Run, false, 1_000_000);
        divergent.checksum = Some(40);
        let data = build(&Machine::default(), &campaign(), &[divergent], Some(42));
        assert_eq!(data.algos[0].rows[0].checksum_delta, "-2");
    }

    #[test]
    fn dispersion_below_three_samples_is_not_reported_as_zero() {
        // The lower median of `[0, d]` is `0`, so a two-sample MAD is
        // structurally zero and would claim a precision we never measured.
        let samples = vec![
            sample("c-gcc", FpMode::Strict, Phase::Run, false, 1_000_000),
            sample("c-gcc", FpMode::Strict, Phase::Run, false, 5_000_000),
        ];
        let data = build(&Machine::default(), &campaign(), &samples, Some(42));
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
        let data = build(&Machine::default(), &campaign(), &samples, Some(42));
        assert_eq!(data.algos[0].rows[0].checksum_delta, "0");
    }

    #[test]
    fn rows_keep_the_schedule_order() {
        let samples = vec![
            sample("c-gcc", FpMode::Strict, Phase::Run, false, 1_000_000),
            sample("rust-llvm", FpMode::Strict, Phase::Run, false, 1_000_000),
            sample("c-gcc", FpMode::Strict, Phase::Run, false, 1_000_000),
        ];
        let data = build(&Machine::default(), &campaign(), &samples, Some(42));
        let names: Vec<_> = data.algos[0]
            .rows
            .iter()
            .map(|row| row.implementation.as_str())
            .collect();
        assert_eq!(names, ["c-gcc", "rust-llvm"]);
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
        let data = build(&Machine::default(), &campaign(), &samples, Some(42));
        let markdown = render(&data).unwrap();
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
        let markdown =
            render(&build(&Machine::default(), &campaign(), &samples, Some(42))).unwrap();

        let header = markdown
            .lines()
            .find(|line| line.starts_with("| Implementation |"))
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
                markdown.contains(&format!("**{column}**")),
                "column `{column}` has no entry in the column reference",
            );
        }
    }

    #[test]
    fn the_report_surfaces_the_hosts_warnings() {
        let data = build(&Machine::default(), &campaign(), &[], None);
        let markdown = render(&data).unwrap();
        assert!(markdown.contains("Linux"));
        assert!(markdown.contains("n/a"));
    }
}
