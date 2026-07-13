//! The numbers, before anybody formats them.
//!
//! Bucketing samples into `(workload, backend, mode)` and summarizing each bucket is
//! the one piece of arithmetic in this repository that has more than one
//! consumer: `langbench report md` formats it into a table, and the WebAssembly build
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
use crate::workload::Workload;

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
    /// The architecture the campaign ran on, lifted out of the machine so a consumer can
    /// key on it without scraping a label out of [`Self::machine_fields`].
    ///
    /// The website needs it for one reason: **an absolute timing never crosses an
    /// architecture**. Two campaigns from two architectures are two experiments, and the
    /// site has to be able to tell them apart structurally — from the header the
    /// campaign recorded, never from the name of the file it was served under. A
    /// filename is a label somebody types; this is what the machine said about
    /// itself. See `METHODOLOGY.md#the-architecture-rule`.
    pub architecture: String,
    pub hostname: Option<String>,
    pub machine_fields: Vec<Field>,
    /// Every reason this host was a poor benchmark target. It travels with the
    /// numbers, so a chart cannot be read without the caveat that qualifies it.
    pub warnings: Vec<String>,
    pub workloads: Vec<WorkloadAnalysis>,
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
    pub workload: String,
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
pub struct WorkloadAnalysis {
    pub workload: String,
    /// The value every strict-mode run of *this* workload agreed on. A property
    /// of `(workload, grid size, max_iter)`, never of the campaign.
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
    pub workload: String,
    pub backend: String,
    pub language: String,
    pub compiler: Option<String>,
    pub interpreter: Option<String>,
    pub description: String,
    pub comments: Option<String>,
    /// Bytes of the kernel this backend runs. On the card and not only on the row:
    /// it is the same file in all three FP modes, because the mode is a compiler
    /// flag and never an edit to the source.
    pub source_bytes: Option<u64>,
}

/// Everything the campaign learned about one `(workload, backend, mode)`.
///
/// Numbers, in their native units — nanoseconds, microseconds, bytes. Not one
/// formatted string: `"2000.0 ms"` cannot be sorted, plotted, or divided by
/// another backend's time, and a renderer that receives it has already lost the
/// only thing it needed.
#[derive(Debug, Serialize)]
pub struct Aggregate {
    pub workload: String,
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
    /// Cores kept busy, in thousandths of a core, per sample. Read against
    /// [`Self::cpu`]: the thread count the harness handed this kernel.
    ///
    /// The **median**, not the minimum, is the statistic to quote — and it is the
    /// one exception on this page. Every timing here is a min-of-N because
    /// contention can only ever *slow a run down*, so the minimum estimates the
    /// machine's capability. Parallelism is not like that: contention inflates the
    /// CPU clock (threads spinning) and the compute clock alike, in both
    /// directions, so there is no one-sided noise to argue from and no reason the
    /// extreme should be the estimate. See
    /// `METHODOLOGY.md#parallel-efficiency-is-a-median-not-a-minimum`.
    pub run_cores: Option<Summary>,
    /// The peak memory the container needed, min-of-N. The minimum is right for
    /// the same reason it is right for a timing, and for once the argument is
    /// exact rather than statistical: page cache and a lazy collector can only
    /// ever push the high-water mark *up*, never below what the backend actually
    /// had to allocate.
    pub run_peak_bytes: Option<Summary>,
    /// The compiler's own elapsed time, from inside the container — never the
    /// `docker build` wall-clock. See
    /// `METHODOLOGY.md#the-build-column-reports-the-internal-clock-the-run-column-the-external-one`.
    pub build_elapsed: Option<Summary>,
    /// What the *compiler* did with the cores it was given. A single-threaded
    /// front end is a fact about a toolchain, and it is invisible in a wall-clock.
    pub build_cores: Option<Summary>,
    /// What the *compiler* needed. It includes the tmpfs the build wrote into,
    /// which is charged to the same cgroup — a compiler's output is memory it made
    /// the machine hold.
    pub build_peak_bytes: Option<Summary>,
    /// The thread count the harness handed every kernel of this campaign. Carried
    /// on the row so a reader can size [`Self::run_cores`] against it without
    /// reaching back into the campaign header.
    pub cpu: usize,
    /// Bytes of the kernel's one source file. A property of the *language*, not of
    /// the backend: `c-gcc` and `c-clang` compile the same file and report the same
    /// number. See [`crate::sample::Sample::source_bytes`].
    pub source_bytes: Option<u64>,
    pub binary_bytes: Option<u64>,
    pub binary_stripped_bytes: Option<u64>,
    pub text_bytes: Option<u64>,
    /// A 64-bit integer, on the wire as a string. See
    /// [`WorkloadAnalysis::strict_checksum`].
    #[serde(serialize_with = "as_string")]
    pub checksum: Option<u64>,
    /// This mode's distance from the strict reference: the precision sold for the
    /// speed gained. `i128`, because two `u64` checksums can differ by more than
    /// an `i64` holds — and a string on the wire for the same reason as the
    /// checksum itself.
    #[serde(serialize_with = "as_string")]
    pub checksum_delta: Option<i128>,
}

