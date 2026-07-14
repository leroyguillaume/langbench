//! Two backends, head to head — and the one question a table of two rows does
//! not answer: *is the gap real?*
//!
//! A reader who puts two rows side by side is about to make a claim, and the
//! claim they want to make is a ratio: "gcc is 1.15× faster than clang here". The
//! campaign is entitled to that claim only when the gap survives its own noise.
//! A 3% gap between two rows that each wobble by 5% is not a result; it is the
//! same number, measured twice, on a machine that was busy.
//! See `site/src/content/methodology.md#sampling-and-what-may-be-concluded`.
//!
//! So the verdict is computed **here**, in the harness, and not in the browser
//! that displays it. Min-of-N, the dispersion, and what counts as a difference
//! are one definition of what this project measures; a second one written in
//! TypeScript would drift from this one the first time either was "fixed" — the
//! same reason [`crate::analysis`] exists at all. The site picks the two rows,
//! spells the numbers, and draws them. It decides nothing.
//!
//! Nothing here is a new measurement: every input is a number
//! [`crate::analysis`] already published, and the whole module is a pure function
//! of an [`Analysis`]. Recomputable, like every derived thing in this repository.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::analysis::{Aggregate, Analysis};
use crate::mode::Mode;
use crate::stats::Summary;

/// The pair a reader asked for. Two rows of one campaign, named the way the
/// samples name them: the backend slug and the FP mode, never a row index —
/// a position in a table is a property of the sort somebody clicked.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Selection {
    pub workload: String,
    pub left: Row,
    pub right: Row,
}

/// One side of the pair.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Row {
    pub backend: String,
    pub mode: Mode,
}

/// What a number is measured in. The site spells it; it never converts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
/// Every unit a metric can arrive in — and a **closed set on the wire**: the site
/// validates against it, and an unknown variant fails the parse rather than
/// degrading to an unformatted number. A new one lands with the renderer that can
/// spell it, never before.
pub enum Unit {
    Nanoseconds,
    Microseconds,
    Bytes,
}

/// Which side of a metric the campaign is entitled to call better — smaller
/// being better on every metric this project publishes, timings and bytes alike.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Left,
    Right,
    /// The gap is inside the noise the campaign carries on these two rows, so
    /// there is no gap to report. Not "equal": *indistinguishable*, which is a
    /// statement about this campaign and not about the backends.
    Tie,
    /// At least one of the two rows has no such number — an interpreted backend
    /// has no binary, and a quarantined one has no timing. An absence is a
    /// published fact, and it is never a zero.
    Unmeasured,
}

/// One row of the head-to-head: the same number, on both backends.
#[derive(Clone, Debug, Serialize)]
pub struct Metric {
    /// Stable across renderings, and what a stylesheet keys on: `run`, `compute`,
    /// `startup`, `cpu`, `build`, `binary`, `text`.
    pub key: String,
    pub label: String,
    pub unit: Unit,
    pub left: Option<u64>,
    pub right: Option<u64>,
    /// `right / left`. Below 1, the right-hand backend is the smaller one.
    ///
    /// A ratio, and never an absolute difference: two backends of one campaign on
    /// one architecture is exactly the comparison this project is allowed to publish, and
    /// the ratio is the part of it that travels. See `site/src/content/methodology.md#flags-and-the-architecture-baseline`.
    pub ratio: Option<f64>,
    /// The gap, as a percentage of the smaller of the two. Always positive.
    pub gap_pct: Option<f64>,
    /// The dispersion this pair carries: the worse of the two rows', and the bar
    /// [`Self::gap_pct`] has to clear before the gap is a result rather than
    /// weather. `None` for a metric with no dispersion — a binary is not sampled
    /// thirty times, it is one integer, and two of them differ or they do not.
    pub noise_pct: Option<f64>,
    pub verdict: Verdict,
}

