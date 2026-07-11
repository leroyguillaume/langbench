//! The numbers, before anybody formats them.
//!
//! Bucketing samples into `(algo, backend, mode)` and summarizing each bucket is
//! the one piece of arithmetic in this repository that has more than one
//! consumer: `langbench md` formats it into a table, and the WebAssembly build
//! hands it to a browser that re-sorts and re-plots it. Both call [`analyze`].
//!
//! That is deliberate, and it is the same rule as `bench.schema.json`: a second
//! implementation of min-of-N in TypeScript would be a second definition of what
//! this project measures, and the two would drift the first time one of them was
//! "fixed". The website is a rendering of the samples — it does not get its own
//! statistics. See `METHODOLOGY.md#why-min-of-n-not-the-median`.
//!
//! Everything here is derived and can be recomputed from `samples.ndjson`.
//! Aggregates never replace the samples.

use std::collections::HashMap;

use serde::{Serialize, Serializer};

use crate::machine::Field;
use crate::mode::FpMode;
use crate::sample::{Campaign, Phase, Recording, Sample, Stage};
use crate::stats::{Summary, summarize};

/// What the caller is allowed to vary about an analysis.
///
/// Small on purpose. A knob here is a knob the website exposes, and every one of
/// them is a way to publish a different number from the same file — so a reader
/// has to be able to see which one produced the table in front of them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Options {
    /// Warmup samples are always *recorded*; this decides whether they are
    /// *aggregated*. The report never includes them, and neither does the site
    /// by default — the toggle exists so a reader can see the cold-start cost
    /// the campaign paid, not so they can quietly inflate a number.
    pub include_warmup: bool,
}

/// One campaign, bucketed and summarized. The input of every rendering.
///
/// Serialized in `snake_case`, like every other field this project puts on a
/// wire — `samples.ndjson` spells the campaign header that way, and one
/// vocabulary that spans the file, the CSV and the website beats two that need a
/// translation table between them.
#[derive(Debug, Serialize)]
pub struct Analysis {
    pub campaign: Campaign,
    pub options: Options,
    /// The ISA the campaign ran on, lifted out of the machine so a consumer can
    /// key on it without scraping a label out of [`Self::machine_fields`].
    ///
    /// The website needs it for one reason: **an absolute timing never crosses an
    /// ISA**. Two campaigns from two architectures are two experiments, and the
    /// site has to be able to tell them apart structurally — from the header the
    /// campaign recorded, never from the name of the file it was served under. A
    /// filename is a label somebody types; this is what the machine said about
    /// itself. See `METHODOLOGY.md#the-isa-rule`.
    pub arch: String,
    pub hostname: Option<String>,
    pub machine_fields: Vec<Field>,
    /// Every reason this host was a poor benchmark target. It travels with the
    /// numbers, so a chart cannot be read without the caveat that qualifies it.
    pub warnings: Vec<String>,
    pub algos: Vec<AlgoAnalysis>,
    /// Every backend the campaign measured, described once. The aggregates
    /// repeat a backend per FP mode; its identity card does not.
    pub backends: Vec<Backend>,
    /// Every backend the campaign *lost*, and what it lost it to.
    ///
    /// It travels with the numbers for the same reason [`Self::warnings`] does: a
    /// table cannot be read without knowing what is not in it. A quarantined
    /// backend has no row, and a missing row is indistinguishable from a backend
    /// nobody ever wrote — which is precisely the wrong thing for a reader to
    /// conclude about a compiler that segfaulted.
    pub failures: Vec<Failure>,
}

/// One quarantined `(backend, mode)`, as the renderers need it.
///
/// [`crate::sample::Failure`] with the derived slugs added — the same enrichment
/// [`Aggregate`] gets, and for the same reason: a Liquid template and a browser
/// cannot call `backend_slug`.
#[derive(Clone, Debug, Serialize)]
pub struct Failure {
    pub algo: String,
    pub backend: String,
    pub backend_id: String,
    pub language: String,
    pub compiler: Option<String>,
    pub interpreter: Option<String>,
    pub description: String,
    pub comments: Option<String>,
    pub mode: FpMode,
    /// `prepare` — the image never built — or `measure`: it built, and then the
    /// run did not produce a valid record.
    pub stage: Stage,
    pub phase: Option<Phase>,
    pub round: Option<u32>,
    pub error: String,
}