/// Samples accumulated for one `(workload, backend, mode)`, before summarizing.
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
    run_cores: Vec<u64>,
    run_peak_bytes: Vec<u64>,
    build_elapsed: Vec<u64>,
    build_cores: Vec<u64>,
    build_peak_bytes: Vec<u64>,
    cpu: usize,
    source_bytes: Option<u64>,
    checksum: Option<u64>,
    binary_bytes: Option<u64>,
    binary_stripped_bytes: Option<u64>,
    text_bytes: Option<u64>,
}

pub fn analyze(recording: &Recording, options: Options) -> Analysis {
    let samples = &recording.samples;
    let references = strict_references(&recording.campaign.workload, samples);

    // Insertion order comes from the first round, which is the schedule order.
    let mut order: Vec<(String, String, FpMode)> = Vec::new();
    let mut buckets: HashMap<(String, String, FpMode), Bucket> = HashMap::new();

    for sample in samples {
        let key = (sample.workload.clone(), sample.backend(), sample.mode);
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

        // Constants of the image, of the source, of the schedule: take them
        // wherever they first appear, warmup round included. None of them is a
        // measurement, so none of them is sampled.
        bucket.checksum = bucket.checksum.or(sample.checksum);
        bucket.binary_bytes = bucket.binary_bytes.or(sample.binary_bytes);
        bucket.binary_stripped_bytes = bucket
            .binary_stripped_bytes
            .or(sample.binary_stripped_bytes);
        bucket.text_bytes = bucket.text_bytes.or(sample.text_bytes);
        bucket.source_bytes = bucket.source_bytes.or(sample.source_bytes);
        bucket.cpu = sample.cpu;

        if sample.warmup && !options.include_warmup {
            continue;
        }
        // `peak_bytes` and the core count are each an `Option`: a kernel with no
        // `memory.peak`, a run that reported no compute time. An absent value is not
        // pushed, so it never becomes a zero in a summary — and the metric ends up
        // `None` for the whole row rather than reporting a backend that used no
        // memory.
        match sample.phase {
            Phase::Build => {
                bucket.build_elapsed.push(sample.elapsed_ns);
                bucket.build_cores.extend(sample.cores_milli());
                bucket.build_peak_bytes.extend(sample.peak_bytes);
            }
            Phase::Run => {
                bucket.run_wall.push(sample.wall_ns);
                bucket.run_elapsed.push(sample.elapsed_ns);
                bucket.run_startup.push(sample.startup_ns());
                bucket.run_cpu_usec.push(sample.cpu_usec());
                bucket.run_cores.extend(sample.cores_milli());
                bucket.run_peak_bytes.extend(sample.peak_bytes);
            }
        }
    }

    let mut workloads: Vec<WorkloadAnalysis> = Vec::new();
    let mut backends: Vec<Backend> = Vec::new();
    for key in &order {
        let (workload, backend, mode) = key;
        let bucket = &buckets[key];
        let reference = references.get(workload).copied();
        let aggregate = Aggregate {
            workload: workload.clone(),
            backend: backend.clone(),
            backend_id: backend_id(workload, backend),
            language: bucket.language.clone(),
            compiler: bucket.compiler.clone(),
            interpreter: bucket.interpreter.clone(),
            mode: *mode,
            run_wall: summarize(&bucket.run_wall),
            run_elapsed: summarize(&bucket.run_elapsed),
            run_startup: summarize(&bucket.run_startup),
            run_cpu_usec: summarize(&bucket.run_cpu_usec),
            run_cores: summarize(&bucket.run_cores),
            run_peak_bytes: summarize(&bucket.run_peak_bytes),
            build_elapsed: summarize(&bucket.build_elapsed),
            build_cores: summarize(&bucket.build_cores),
            build_peak_bytes: summarize(&bucket.build_peak_bytes),
            cpu: bucket.cpu,
            source_bytes: bucket.source_bytes,
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
        let id = backend_id(workload, backend);
        if !backends.iter().any(|known| known.id == id) {
            backends.push(Backend {
                id,
                workload: workload.clone(),
                backend: backend.clone(),
                language: bucket.language.clone(),
                compiler: bucket.compiler.clone(),
                interpreter: bucket.interpreter.clone(),
                description: bucket.description.clone(),
                comments: bucket.comments.clone(),
                source_bytes: bucket.source_bytes,
            });
        }

        match workloads
            .iter_mut()
            .find(|analysis| &analysis.workload == workload)
        {
            Some(analysis) => analysis.aggregates.push(aggregate),
            None => workloads.push(WorkloadAnalysis {
                workload: workload.clone(),
                strict_checksum: reference,
                aggregates: vec![aggregate],
            }),
        }
    }

    // Fastest first, on the same statistic the table headlines: the minimum
    // wall-clock. `sort_by_key` is stable, so rows the campaign could not measure
    // keep their schedule order at the bottom instead of being shuffled among
    // themselves.
    for analysis in &mut workloads {
        analysis.aggregates.sort_by_key(|aggregate| {
            let min = aggregate.run_wall.map(|summary| summary.min);
            (min.is_none(), min)
        });
    }

    Analysis {
        campaign: recording.campaign.clone(),
        options,
        architecture: recording.machine.architecture.clone(),
        hostname: recording.machine.hostname.clone(),
        machine_fields: recording.machine.fields(),
        warnings: recording.machine.warnings(),
        workloads,
        backends,
        failures: recording.failures.iter().map(failure).collect(),
    }
}

/// A recorded failure, with the slugs a renderer cannot derive for itself.
fn failure(failure: &crate::sample::Failure) -> Failure {
    let backend = failure.backend();
    Failure {
        backend_id: backend_id(&failure.workload, &backend),
        workload: failure.workload.clone(),
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
pub fn backend_id(workload: &str, backend: &str) -> String {
    format!("{workload}-{backend}")
}

/// The value every strict-mode run of this campaign's workload agreed on.
///
/// **The workload's own `strict_checksum` wins**, when it declares one: it is the
/// answer to the work, established once and outliving any run, and the campaign
/// already refused to record a sample that diverged from it.
///
/// Without one, the reference is whatever the first strict sample produced — which
/// is all a campaign can establish on its own, and it is weaker than it looks: it
/// says the backends agreed with each other, not that any of them was right. It is
/// read out of the samples rather than recomputed, because `Runner::verify` aborted
/// on the spot for any divergence, so the first is as good as the last.
///
/// Keyed by workload id all the same, because that is the key the samples carry and
/// the renderers look up. A campaign has exactly one.
/// See `METHODOLOGY.md#the-strict-mode-invariant`.
fn strict_references(workload: &Workload, samples: &[Sample]) -> HashMap<String, u64> {
    let mut references = HashMap::new();
    if let Some(declared) = workload.checksum {
        references.insert(workload.id.clone(), declared);
    }
    for sample in samples {
        if sample.mode != FpMode::Strict {
            continue;
        }
        if let Some(checksum) = sample.checksum {
            references
                .entry(sample.workload.clone())
                .or_insert(checksum);
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
pub(crate) fn as_string<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
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
            workload: Workload::fixture(),
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
            workload: "mandelbrot".to_owned(),
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
            peak_bytes: Some(12_582_912),
            source_bytes: Some(2_048),
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
        let backends: Vec<&str> = analysis.workloads[0]
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
        let run = cold.workloads[0].aggregates[0].run_wall.unwrap();
        assert_eq!(run.n, 1);
        assert_eq!(run.min, 2_000_000_000);

        let warm = analyze(
            &recording(samples),
            Options {
                include_warmup: true,
            },
        );
        let run = warm.workloads[0].aggregates[0].run_wall.unwrap();
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
        let aggregate = &analysis.workloads[0].aggregates[0];
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
        assert_eq!(analysis.workloads[0].strict_checksum, Some(1_000));

        let fast = analysis.workloads[0]
            .aggregates
            .iter()
            .find(|aggregate| aggregate.mode == FpMode::Fast)
            .unwrap();
        assert_eq!(fast.checksum_delta, Some(-6));
    }

    /// The architecture is read off the machine the campaign recorded, never off a
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
        recording.machine.architecture = "x86_64".to_owned();
        recording.machine.hostname = Some("bench-01".to_owned());

        let analysis = analyze(&recording, Options::default());
        assert_eq!(analysis.architecture, "x86_64");
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
        let aggregate = &analysis.workloads[0].aggregates[0];

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

    /// The one metric on this page that is *not* a min-of-N. Contention slows a
    /// run down, so the minimum estimates a timing; it pushes the core count in
    /// both directions, so nothing recommends the extreme over the middle.
    #[test]
    fn the_core_count_is_summarized_from_the_median_not_the_minimum() {
        // Two rounds of the same backend: one where it got the machine to itself,
        // one where it was fighting for it.
        let mut clean = sample("c-gcc", FpMode::Strict, false, 2_000_000_000, 42);
        clean.cpu = 8;
        clean.elapsed_ns = 1_000_000_000;
        clean.user_usec = 7_800_000; // 7.8 cores
        let mut contended = clean.clone();
        contended.user_usec = 3_100_000; // 3.1 cores

        let analysis = analyze(&recording(vec![clean, contended]), Options::default());
        let cores = analysis.workloads[0].aggregates[0].run_cores.unwrap();
        assert_eq!(cores.n, 2);
        // The lower median of the two, and *not* silently the smaller one as some
        // statistic in its own right: the minimum is what a timing reports.
        assert_eq!(cores.median, 3_100);
        assert_eq!(analysis.workloads[0].aggregates[0].cpu, 8);
    }

    /// The GIL, as the table shows it: same workload, same eight threads, one
    /// core. Without this column a reader sees only "slow" and cannot tell a bad
    /// code generator from a runtime that will not parallelise at all.
    #[test]
    fn a_serial_backend_and_a_parallel_one_are_told_apart_by_their_cores() {
        let mut parallel = sample("c-gcc", FpMode::Strict, false, 2_000_000_000, 42);
        parallel.cpu = 8;
        parallel.elapsed_ns = 1_000_000_000;
        parallel.user_usec = 8_000_000;

        let mut serial = sample("python-cpython", FpMode::Strict, false, 9_000_000_000, 42);
        serial.cpu = 8;
        serial.elapsed_ns = 8_000_000_000;
        serial.user_usec = 8_000_000;

        let analysis = analyze(&recording(vec![parallel, serial]), Options::default());
        let cores = |backend: &str| {
            analysis.workloads[0]
                .aggregates
                .iter()
                .find(|aggregate| aggregate.backend == backend)
                .unwrap()
                .run_cores
                .unwrap()
                .median
        };
        assert_eq!(cores("c-gcc"), 8_000);
        assert_eq!(cores("python-cpython"), 1_000);
    }

    /// Page cache and a lazy collector can only ever push the high-water mark up.
    /// The minimum is the memory the backend genuinely had to have.
    #[test]
    fn peak_memory_is_the_smallest_high_water_mark_the_campaign_saw() {
        let mut lean = sample("c-gcc", FpMode::Strict, false, 2_000_000_000, 42);
        lean.peak_bytes = Some(10_000_000);
        let mut fat = sample("c-gcc", FpMode::Strict, false, 2_100_000_000, 42);
        fat.peak_bytes = Some(14_000_000);

        let analysis = analyze(&recording(vec![lean, fat]), Options::default());
        assert_eq!(
            analysis.workloads[0].aggregates[0]
                .run_peak_bytes
                .unwrap()
                .min,
            10_000_000,
        );
    }

    /// A kernel with no `memory.peak` reports no peak — never zero bytes, which
    /// would read as the most frugal backend ever measured.
    #[test]
    fn a_campaign_with_no_memory_peak_reports_an_absence_never_a_zero() {
        let mut blind = sample("c-gcc", FpMode::Strict, false, 2_000_000_000, 42);
        blind.peak_bytes = None;

        let analysis = analyze(&recording(vec![blind]), Options::default());
        let aggregate = &analysis.workloads[0].aggregates[0];
        assert_eq!(aggregate.run_peak_bytes, None);
        // ... and the rest of the row is unharmed.
        assert_eq!(aggregate.run_wall.unwrap().min, 2_000_000_000);
    }

    /// A single blind round must not drag a summary down: the samples that *do*
    /// carry a number are summarized on their own, and `n` says how many there were.
    #[test]
    fn a_round_that_reported_no_memory_peak_is_left_out_rather_than_counted_as_zero() {
        let measured = sample("c-gcc", FpMode::Strict, false, 2_000_000_000, 42);
        let mut blind = sample("c-gcc", FpMode::Strict, false, 2_100_000_000, 42);
        blind.peak_bytes = None;

        let analysis = analyze(&recording(vec![measured, blind]), Options::default());
        let peak = analysis.workloads[0].aggregates[0].run_peak_bytes.unwrap();
        assert_eq!(peak.n, 1);
        assert_eq!(peak.min, 12_582_912);
    }

    /// The source is the language's, not the backend's: the same file compiled by
    /// two compilers is the same number twice, and that is the honest answer.
    #[test]
    fn the_source_size_travels_onto_the_row_and_onto_the_backend_card() {
        let analysis = analyze(
            &recording(vec![sample(
                "c-gcc",
                FpMode::Strict,
                false,
                2_000_000_000,
                42,
            )]),
            Options::default(),
        );
        assert_eq!(
            analysis.workloads[0].aggregates[0].source_bytes,
            Some(2_048)
        );
        assert_eq!(analysis.backends[0].source_bytes, Some(2_048));
    }

    /// Nanoseconds, microseconds and bytes stay numbers: they are plotted,
    /// divided and compared, and none of them comes close to 2^53.
    #[test]
    fn a_timing_crosses_the_wire_as_a_number_so_the_site_can_do_arithmetic_on_it() {
        let samples = vec![sample("c-gcc", FpMode::Strict, false, 2_000_000_000, 42)];
        let analysis = analyze(&recording(samples), Options::default());
        let json = serde_json::to_value(&analysis).unwrap();
        let run_wall = &json["workloads"][0]["aggregates"][0]["run_wall"];
        assert_eq!(run_wall["min"], 2_000_000_000_u64);
        assert!(run_wall["min"].is_number());
    }
}