/// Whether the two backends computed the same thing — the only claim that has to
/// hold before any timing beside it means anything.
///
/// Strings on the wire, because the checksum is a 64-bit integer and a JavaScript
/// `Number` is a double. Compared here, in Rust, on the full width. See
/// `site/src/content/methodology.md#the-strict-mode-invariant`.
#[derive(Clone, Debug, Serialize)]
pub struct Checksums {
    #[serde(serialize_with = "crate::analysis::as_string")]
    pub left: Option<u64>,
    #[serde(serialize_with = "crate::analysis::as_string")]
    pub right: Option<u64>,
    /// `None` when either side never reported one.
    pub same: Option<bool>,
    /// True whenever the two disagree. **In any mode, in either combination.**
    ///
    /// There is no relaxed mode left to excuse a divergence. This used to require
    /// both sides to be `strict`, because `fma` and `fast` were *expected* to
    /// diverge — that was what the mode bought, and holding them to the reference
    /// would have been a category error. `baseline` and `native` buy no such thing:
    /// they emit different instructions to compute identical bits, so two rows that
    /// disagree are two rows where one of them is wrong, whatever they were built
    /// for.
    ///
    /// The harness quarantines the backend over this, so a campaign it wrote can
    /// never contain one. It is computed anyway, for the file recorded by something
    /// else — and for the pair the checksum invariant is *most* interesting on: the
    /// two sides of an architecture crossing, which are obliged to agree bit for bit
    /// and have no shared silicon to agree by accident on.
    pub violates_checksum_invariant: bool,
}

/// Two rows, and what may be said about the pair.
#[derive(Clone, Debug, Serialize)]
pub struct Comparison {
    pub workload: String,
    pub left: Side,
    pub right: Side,
    pub metrics: Vec<Metric>,
    pub checksums: Checksums,
    /// The two rows come from two architectures, and **every timing below is
    /// therefore meaningless as a comparison**. It is computed here rather than
    /// left to the caller to notice: a renderer that forgot to check would publish
    /// exactly the claim `site/src/content/methodology.md#flags-and-the-architecture-baseline` exists to forbid. A ratio
    /// travels between architectures; a millisecond does not.
    ///
    /// The checksums, on the other hand, are *more* interesting across an architecture
    /// than within one: they are obliged to be bit-identical on x86-64 and on AArch64
    /// alike — in both modes, since neither relaxes the arithmetic — and a divergence
    /// is a bug in one of them.
    pub cross_architecture: bool,
}

/// A backend's identity, on one side of the pair. The manifest's own fields — the
/// sample carries them, so a comparison describes itself without joining against
/// anything.
#[derive(Clone, Debug, Serialize)]
pub struct Side {
    pub backend: String,
    pub backend_id: String,
    pub language: String,
    pub compiler: Option<String>,
    pub interpreter: Option<String>,
    pub mode: Mode,
    /// The architecture of the campaign this row was measured on — out of the machine record
    /// inside the file, never out of its name.
    pub architecture: String,
}

/// Below three samples the median absolute deviation is structurally zero — the
/// lower median of `[0, d]` is `0` — so a dispersion drawn from two rounds is not
/// a small dispersion, it is an unknown one. It buys the pair no tolerance.
const MINIMUM_SAMPLES_FOR_DISPERSION: usize = 3;

/// Compare two rows of one campaign.
///
/// Fails only on a selection the campaign cannot honour: a row it never measured,
/// or two rows from two workloads — whose timings are two different amounts of
/// work and whose checksums are two different reference values.
pub fn compare(analysis: &Analysis, selection: &Selection) -> Result<Comparison> {
    compare_across(analysis, analysis, selection)
}