#[derive(Debug, Serialize)]
pub struct AlgoAnalysis {
    pub algo: String,
    /// The value every strict-mode run of *this* algorithm agreed on. A property
    /// of `(algo, grid size, max_iter)`, never of the campaign.
    ///
    /// A string on the wire: it is a 64-bit integer, and a JSON number wider than
    /// 2^53 is rounded by every JavaScript parser that reads it. The correctness
    /// gate of the whole harness does not travel as a float.
    /// See `METHODOLOGY.md#the-strict-mode-invariant`.
    #[serde(serialize_with = "as_string")]
    pub strict_checksum: Option<u64>,
    /// Fastest first, on the minimum wall-clock — the statistic the report
    /// headlines. Sorted here rather than in each renderer, so two renderings of
    /// one campaign cannot disagree about which backend won.
    pub aggregates: Vec<Aggregate>,
}

/// A backend's identity card, as its manifest declared it. Straight from the
/// `bench.yaml` the samples carry.
#[derive(Clone, Debug, Serialize)]
pub struct Backend {
    /// `mandelbrot-python-cython-cpython`. The anchor of this backend's section
    /// in the Markdown report, and the row key on the website.
    pub id: String,
    pub algo: String,
    pub backend: String,
    pub language: String,
    pub compiler: Option<String>,
    pub interpreter: Option<String>,
    pub description: String,
    pub comments: Option<String>,
}

/// Everything the campaign learned about one `(algo, backend, mode)`.
///
/// Numbers, in their native units — nanoseconds, microseconds, bytes. Not one
/// formatted string: `"2000.0 ms"` cannot be sorted, plotted, or divided by
/// another backend's time, and a renderer that receives it has already lost the
/// only thing it needed.
#[derive(Debug, Serialize)]
pub struct Aggregate {
    pub algo: String,
    pub backend: String,
    /// This backend's entry in [`Analysis::backends`].
    pub backend_id: String,
    pub language: String,
    /// `None` for a backend that compiles nothing ahead of the run. An absence is
    /// a published fact about the backend, not a hole in the data.
    pub compiler: Option<String>,
    /// `None` for a backend that ships machine code and no runtime.
    pub interpreter: Option<String>,
    pub mode: FpMode,
    /// External wall-clock of the run: container create + runtime init + compute.
    /// The headline of the run column.
    pub run_wall: Option<Summary>,
    /// The program's own clock: compute alone.
    pub run_elapsed: Option<Summary>,
    /// `wall - elapsed`, per sample. Never the difference of two minima drawn
    /// from different rounds — that would describe a run that never happened.
    pub run_startup: Option<Summary>,
    pub run_cpu_usec: Option<Summary>,
    /// The compiler's own elapsed time, from inside the container — never the
    /// `docker build` wall-clock. See
    /// `METHODOLOGY.md#the-build-column-reports-the-internal-clock-the-run-column-the-external-one`.
    pub build_elapsed: Option<Summary>,
    pub binary_bytes: Option<u64>,
    pub binary_stripped_bytes: Option<u64>,
    pub text_bytes: Option<u64>,
    /// A 64-bit integer, on the wire as a string. See
    /// [`AlgoAnalysis::strict_checksum`].
    #[serde(serialize_with = "as_string")]
    pub checksum: Option<u64>,
    /// This mode's distance from the strict reference: the precision sold for the
    /// speed gained. `i128`, because two `u64` checksums can differ by more than
    /// an `i64` holds — and a string on the wire for the same reason as the
    /// checksum itself.
    #[serde(serialize_with = "as_string")]
    pub checksum_delta: Option<i128>,
}

/// Samples accumulated for one `(algo, backend, mode)`, before summarizing.
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
    build_elapsed: Vec<u64>,
    checksum: Option<u64>,
    binary_bytes: Option<u64>,
    binary_stripped_bytes: Option<u64>,
    text_bytes: Option<u64>,
}