/// Compare a row of one campaign with a row of another — including, deliberately,
/// two campaigns from two architectures.
///
/// The result then carries `cross_architecture`, and **that flag is the point**. The timings
/// it hands back are computed exactly as they would be within one campaign, because
/// refusing to compute them would only push somebody into doing the division by
/// hand; what the harness will not do is let the pair pass for a comparable one. A
/// millisecond on x86-64 and a millisecond on AArch64 are two different machines
/// answering two different questions, and no ratio of them means anything.
/// See `site/src/content/methodology.md#flags-and-the-architecture-baseline`.
pub fn compare_across(
    left_analysis: &Analysis,
    right_analysis: &Analysis,
    selection: &Selection,
) -> Result<Comparison> {
    let left_algo = algo_of(left_analysis, &selection.workload)?;
    let right_algo = algo_of(right_analysis, &selection.workload)?;

    let left = find(&left_algo.aggregates, &selection.left)?;
    let right = find(&right_algo.aggregates, &selection.right)?;

    Ok(Comparison {
        workload: left_algo.workload.clone(),
        cross_architecture: left_analysis.architecture != right_analysis.architecture,
        left: side(left, &left_analysis.architecture),
        right: side(right, &right_analysis.architecture),
        metrics: vec![
            timing(
                "run",
                "Run (external wall-clock)",
                left.run_wall,
                right.run_wall,
            ),
            timing(
                "compute",
                "Compute (the program's own clock)",
                left.run_elapsed,
                right.run_elapsed,
            ),
            timing(
                "startup",
                "Startup (container + runtime init)",
                left.run_startup,
                right.run_startup,
            ),
            // The median, like the table's CPU column: total CPU time is the work
            // the machine did, not the latency a reader waits for, and its
            // minimum would flatter whichever backend happened to get a quiet
            // core.
            cpu(left.run_cpu_usec, right.run_cpu_usec),
            // Deliberately absent: the core count. It is on every row of the table
            // and it explains half of what the rows above say — but this list ranks
            // two backends, and *busier is not better*. A kernel that saturates
            // eight cores has not beaten one that needed four, and a `Verdict` on
            // that number would be an opinion the campaign has not earned. The
            // table describes it; the head-to-head declines to score it.
            smallest(
                "memory",
                "Peak memory (the whole container)",
                Unit::Bytes,
                left.run_peak_bytes,
                right.run_peak_bytes,
            ),
            timing(
                "build",
                "Compile (the compiler's own clock)",
                left.build_elapsed,
                right.build_elapsed,
            ),
            exact(
                "binary",
                "Binary size",
                left.binary_bytes,
                right.binary_bytes,
            ),
            exact("text", ".text size", left.text_bytes, right.text_bytes),
            // The source is the *language's*, not the backend's: two rows that
            // compile the same file are the same number twice, and the comparison
            // says `tie` — which is the honest answer, and a useful one when the two
            // rows are `c-gcc` and `c-clang`.
            exact(
                "source",
                "Source size",
                left.source_bytes,
                right.source_bytes,
            ),
        ],
        checksums: checksums(left, right),
    })
}

fn algo_of<'a>(
    analysis: &'a Analysis,
    workload: &str,
) -> Result<&'a crate::analysis::WorkloadAnalysis> {
    analysis
        .workloads
        .iter()
        .find(|candidate| candidate.workload == workload)
        .ok_or_else(|| anyhow::anyhow!("this campaign has no workload {workload}"))
}

fn find<'a>(aggregates: &'a [Aggregate], row: &Row) -> Result<&'a Aggregate> {
    aggregates
        .iter()
        .find(|candidate| candidate.backend == row.backend && candidate.mode == row.mode)
        .ok_or_else(
            || anyhow::anyhow!("this campaign has no {} in {} mode", row.backend, row.mode,),
        )
}

fn side(aggregate: &Aggregate, architecture: &str) -> Side {
    Side {
        backend: aggregate.backend.clone(),
        backend_id: aggregate.backend_id.clone(),
        language: aggregate.language.clone(),
        compiler: aggregate.compiler.clone(),
        interpreter: aggregate.interpreter.clone(),
        mode: aggregate.mode,
        architecture: architecture.to_owned(),
    }
}

/// A metric drawn from the minimum of N — the statistic this project reports for
/// everything whose noise is one-sided, and the one the dispersion qualifies.
///
/// Timings and bytes of memory both qualify: a busy machine can only ever make a
/// run slower, or make it hold more pages. Neither can come out below what the
/// backend genuinely needed.
fn smallest(
    key: &str,
    label: &str,
    unit: Unit,
    left: Option<Summary>,
    right: Option<Summary>,
) -> Metric {
    metric(
        key,
        label,
        unit,
        left.map(|summary| summary.min),
        right.map(|summary| summary.min),
        noise(left, right),
    )
}

fn timing(key: &str, label: &str, left: Option<Summary>, right: Option<Summary>) -> Metric {
    smallest(key, label, Unit::Nanoseconds, left, right)
}

fn cpu(left: Option<Summary>, right: Option<Summary>) -> Metric {
    metric(
        "cpu",
        "CPU time (all cores)",
        Unit::Microseconds,
        left.map(|summary| summary.median),
        right.map(|summary| summary.median),
        noise(left, right),
    )
}

/// A metric that is not sampled: one integer, read off the image. No dispersion,
/// so no tolerance — two sizes differ or they are the same size.
fn exact(key: &str, label: &str, left: Option<u64>, right: Option<u64>) -> Metric {
    metric(key, label, Unit::Bytes, left, right, None)
}

fn metric(
    key: &str,
    label: &str,
    unit: Unit,
    left: Option<u64>,
    right: Option<u64>,
    noise_pct: Option<f64>,
) -> Metric {
    let (ratio, gap_pct, verdict) = match (left, right) {
        (Some(left), Some(right)) => {
            let (ratio, gap_pct) = if left == 0 {
                // A zero denominator is not a ratio, and a zero timing is not a
                // fast one. Say nothing rather than divide.
                (None, None)
            } else {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a timing in nanoseconds is far below the 2^53 a double holds exactly"
                )]
                let ratio = right as f64 / left as f64;
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "same: the operands are timings and byte counts, not checksums"
                )]
                let gap_pct = left.abs_diff(right) as f64 / left.min(right) as f64 * 100.0;
                (Some(ratio), Some(gap_pct))
            };
            (ratio, gap_pct, verdict(left, right, gap_pct, noise_pct))
        }
        // One of the two has no such number. There is no ratio to take, and the
        // absence is the result: an interpreted backend ships no binary.
        _ => (None, None, Verdict::Unmeasured),
    };

    Metric {
        key: key.to_owned(),
        label: label.to_owned(),
        unit,
        left,
        right,
        ratio,
        gap_pct,
        noise_pct,
        verdict,
    }
}

/// The bar the gap has to clear.
///
/// The *worse* of the two dispersions, because a claim about the pair is only as
/// defensible as its shakier half. A row the campaign drew fewer than three
/// samples for has no known dispersion, and an unknown dispersion is not a zero
/// one — but it cannot widen the bar either, so it simply does not lower it.
fn noise(left: Option<Summary>, right: Option<Summary>) -> Option<f64> {
    let known = |summary: Option<Summary>| {
        summary
            .filter(|summary| summary.n >= MINIMUM_SAMPLES_FOR_DISPERSION)
            .map(|summary| summary.mad_pct)
    };
    match (known(left), known(right)) {
        (None, None) => None,
        (left, right) => Some(left.unwrap_or(0.0).max(right.unwrap_or(0.0))),
    }
}