pub fn analyze(recording: &Recording, options: Options) -> Analysis {
    let samples = &recording.samples;
    let references = strict_references(samples);

    // Insertion order comes from the first round, which is the schedule order.
    let mut order: Vec<(String, String, FpMode)> = Vec::new();
    let mut buckets: HashMap<(String, String, FpMode), Bucket> = HashMap::new();

    for sample in samples {
        let key = (sample.algo.clone(), sample.backend(), sample.mode);
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

        // Constants of the image: take them wherever they first appear, warmup
        // round included. A checksum is not a measurement.
        bucket.checksum = bucket.checksum.or(sample.checksum);
        bucket.binary_bytes = bucket.binary_bytes.or(sample.binary_bytes);
        bucket.binary_stripped_bytes = bucket
            .binary_stripped_bytes
            .or(sample.binary_stripped_bytes);
        bucket.text_bytes = bucket.text_bytes.or(sample.text_bytes);

        if sample.warmup && !options.include_warmup {
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

    let mut algos: Vec<AlgoAnalysis> = Vec::new();
    let mut backends: Vec<Backend> = Vec::new();
    for key in &order {
        let (algo, backend, mode) = key;
        let bucket = &buckets[key];
        let reference = references.get(algo).copied();
        let aggregate = Aggregate {
            algo: algo.clone(),
            backend: backend.clone(),
            backend_id: backend_id(algo, backend),
            language: bucket.language.clone(),
            compiler: bucket.compiler.clone(),
            interpreter: bucket.interpreter.clone(),
            mode: *mode,
            run_wall: summarize(&bucket.run_wall),
            run_elapsed: summarize(&bucket.run_elapsed),
            run_startup: summarize(&bucket.run_startup),
            run_cpu_usec: summarize(&bucket.run_cpu_usec),
            build_elapsed: summarize(&bucket.build_elapsed),
            binary_bytes: bucket.binary_bytes,
            binary_stripped_bytes: bucket.binary_stripped_bytes,
            text_bytes: bucket.text_bytes,
            checksum: bucket.checksum,
            checksum_delta: match (bucket.checksum, reference) {
                (Some(checksum), Some(reference)) => {
                    Some(i128::from(checksum) - i128::from(reference))
                }
                _ => None,
            },
        };

        // One card per backend, not one per (backend, mode): the three modes are
        // three experiments on the same thing, and the thing is what a card
        // describes.
        let id = backend_id(algo, backend);
        if !backends.iter().any(|known| known.id == id) {
            backends.push(Backend {
                id,
                algo: algo.clone(),
                backend: backend.clone(),
                language: bucket.language.clone(),
                compiler: bucket.compiler.clone(),
                interpreter: bucket.interpreter.clone(),
                description: bucket.description.clone(),
                comments: bucket.comments.clone(),
            });
        }

        match algos.iter_mut().find(|analysis| &analysis.algo == algo) {
            Some(analysis) => analysis.aggregates.push(aggregate),
            None => algos.push(AlgoAnalysis {
                algo: algo.clone(),
                strict_checksum: reference,
                aggregates: vec![aggregate],
            }),
        }
    }

    // Fastest first, on the same statistic the table headlines: the minimum
    // wall-clock. `sort_by_key` is stable, so rows the campaign could not measure
    // keep their schedule order at the bottom instead of being shuffled among
    // themselves.
    for analysis in &mut algos {
        analysis.aggregates.sort_by_key(|aggregate| {
            let min = aggregate.run_wall.map(|summary| summary.min);
            (min.is_none(), min)
        });
    }

    Analysis {
        campaign: recording.campaign.clone(),
        options,
        arch: recording.machine.arch.clone(),
        hostname: recording.machine.hostname.clone(),
        machine_fields: recording.machine.fields(),
        warnings: recording.machine.warnings(),
        algos,
        backends,
        failures: recording.failures.iter().map(failure).collect(),
    }
}

/// A recorded failure, with the slugs a renderer cannot derive for itself.
fn failure(failure: &crate::sample::Failure) -> Failure {
    let backend = failure.backend();
    Failure {
        backend_id: backend_id(&failure.algo, &backend),
        algo: failure.algo.clone(),
        backend,
        language: failure.language.clone(),
        compiler: failure.compiler.clone(),
        interpreter: failure.interpreter.clone(),
        description: failure.description.clone(),
        comments: failure.comments.clone(),
        mode: failure.mode,
        stage: failure.stage,
        phase: failure.phase,
        round: failure.round,
        error: failure.error.clone(),
    }
}

/// The anchor of a backend's section, and the heading it is generated from.
///
/// Markdown renderers derive an anchor from the heading text, and they do not all
/// derive it the same way. So the heading *is* the anchor: lowercase, no spaces,
/// no punctuation — nothing for a renderer to reinterpret.
pub fn backend_id(algo: &str, backend: &str) -> String {
    format!("{algo}-{backend}")
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

/// Serialize a wide integer as a JSON string.
///
/// `JSON.parse` in a browser produces an IEEE 754 double, which silently rounds
/// every integer past 2^53. The checksum is the correctness gate of this
/// harness; it does not lose its low bits on the way to a web page. The site
/// treats it as an opaque token — it displays it and compares it, it never does
/// arithmetic on it.
fn as_string<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    T: std::fmt::Display,
    S: Serializer,
{
    match value {
        Some(value) => serializer.serialize_str(&value.to_string()),
        None => serializer.serialize_none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::Machine;
    use crate::sample::Recording;

    fn recording(samples: Vec<Sample>) -> Recording {
        Recording {
            machine: Machine::default(),
            campaign: campaign(),
            samples,
            failures: Vec::new(),
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

    fn sample(backend: &str, mode: FpMode, warmup: bool, wall: u64, checksum: u64) -> Sample {
        let (language, compiler) = backend.split_once('-').expect("<language>-<compiler>");
        Sample {
            algo: "mandelbrot".to_owned(),
            language: language.to_owned(),
            compiler: Some(compiler.to_owned()),
            interpreter: None,
            description: format!("{backend}, as the fixture declares it"),
            comments: None,
            mode,
            phase: Phase::Run,
            round: 0,
            warmup,
            cpu: 8,
            wall_ns: wall,
            elapsed_ns: wall / 2,
            user_usec: 1_000,
            system_usec: 0,
            checksum: Some(checksum),
            binary_bytes: Some(2048),
            binary_stripped_bytes: None,
            text_bytes: Some(1024),
        }
    }

    #[test]
    fn aggregates_are_sorted_fastest_first() {
        let samples = vec![
            sample("c-clang", FpMode::Strict, false, 3_000_000_000, 42),
            sample("c-gcc", FpMode::Strict, false, 2_000_000_000, 42),
        ];
        let analysis = analyze(&recording(samples), Options::default());
        let backends: Vec<&str> = analysis.algos[0]
            .aggregates
            .iter()
            .map(|aggregate| aggregate.backend.as_str())
            .collect();
        assert_eq!(backends, ["c-gcc", "c-clang"]);
    }

    #[test]
    fn warmup_samples_are_excluded_by_default_and_included_on_request() {
        let samples = vec![
            sample("c-gcc", FpMode::Strict, true, 9_000_000_000, 42),
            sample("c-gcc", FpMode::Strict, false, 2_000_000_000, 42),
        ];

        let cold = analyze(&recording(samples.clone()), Options::default());
        let run = cold.algos[0].aggregates[0].run_wall.unwrap();
        assert_eq!(run.n, 1);
        assert_eq!(run.min, 2_000_000_000);

        let warm = analyze(
            &recording(samples),
            Options {
                include_warmup: true,
            },
        );
        let run = warm.algos[0].aggregates[0].run_wall.unwrap();
        assert_eq!(run.n, 2);
        assert_eq!(run.min, 2_000_000_000);
    }

    /// A checksum is a property of the image, not a measurement: it is read off
    /// whichever sample carries it first, warmup included, so a bucket whose only
    /// measured rounds were discarded still knows what it computed.
    #[test]
    fn the_checksum_is_read_from_a_warmup_sample_too() {
        let samples = vec![sample("c-gcc", FpMode::Strict, true, 9_000_000_000, 7)];
        let analysis = analyze(&recording(samples), Options::default());
        let aggregate = &analysis.algos[0].aggregates[0];
        assert_eq!(aggregate.checksum, Some(7));
        assert!(aggregate.run_wall.is_none());
    }

    #[test]
    fn a_relaxed_mode_is_scored_against_the_strict_reference_of_its_own_algorithm() {
        let samples = vec![
            sample("c-gcc", FpMode::Strict, false, 2_000_000_000, 1_000),
            sample("c-gcc", FpMode::Fast, false, 1_000_000_000, 994),
        ];
        let analysis = analyze(&recording(samples), Options::default());
        assert_eq!(analysis.algos[0].strict_checksum, Some(1_000));

        let fast = analysis.algos[0]
            .aggregates
            .iter()
            .find(|aggregate| aggregate.mode == FpMode::Fast)
            .unwrap();
        assert_eq!(fast.checksum_delta, Some(-6));
    }

    /// The ISA is read off the machine the campaign recorded, never off a
    /// filename. Two campaigns from two architectures are two experiments, and
    /// the consumer that has to keep their absolute timings apart cannot be asked
    /// to trust what somebody called the file.
    #[test]
    fn the_analysis_carries_the_isa_the_campaign_ran_on() {
        let mut recording = recording(vec![sample(
            "c-gcc",
            FpMode::Strict,
            false,
            2_000_000_000,
            42,
        )]);
        recording.machine.arch = "x86_64".to_owned();
        recording.machine.hostname = Some("bench-01".to_owned());

        let analysis = analyze(&recording, Options::default());
        assert_eq!(analysis.arch, "x86_64");
        assert_eq!(analysis.hostname.as_deref(), Some("bench-01"));
    }

    /// The two clocks, and the gap between them, all come off the *same* sample.
    #[test]
    fn startup_is_summarized_per_sample_never_as_a_difference_of_minima() {
        let mut fast_start = sample("c-gcc", FpMode::Strict, false, 2_000_000_000, 42);
        fast_start.elapsed_ns = 1_900_000_000; // 100 ms of startup
        let mut slow_start = sample("c-gcc", FpMode::Strict, false, 2_500_000_000, 42);
        slow_start.elapsed_ns = 1_800_000_000; // 700 ms of startup

        let analysis = analyze(&recording(vec![fast_start, slow_start]), Options::default());
        let aggregate = &analysis.algos[0].aggregates[0];

        // The naive `min(wall) - min(elapsed)` would be 2000 - 1800 = 200 ms: a
        // run that never happened. The smallest gap *within a sample* is 100 ms.
        assert_eq!(aggregate.run_startup.unwrap().min, 100_000_000);
        assert_eq!(aggregate.run_wall.unwrap().min, 2_000_000_000);
        assert_eq!(aggregate.run_elapsed.unwrap().min, 1_800_000_000);
    }

    /// The trap the whole wire format exists to avoid: 2^53 + 1 is the first
    /// integer a JavaScript `Number` cannot hold, and the checksum routinely
    /// exceeds it.
    #[test]
    fn a_checksum_crosses_the_wire_as_a_string_never_as_a_json_number() {
        let samples = vec![sample(
            "c-gcc",
            FpMode::Strict,
            false,
            2_000_000_000,
            9_007_199_254_740_993,
        )];
        let analysis = analyze(&recording(samples), Options::default());
        let json = serde_json::to_string(&analysis).unwrap();

        assert!(json.contains(r#""checksum":"9007199254740993""#), "{json}");
        assert!(
            json.contains(r#""strict_checksum":"9007199254740993""#),
            "{json}",
        );
        assert!(
            !json.contains("9007199254740993,"),
            "the checksum leaked onto the wire as a JSON number: {json}",
        );
    }

    /// Nanoseconds, microseconds and bytes stay numbers: they are plotted,
    /// divided and compared, and none of them comes close to 2^53.
    #[test]
    fn a_timing_crosses_the_wire_as_a_number_so_the_site_can_do_arithmetic_on_it() {
        let samples = vec![sample("c-gcc", FpMode::Strict, false, 2_000_000_000, 42)];
        let analysis = analyze(&recording(samples), Options::default());
        let json = serde_json::to_value(&analysis).unwrap();
        let run_wall = &json["algos"][0]["aggregates"][0]["run_wall"];
        assert_eq!(run_wall["min"], 2_000_000_000_u64);
        assert!(run_wall["min"].is_number());
    }
}