/// Who won, if anybody did.
///
/// Smaller is better on every metric this project publishes. A gap that does not
/// clear the noise the campaign carries is not a win for the row that happens to
/// hold the smaller number: it is a tie, and saying so is the whole point of
/// putting the two rows side by side.
fn verdict(left: u64, right: u64, gap_pct: Option<f64>, noise_pct: Option<f64>) -> Verdict {
    if left == right {
        return Verdict::Tie;
    }
    if let (Some(gap_pct), Some(noise_pct)) = (gap_pct, noise_pct)
        && gap_pct <= noise_pct
    {
        return Verdict::Tie;
    }
    if left < right {
        Verdict::Left
    } else {
        Verdict::Right
    }
}

fn checksums(left: &Aggregate, right: &Aggregate) -> Checksums {
    let same = match (left.checksum, right.checksum) {
        (Some(left), Some(right)) => Some(left == right),
        _ => None,
    };
    Checksums {
        left: left.checksum,
        right: right.checksum,
        same,
        violates_checksum_invariant: same == Some(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{Options, analyze};
    use crate::machine::Machine;
    use crate::sample::{Campaign, Phase, Recording, Sample};
    use crate::workload::Workload;

    fn campaign() -> Campaign {
        Campaign {
            langbench_version: "0.1.0".to_owned(),
            timestamp: "2026-07-11T12:00:00Z".to_owned(),
            cpu: 8,
            workload: Workload::fixture(),
            rounds: 30,
            build_rounds: 5,
            warmup_rounds: 2,
            march: "x86-64-v3".to_owned(),
            modes: vec!["baseline".to_owned()],
        }
    }

    fn sample(backend: &str, mode: Mode, wall: u64, checksum: u64) -> Sample {
        let (language, compiler) = backend.split_once('-').expect("<language>-<compiler>");
        Sample {
            workload: "mandelbrot".to_owned(),
            language: language.to_owned(),
            compiler: Some(compiler.to_owned()),
            interpreter: None,
            description: format!("{backend}, as the fixture declares it"),
            comments: None,
            mode,
            isa: Some("x86-64-v3".to_owned()),
            phase: Phase::Run,
            round: 0,
            warmup: false,
            cpu: 8,
            wall_ns: wall,
            elapsed_ns: wall / 2,
            user_usec: wall / 1_000,
            system_usec: 0,
            peak_bytes: Some(12_582_912),
            source_bytes: Some(2_048),
            checksum: Some(checksum),
            binary_bytes: Some(2048),
            binary_stripped_bytes: None,
            text_bytes: Some(1024),
        }
    }

    /// One `(backend, mode)`, sampled `walls.len()` times.
    fn rows(backend: &str, mode: Mode, walls: &[u64], checksum: u64) -> Vec<Sample> {
        walls
            .iter()
            .map(|wall| sample(backend, mode, *wall, checksum))
            .collect()
    }

    fn analysis(samples: Vec<Sample>) -> Analysis {
        on_arch("x86_64", samples)
    }

    /// A campaign, and the machine it says it ran on. The architecture is read out of that
    /// record — never out of a filename, which is a label somebody typed.
    fn on_arch(architecture: &str, samples: Vec<Sample>) -> Analysis {
        let machine = Machine {
            architecture: architecture.to_owned(),
            ..Machine::default()
        };
        analyze(
            &Recording {
                machine,
                campaign: campaign(),
                samples,
                failures: Vec::new(),
            },
            Options::default(),
        )
    }

    fn selection(left: &str, right: &str) -> Selection {
        Selection {
            workload: "mandelbrot".to_owned(),
            left: Row {
                backend: left.to_owned(),
                mode: Mode::Baseline,
            },
            right: Row {
                backend: right.to_owned(),
                mode: Mode::Baseline,
            },
        }
    }

    fn metric<'a>(comparison: &'a Comparison, key: &str) -> &'a Metric {
        comparison
            .metrics
            .iter()
            .find(|metric| metric.key == key)
            .expect("the comparison publishes this metric")
    }

    /// Three clean samples on each side, and a gap far outside the noise.
    fn quiet() -> Analysis {
        let mut samples = rows(
            "c-gcc",
            Mode::Baseline,
            &[2_000_000_000, 2_000_000_000, 2_010_000_000],
            42,
        );
        samples.extend(rows(
            "c-clang",
            Mode::Baseline,
            &[3_000_000_000, 3_000_000_000, 3_010_000_000],
            42,
        ));
        analysis(samples)
    }

    #[test]
    fn the_ratio_is_the_right_hand_row_over_the_left_hand_one() {
        let comparison = compare(&quiet(), &selection("c-gcc", "c-clang")).unwrap();
        let run = metric(&comparison, "run");

        assert_eq!(run.left, Some(2_000_000_000));
        assert_eq!(run.right, Some(3_000_000_000));
        assert!((run.ratio.unwrap() - 1.5).abs() < 1e-9);
        assert!((run.gap_pct.unwrap() - 50.0).abs() < 1e-9);
        assert_eq!(run.verdict, Verdict::Left);
    }

    /// Swapping the two rows swaps the verdict and inverts the ratio, and changes
    /// nothing else. A comparison is a property of the pair, not of the order the
    /// reader picked them in.
    #[test]
    fn the_comparison_is_symmetric() {
        let analysis = quiet();
        let forward = compare(&analysis, &selection("c-gcc", "c-clang")).unwrap();
        let backward = compare(&analysis, &selection("c-clang", "c-gcc")).unwrap();

        assert_eq!(metric(&forward, "run").verdict, Verdict::Left);
        assert_eq!(metric(&backward, "run").verdict, Verdict::Right);

        let forward_ratio = metric(&forward, "run").ratio.unwrap();
        let backward_ratio = metric(&backward, "run").ratio.unwrap();
        assert!((forward_ratio * backward_ratio - 1.0).abs() < 1e-9);
        assert_eq!(
            metric(&forward, "run").gap_pct,
            metric(&backward, "run").gap_pct,
        );
    }

    /// The reason this module is in Rust and not in the browser. A 3% gap between
    /// two rows that each wobble by more than that is not a result: it is the same
    /// number, measured twice, on a machine that was busy.
    #[test]
    fn a_gap_smaller_than_the_dispersion_is_a_tie_not_a_win() {
        let mut samples = rows(
            "c-gcc",
            Mode::Baseline,
            &[1_000_000_000, 1_100_000_000, 1_200_000_000],
            42,
        );
        samples.extend(rows(
            "c-clang",
            Mode::Baseline,
            &[1_030_000_000, 1_130_000_000, 1_230_000_000],
            42,
        ));
        let comparison = compare(&analysis(samples), &selection("c-gcc", "c-clang")).unwrap();
        let run = metric(&comparison, "run");

        // gcc's minimum is genuinely the smaller one -- and the campaign is in no
        // position to say so: a 3% gap under a ~9% dispersion.
        assert!(run.left.unwrap() < run.right.unwrap());
        assert!(run.gap_pct.unwrap() < run.noise_pct.unwrap());
        assert_eq!(run.verdict, Verdict::Tie);
    }

    /// A dispersion drawn from two rounds is structurally zero, and a structural
    /// zero is not a quiet machine. It buys the pair no tolerance -- the gap is
    /// reported for what it is, with nothing to hide behind.
    #[test]
    fn a_dispersion_below_three_samples_is_unknown_never_zero() {
        let mut samples = rows("c-gcc", Mode::Baseline, &[1_000_000_000, 1_500_000_000], 42);
        samples.extend(rows(
            "c-clang",
            Mode::Baseline,
            &[1_010_000_000, 1_510_000_000],
            42,
        ));
        let comparison = compare(&analysis(samples), &selection("c-gcc", "c-clang")).unwrap();
        let run = metric(&comparison, "run");

        assert_eq!(run.noise_pct, None);
        assert_eq!(run.verdict, Verdict::Left);
    }

    /// A backend that ships no binary is not a backend with a zero-byte one.
    #[test]
    fn an_absent_number_is_never_a_zero_and_never_a_winner() {
        let mut samples = rows("c-gcc", Mode::Baseline, &[2_000_000_000], 42);
        let mut interpreted = rows("python-cpython", Mode::Baseline, &[9_000_000_000], 42);
        for sample in &mut interpreted {
            sample.binary_bytes = None;
            sample.text_bytes = None;
        }
        samples.append(&mut interpreted);

        let comparison = compare(
            &analysis(samples),
            &Selection {
                workload: "mandelbrot".to_owned(),
                left: Row {
                    backend: "c-gcc".to_owned(),
                    mode: Mode::Baseline,
                },
                right: Row {
                    backend: "python-cpython".to_owned(),
                    mode: Mode::Baseline,
                },
            },
        )
        .unwrap();

        let binary = metric(&comparison, "binary");
        assert_eq!(binary.left, Some(2048));
        assert_eq!(binary.right, None);
        assert_eq!(binary.ratio, None);
        assert_eq!(binary.verdict, Verdict::Unmeasured);
    }

    /// A relaxed mode is *expected* to diverge -- that is what it buys. The
    /// comparison says the two backends did not compute the same thing, and does
    /// not call it a violation.
    #[test]
    fn a_native_row_that_diverges_from_its_baseline_violates_the_invariant() {
        let mut samples = rows("c-gcc", Mode::Baseline, &[2_000_000_000], 1_000);
        samples.extend(rows("c-gcc", Mode::Native, &[1_000_000_000], 994));

        let comparison = compare(
            &analysis(samples),
            &Selection {
                workload: "mandelbrot".to_owned(),
                left: Row {
                    backend: "c-gcc".to_owned(),
                    mode: Mode::Baseline,
                },
                right: Row {
                    backend: "c-gcc".to_owned(),
                    mode: Mode::Native,
                },
            },
        )
        .unwrap();

        // Under the floating-point axis this pair was *fine*: a relaxed mode was
        // licensed to compute a different number, and the difference was the column.
        // Under the ISA axis it is a bug. `native` emits wider instructions to
        // compute identical bits — it reorders no arithmetic and rounds nothing
        // differently — so a native row that disagrees with its own baseline is a
        // miscompilation, and the harness takes the backend out of the campaign
        // rather than printing the delta and letting the reader shrug.
        assert_eq!(comparison.checksums.same, Some(false));
        assert!(comparison.checksums.violates_checksum_invariant);
    }

    /// Two rows that disagree are the one thing this project treats as a
    /// bug rather than a rounding excuse. The campaign aborts over it, so a file
    /// carrying one was not written by this harness -- and the comparison says so
    /// instead of ranking their timings.
    #[test]
    fn two_strict_rows_that_disagree_violate_the_invariant() {
        let mut samples = rows("c-gcc", Mode::Baseline, &[2_000_000_000], 1_000);
        samples.extend(rows("c-clang", Mode::Baseline, &[2_000_000_000], 1_001));

        let comparison = compare(&analysis(samples), &selection("c-gcc", "c-clang")).unwrap();
        assert_eq!(comparison.checksums.same, Some(false));
        assert!(comparison.checksums.violates_checksum_invariant);
    }

    /// A 64-bit checksum leaves as a string, here as everywhere: `JSON.parse`
    /// rounds every integer past 2^53, and the correctness gate of this harness
    /// does not lose its low bits on the way to a web page.
    #[test]
    fn a_checksum_crosses_the_wire_as_a_string() {
        let mut samples = rows(
            "c-gcc",
            Mode::Baseline,
            &[2_000_000_000],
            9_007_199_254_740_993,
        );
        samples.extend(rows(
            "c-clang",
            Mode::Baseline,
            &[2_000_000_000],
            9_007_199_254_740_993,
        ));

        let comparison = compare(&analysis(samples), &selection("c-gcc", "c-clang")).unwrap();
        let json = serde_json::to_string(&comparison).unwrap();

        assert!(json.contains(r#""left":"9007199254740993""#), "{json}");
        assert!(!json.contains("9007199254740993,"), "{json}");
    }

    #[test]
    fn a_row_the_campaign_never_measured_is_refused_never_invented() {
        let error = compare(&quiet(), &selection("c-gcc", "fortran-gfortran")).unwrap_err();
        assert!(error.to_string().contains("fortran-gfortran"), "{error:#}",);
    }

    /// Two workloads are two different amounts of work and two different
    /// reference checksums. A ratio across them would be a number about nothing.
    #[test]
    fn an_algorithm_the_campaign_never_ran_is_refused() {
        let error = compare(
            &quiet(),
            &Selection {
                workload: "nbody".to_owned(),
                left: Row {
                    backend: "c-gcc".to_owned(),
                    mode: Mode::Baseline,
                },
                right: Row {
                    backend: "c-clang".to_owned(),
                    mode: Mode::Baseline,
                },
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("nbody"), "{error:#}");
    }

    /// Two architectures, deliberately. The pair is computed — refusing would only
    /// send somebody off to divide the two numbers by hand — but it is *flagged*,
    /// because a millisecond does not cross an architecture and a renderer that forgot to say
    /// so would publish the one claim `site/src/content/methodology.md#flags-and-the-architecture-baseline` forbids.
    #[test]
    fn a_pair_drawn_from_two_architectures_says_so() {
        let x86 = on_arch(
            "x86_64",
            rows("c-gcc", Mode::Baseline, &[1_000_000_000; 5], 42),
        );
        let arm = on_arch(
            "aarch64",
            rows("rust-rustc", Mode::Baseline, &[1_500_000_000; 5], 42),
        );

        let across = compare_across(&x86, &arm, &selection("c-gcc", "rust-rustc")).unwrap();
        assert!(across.cross_architecture);
        assert_eq!(across.left.architecture, "x86_64");
        assert_eq!(across.right.architecture, "aarch64");

        // The timings are still computed. The flag is what makes them honest.
        let run = across
            .metrics
            .iter()
            .find(|metric| metric.key == "run")
            .unwrap();
        assert_eq!(run.left, Some(1_000_000_000));
        assert_eq!(run.right, Some(1_500_000_000));

        // And the one thing that *is* obliged to survive the crossing: in `strict`
        // the checksum is bit-identical on every architecture. That is the whole invariant.
        assert_eq!(across.checksums.same, Some(true));
        assert!(!across.checksums.violates_checksum_invariant);
    }

    #[test]
    fn two_rows_of_one_campaign_are_not_a_cross_architecture_pair() {
        let campaign = analysis(
            [
                rows("c-gcc", Mode::Baseline, &[1_000_000_000; 5], 42),
                rows("rust-rustc", Mode::Baseline, &[1_100_000_000; 5], 42),
            ]
            .concat(),
        );
        let same = compare(&campaign, &selection("c-gcc", "rust-rustc")).unwrap();
        assert!(!same.cross_architecture);
        assert_eq!(same.left.architecture, "x86_64");
    }

    /// Two `strict` rows on two architectures whose checksums disagree: the invariant broken,
    /// and it is a bug in one of them rather than a rounding excuse.
    #[test]
    fn a_strict_checksum_that_does_not_survive_the_crossing_is_a_violation() {
        let x86 = on_arch(
            "x86_64",
            rows("c-gcc", Mode::Baseline, &[1_000_000_000; 5], 42),
        );
        let arm = on_arch(
            "aarch64",
            rows("c-gcc", Mode::Baseline, &[1_000_000_000; 5], 43),
        );
        let across = compare_across(&x86, &arm, &selection("c-gcc", "c-gcc")).unwrap();
        assert!(across.cross_architecture);
        assert!(across.checksums.violates_checksum_invariant);
    }
}
