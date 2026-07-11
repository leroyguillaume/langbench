//! Orchestration: prepare every image, then measure round-robin.
//!
//! Deliberately sequential. Running two benchmarks concurrently would destroy
//! the measurement, so there is no async here and no `tokio`.

use std::collections::HashMap;
use std::fs;
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use chrono::Utc;
use tracing::{info, warn};

use crate::cli::{Arch, FpMode, RunArgs};
use crate::discovery::{Implementation, discover};
use crate::engine::{BuildSpec, ContainerEngine, RunSpec};
use crate::machine::Machine;
use crate::sample::{Campaign, Phase, Sample, SampleWriter};
use crate::shutdown;

/// One (implementation, FP mode) pair: the atom of the schedule.
struct Unit {
    implementation: Implementation,
    mode: FpMode,
    image: String,
}

pub fn execute(args: RunArgs, engine: &impl ContainerEngine) -> Result<()> {
    let machine = Machine::collect();
    for warning in machine.warnings() {
        warn!("{warning}");
    }

    let implementations = discover(&args.benchmarks_dir, &args.algo)?;
    ensure!(
        !implementations.is_empty(),
        "no implementation found under {}",
        args.benchmarks_dir.display(),
    );

    let host = Arch::current();
    let units = schedule(&implementations, &args.mode, host);
    ensure!(
        !units.is_empty(),
        // Two ways to schedule nothing, and they call for different fixes: a mode
        // nobody declares is a flag to change, an architecture nobody builds on is
        // a machine to change. Blaming `modes` for what the ISA did would send the
        // reader looking in the wrong file.
        "no implementation is buildable on {} in any of the requested modes ({}). Every \
         discovered implementation either declares a narrower `modes` list, or declares an \
         `arch` list that does not include this machine.",
        host.map_or_else(
            || format!("this machine ({})", std::env::consts::ARCH),
            |host| host.to_string(),
        ),
        args.mode
            .iter()
            .map(FpMode::to_string)
            .collect::<Vec<_>>()
            .join(", "),
    );

    // The campaign is about to spend an hour measuring; find out *now* that its
    // destination directory does not exist, not when the first sample is due.
    if let Some(parent) = args.output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }

    let campaign = Campaign {
        langbench_version: env!("CARGO_PKG_VERSION").to_owned(),
        timestamp: Utc::now().to_rfc3339(),
        cpu: args.cpu,
        grid_size: args.grid_size,
        max_iter: args.max_iter,
        rounds: args.rounds,
        build_rounds: args.build_rounds,
        warmup_rounds: args.warmup_rounds,
        march: args.march.clone(),
        modes: args.mode.iter().map(FpMode::to_string).collect(),
    };

    let mut writer = SampleWriter::create(&args.output)?;
    writer.write_header(&machine, &campaign)?;

    let mut runner = Runner {
        engine,
        args: &args,
        writer,
        written: 0,
        strict_checksums: HashMap::new(),
    };

    let campaign = (|| -> Result<()> {
        runner.prepare(&units)?;
        runner.measure_phase(&units, Phase::Build, args.build_rounds)?;
        runner.measure_phase(&units, Phase::Run, args.rounds)
    })();

    // The campaign produces the samples and stops there. The CSV and the report
    // are renderings, recomputed on demand from this file — including from a
    // campaign that was interrupted, which is exactly when you want them.
    match campaign {
        Ok(()) => info!(
            samples = runner.written,
            path = %args.output.display(),
            "campaign complete; render it with `langbench csv` or `langbench md`",
        ),
        // A signal is an answer, not a failure: exit 0, because the samples on
        // disk are as valid as they were a moment ago and the file renders. The
        // partial campaign is the user's to keep or discard — a non-zero exit
        // would say the harness broke, and it did not.
        Err(error) if shutdown::was_interrupted(&error) => warn!(
            samples = runner.written,
            path = %args.output.display(),
            "campaign interrupted; the samples written so far are intact and \
             render with `langbench csv` or `langbench md`",
        ),
        Err(error) => return Err(error),
    }
    Ok(())
}

/// The requested modes, restricted to what each implementation actually
/// distinguishes — and to the implementations this machine can build at all.
///
/// An interpreter has one floating-point semantics: building it three times
/// would measure the same image under three names, and the report would show
/// three rows whose only difference is noise. The skip is loud — a row missing
/// from a report with no explanation is worse than a redundant one.
///
/// The architecture skip is the same idea about a harsher fact. Some toolchains
/// are simply not published for an ISA — Kotlin/Native has no `linux-aarch64`
/// host compiler — and the only ways to run one anyway are emulation, which this
/// project forbids, or cross-building, which would measure a build that never
/// happened here. So the manifest declares where it can be built, and a campaign
/// elsewhere drops the row *before* spending a `docker build` on discovering it.
fn schedule(
    implementations: &[Implementation],
    requested: &[FpMode],
    host: Option<Arch>,
) -> Vec<Unit> {
    let mut units = Vec::new();
    for implementation in implementations {
        if !implementation.supports(host) {
            warn!(
                algo = %implementation.algo,
                language = %implementation.language,
                compiler = none_if_absent(implementation.compiler.as_deref()),
                interpreter = none_if_absent(implementation.interpreter.as_deref()),
                host = host.map_or("unknown", Arch::as_str),
                declares = %implementation
                    .arches
                    .iter()
                    .map(Arch::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
                "skipping: this backend's toolchain does not exist for this architecture",
            );
            continue;
        }
        let selected = implementation.selected_modes(requested);
        for &mode in requested {
            if selected.contains(&mode) {
                units.push(Unit {
                    image: implementation.image(mode),
                    implementation: implementation.clone(),
                    mode,
                });
            } else {
                warn!(
                    algo = %implementation.algo,
                    language = %implementation.language,
                    compiler = none_if_absent(implementation.compiler.as_deref()),
                    interpreter = none_if_absent(implementation.interpreter.as_deref()),
                    %mode,
                    declares = %implementation
                        .fp_modes
                        .iter()
                        .map(FpMode::to_string)
                        .collect::<Vec<_>>()
                        .join(","),
                    "skipping: the implementation declares it does not distinguish this mode",
                );
            }
        }
    }
    units
}

/// A backend that compiles nothing, or interprets nothing, says so — a log field
/// that is sometimes absent cannot be filtered on.
fn none_if_absent(value: Option<&str>) -> &str {
    value.unwrap_or("none")
}

struct Runner<'a, E: ContainerEngine> {
    engine: &'a E,
    args: &'a RunArgs,
    writer: SampleWriter,
    /// Samples are streamed to disk, never accumulated: the harness holds a
    /// count, and whoever renders reads them back.
    written: usize,
    /// The value every strict-mode run of an algorithm must agree on, bit for
    /// bit, keyed by algorithm. Per algorithm and not per campaign: the checksum
    /// is a property of `(algo, grid size, max_iter)`, so a shared reference
    /// would abort the campaign on the first strict run of the second algorithm.
    strict_checksums: HashMap<String, u64>,
}

impl<E: ContainerEngine> Runner<'_, E> {
    /// Build every image. Never measured: this is where the network lives.
    fn prepare(&self, units: &[Unit]) -> Result<()> {
        for unit in units {
            shutdown::checkpoint()?;
            info!(image = %unit.image, "preparing image");
            let mut build_args = vec![
                ("FP_MODE".to_owned(), unit.mode.to_string()),
                ("JOBS".to_owned(), self.args.cpu.to_string()),
            ];
            if !self.args.march.is_empty() {
                build_args.push(("MARCH".to_owned(), self.args.march.clone()));
            }
            self.engine.build(&BuildSpec {
                image: unit.image.clone(),
                context: unit.implementation.context.clone(),
                build_args,
            })?;
        }
        Ok(())
    }

    /// Outer loop over rounds, inner loop over units. Never the reverse:
    /// blocking by implementation turns ambient noise into bias.
    fn measure_phase(&mut self, units: &[Unit], phase: Phase, rounds: u32) -> Result<()> {
        let total = self.args.warmup_rounds + rounds;
        info!(
            phase = phase.as_str(),
            rounds = total,
            units = units.len(),
            "measuring",
        );

        for round in 0..total {
            let warmup = round < self.args.warmup_rounds;
            for unit in units {
                // Between invocations, never inside one. A sample is written only
                // once it has been verified, so an interruption costs the run in
                // flight and nothing that came before it.
                shutdown::checkpoint()?;
                let sample = self.measure(unit, phase, round, warmup)?;
                self.verify(&sample)?;
                self.writer.write_sample(&sample)?;
                // One line per invocation: a campaign that prints nothing for an
                // hour is indistinguishable from a campaign that has hung.
                // The identity goes out as the three fields it is made of, never
                // as a slug: a log line is queried by field, and `compiler=gcc`
                // is a filter where `c-gcc` is a substring match waiting to
                // match `c-gcc-lto` too.
                info!(
                    phase = phase.as_str(),
                    round = round + 1,
                    of = total,
                    algo = %sample.algo,
                    language = %sample.language,
                    compiler = none_if_absent(sample.compiler.as_deref()),
                    interpreter = none_if_absent(sample.interpreter.as_deref()),
                    mode = %unit.mode,
                    warmup,
                    wall_ms = sample.wall_ns / 1_000_000,
                    "measured",
                );
                self.written += 1;
            }
        }
        Ok(())
    }

    fn measure(&self, unit: &Unit, phase: Phase, round: u32, warmup: bool) -> Result<Sample> {
        let execution = self.engine.run(&RunSpec {
            image: unit.image.clone(),
            args: self.container_args(phase),
            tmpfs_size_mb: self.args.tmpfs_size_mb,
            timeout: Duration::from_secs(self.args.run_timeout),
        })?;
        let record = execution.record;
        Ok(Sample {
            algo: unit.implementation.algo.clone(),
            language: unit.implementation.language.clone(),
            compiler: unit.implementation.compiler.clone(),
            interpreter: unit.implementation.interpreter.clone(),
            description: unit.implementation.description.clone(),
            comments: unit.implementation.comments.clone(),
            mode: unit.mode,
            phase,
            round,
            warmup,
            cpu: self.args.cpu,
            wall_ns: execution.wall_ns,
            elapsed_ns: record.elapsed_ns,
            user_usec: record.user_usec,
            system_usec: record.system_usec,
            checksum: record.checksum,
            binary_bytes: record.binary_bytes,
            binary_stripped_bytes: record.binary_stripped_bytes,
            text_bytes: record.text_bytes,
        })
    }

    fn container_args(&self, phase: Phase) -> Vec<String> {
        match phase {
            Phase::Build => vec!["build".to_owned(), self.args.cpu.to_string()],
            Phase::Run => vec![
                "run".to_owned(),
                self.args.grid_size.to_string(),
                self.args.max_iter.to_string(),
                self.args.cpu.to_string(),
            ],
        }
    }

    /// Every strict-mode run, warmup included. A wrong run is not a slow run.
    fn verify(&mut self, sample: &Sample) -> Result<()> {
        if sample.mode != FpMode::Strict {
            return Ok(());
        }
        let Some(checksum) = sample.checksum else {
            return Ok(());
        };
        match self.strict_checksums.get(&sample.algo) {
            None => {
                self.strict_checksums.insert(sample.algo.clone(), checksum);
                Ok(())
            }
            Some(&reference) if reference == checksum => Ok(()),
            Some(&reference) => bail!(
                "strict-mode checksum mismatch on {}: {} produced {checksum}, the reference is \
                 {reference}. In strict mode every compiler, language and ISA must agree bit \
                 for bit; a divergence is a bug in the code or the flags, never a rounding \
                 difference.",
                sample.algo,
                sample.backend(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{File, create_dir_all};
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    use tempfile::TempDir;

    use super::*;
    use crate::engine::{Execution, MockContainerEngine};
    use crate::sample::ContainerRecord;

    fn record(checksum: Option<u64>) -> ContainerRecord {
        ContainerRecord {
            elapsed_ns: 1_000,
            user_usec: 10,
            system_usec: 1,
            checksum,
            binary_bytes: None,
            binary_stripped_bytes: None,
            text_bytes: None,
        }
    }

    fn benchmarks(root: &Path, names: &[&str]) {
        benchmarks_for(root, "mandelbrot", names);
    }

    /// Backends are spelled as their slug — `c-gcc` — and written out as the
    /// manifest the harness actually reads.
    fn benchmarks_for(root: &Path, algo: &str, names: &[&str]) {
        for name in names {
            benchmark_declaring_for(root, algo, name, "all");
        }
    }

    fn benchmark_declaring(root: &Path, name: &str, modes: &str) {
        benchmark_declaring_for(root, "mandelbrot", name, modes);
    }

    /// `modes` is `all`, or the modes a manifest would list.
    fn benchmark_declaring_for(root: &Path, algo: &str, name: &str, modes: &str) {
        let (language, compiler) = name.split_once('-').expect("<language>-<compiler>");
        let modes = if modes == "all" {
            modes.to_owned()
        } else {
            format!("[{modes}]")
        };
        let dir = root.join(algo).join(name);
        create_dir_all(&dir).unwrap();
        File::create(dir.join("Dockerfile")).unwrap();
        std::fs::write(
            dir.join(crate::discovery::MANIFEST),
            format!(
                "algo: {algo}\n\
                 language: {language}\n\
                 compiler: {compiler}\n\
                 modes: {modes}\n\
                 description: {name}, as the fixture declares it.\n",
            ),
        )
        .unwrap();
    }

    fn args(benchmarks_dir: &Path, output: &Path, modes: Vec<FpMode>) -> RunArgs {
        RunArgs {
            algo: vec![],
            mode: modes,
            cpu: 4,
            output: output.join("samples.ndjson"),
            benchmarks_dir: benchmarks_dir.to_path_buf(),
            grid_size: 64,
            max_iter: 10,
            rounds: 2,
            build_rounds: 1,
            warmup_rounds: 1,
            march: "x86-64-v3".to_owned(),
            tmpfs_size_mb: 16,
            run_timeout: 60,
        }
    }

    #[test]
    fn an_empty_benchmark_tree_is_an_error() {
        let root = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        create_dir_all(root.path().join("mandelbrot")).unwrap();

        let engine = MockContainerEngine::new();
        let err =
            execute(args(root.path(), out.path(), vec![FpMode::Strict]), &engine).unwrap_err();
        assert!(err.to_string().contains("no implementation found"));
    }

    #[test]
    fn every_unit_is_prepared_once_and_measured_every_round() {
        let root = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        benchmarks(root.path(), &["c-gcc", "rust-llvm"]);

        let mut engine = MockContainerEngine::new();
        // 2 implementations x 2 modes.
        engine.expect_build().times(4).returning(|_| Ok(()));
        // 4 units x ((1 warmup + 1 build round) + (1 warmup + 2 run rounds)).
        engine.expect_run().times(4 * 5).returning(|_| {
            Ok(Execution {
                wall_ns: 2_000,
                record: record(Some(42)),
            })
        });

        let args = args(root.path(), out.path(), vec![FpMode::Strict, FpMode::Fast]);
        execute(args, &engine).unwrap();

        // The samples, and strictly nothing else: rendering is a separate command
        // now, so a campaign cannot emit a report it did not measure.
        assert!(out.path().join("samples.ndjson").is_file());
        assert!(!out.path().join("report.md").exists());
        assert!(!out.path().join("samples.csv").exists());
    }

    #[test]
    fn the_samples_file_gets_the_directories_it_needs() {
        // `--output` names a file now, and an hour of measuring must not be lost
        // to a parent directory that was never created.
        let root = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        benchmarks(root.path(), &["c-gcc"]);

        let mut engine = MockContainerEngine::new();
        engine.expect_build().returning(|_| Ok(()));
        engine.expect_run().returning(|_| {
            Ok(Execution {
                wall_ns: 2_000,
                record: record(Some(42)),
            })
        });

        let mut args = args(root.path(), out.path(), vec![FpMode::Strict]);
        args.output = out.path().join("x86-64/strict/samples.ndjson");
        let path = args.output.clone();
        execute(args, &engine).unwrap();

        assert!(path.is_file());
    }

    #[test]
    fn an_implementation_is_never_built_under_a_mode_it_does_not_distinguish() {
        // CPython has one FP semantics: `fma` and `fast` would be the same image
        // under another tag, and three identical rows in the report.
        let root = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        benchmarks(root.path(), &["c-gcc"]);
        benchmark_declaring(root.path(), "python-cpython", "strict");

        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);

        let mut engine = MockContainerEngine::new();
        engine.expect_build().returning(move |spec| {
            recorded.lock().unwrap().push(spec.image.clone());
            Ok(())
        });
        engine.expect_run().returning(|_| {
            Ok(Execution {
                wall_ns: 2_000,
                record: record(Some(42)),
            })
        });

        execute(args(root.path(), out.path(), FpMode::ALL.to_vec()), &engine).unwrap();

        // Three images for gcc, one for CPython — not six.
        let images = seen.lock().unwrap().clone();
        assert_eq!(
            images,
            [
                "langbench/mandelbrot-c-gcc:strict",
                "langbench/mandelbrot-c-gcc:fma",
                "langbench/mandelbrot-c-gcc:fast",
                "langbench/mandelbrot-python-cpython:strict",
            ],
        );
    }

    #[test]
    fn requesting_only_modes_nobody_distinguishes_is_an_error() {
        // An empty campaign must fail here, not produce a samples file with a
        // header and no samples.
        let root = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        benchmark_declaring(root.path(), "python-cpython", "strict");

        let engine = MockContainerEngine::new();
        let err = execute(args(root.path(), out.path(), vec![FpMode::Fast]), &engine).unwrap_err();
        // The message must name both ways a campaign can end up empty — a mode
        // nobody declares, or an architecture nobody builds on — because the two
        // call for different fixes and the reader has to know which one this was.
        assert!(
            err.to_string().contains("no implementation is buildable"),
            "{err}"
        );
        assert!(err.to_string().contains("modes"), "{err}");
        assert!(err.to_string().contains("arch"), "{err}");
    }

    #[test]
    fn units_are_interleaved_round_robin_not_blocked_by_implementation() {
        let root = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        benchmarks(root.path(), &["c-gcc", "rust-llvm"]);

        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);

        let mut engine = MockContainerEngine::new();
        engine.expect_build().returning(|_| Ok(()));
        engine.expect_run().returning(move |spec| {
            recorded.lock().unwrap().push(spec.image.clone());
            Ok(Execution {
                wall_ns: 2_000,
                record: record(Some(42)),
            })
        });

        let mut args = args(root.path(), out.path(), vec![FpMode::Strict]);
        args.warmup_rounds = 0;
        args.build_rounds = 0;
        args.rounds = 2;
        execute(args, &engine).unwrap();

        let images = seen.lock().unwrap().clone();
        assert_eq!(
            images,
            [
                "langbench/mandelbrot-c-gcc:strict",
                "langbench/mandelbrot-rust-llvm:strict",
                "langbench/mandelbrot-c-gcc:strict",
                "langbench/mandelbrot-rust-llvm:strict",
            ],
        );
    }

    #[test]
    fn a_strict_mode_checksum_divergence_aborts_the_campaign() {
        let root = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        benchmarks(root.path(), &["c-gcc", "c-clang"]);

        let checksums = Arc::new(Mutex::new(vec![7, 9]));
        let mut engine = MockContainerEngine::new();
        engine.expect_build().returning(|_| Ok(()));
        engine.expect_run().returning(move |_| {
            let checksum = checksums.lock().unwrap().remove(0);
            Ok(Execution {
                wall_ns: 2_000,
                record: record(Some(checksum)),
            })
        });

        let mut args = args(root.path(), out.path(), vec![FpMode::Strict]);
        args.warmup_rounds = 0;
        args.build_rounds = 0;
        args.rounds = 1;
        let err = execute(args, &engine).unwrap_err();
        assert!(err.to_string().contains("checksum mismatch"), "{err}");
    }

    #[test]
    fn two_algorithms_are_each_verified_against_their_own_reference() {
        // The checksum is a property of (algo, grid size, max_iter), so two
        // algorithms legitimately disagree. A campaign-wide reference would abort
        // on the first strict run of the second algorithm.
        let root = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        benchmarks_for(root.path(), "mandelbrot", &["c-gcc"]);
        benchmarks_for(root.path(), "nbody", &["c-gcc"]);

        let mut engine = MockContainerEngine::new();
        engine.expect_build().returning(|_| Ok(()));
        engine.expect_run().returning(|spec| {
            let checksum = if spec.image.contains("nbody") { 9 } else { 7 };
            Ok(Execution {
                wall_ns: 2_000,
                record: record(Some(checksum)),
            })
        });

        let mut args = args(root.path(), out.path(), vec![FpMode::Strict]);
        args.warmup_rounds = 0;
        args.build_rounds = 0;
        args.rounds = 2;
        execute(args, &engine).unwrap();
    }

    #[test]
    fn a_relaxed_mode_checksum_never_gates_the_campaign() {
        let root = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        benchmarks(root.path(), &["c-gcc", "c-clang"]);

        let checksums = Arc::new(Mutex::new(vec![7, 9]));
        let mut engine = MockContainerEngine::new();
        engine.expect_build().returning(|_| Ok(()));
        engine.expect_run().returning(move |_| {
            let checksum = checksums.lock().unwrap().remove(0);
            Ok(Execution {
                wall_ns: 2_000,
                record: record(Some(checksum)),
            })
        });

        let mut args = args(root.path(), out.path(), vec![FpMode::Fast]);
        args.warmup_rounds = 0;
        args.build_rounds = 0;
        args.rounds = 1;
        execute(args, &engine).unwrap();
    }

    /// A backend whose toolchain does not exist for this ISA is dropped from the
    /// schedule *before* a `docker build` discovers it the hard way — and the drop
    /// is loud, because a row that silently vanishes from a report is worse than a
    /// row that failed.
    #[test]
    fn an_implementation_is_skipped_on_an_architecture_it_cannot_build_on() {
        let root = TempDir::new().unwrap();
        let dir = root.path().join("mandelbrot").join("kotlin-kotlin-native");
        create_dir_all(&dir).unwrap();
        File::create(dir.join("Dockerfile")).unwrap();
        std::fs::write(
            dir.join(crate::discovery::MANIFEST),
            "algo: mandelbrot\n\
             language: kotlin\n\
             compiler: kotlin-native\n\
             modes: [strict]\n\
             arch: [x86_64]\n\
             description: No linux-aarch64 host compiler exists.\n",
        )
        .unwrap();
        benchmarks(root.path(), &["c-gcc"]);

        let implementations = discover(root.path(), &[]).unwrap();
        assert_eq!(implementations.len(), 2);

        // On x86-64 both are scheduled; on AArch64 only the C one survives.
        let on_x86 = schedule(&implementations, &[FpMode::Strict], Some(Arch::X86_64));
        assert_eq!(on_x86.len(), 2);

        let on_arm = schedule(&implementations, &[FpMode::Strict], Some(Arch::Aarch64));
        assert_eq!(on_arm.len(), 1);
        assert_eq!(on_arm[0].implementation.language, "c");
    }

    /// The point of the whole shutdown path: a signal costs the run in flight,
    /// and nothing that came before it.
    #[test]
    fn an_interrupted_campaign_succeeds_and_keeps_every_sample_it_wrote() {
        let root = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        benchmarks(root.path(), &["c-gcc"]);

        // Two runs land, the third is the one the signal caught.
        let remaining = Arc::new(Mutex::new(2_u32));
        let mut engine = MockContainerEngine::new();
        engine.expect_build().returning(|_| Ok(()));
        engine.expect_run().returning(move |_| {
            let mut remaining = remaining.lock().unwrap();
            if *remaining == 0 {
                return Err(anyhow::Error::from(crate::shutdown::Interrupted)
                    .context("container `langbench-1-2` killed on the way out"));
            }
            *remaining -= 1;
            Ok(Execution {
                wall_ns: 2_000,
                record: record(Some(42)),
            })
        });

        let mut args = args(root.path(), out.path(), vec![FpMode::Strict]);
        args.warmup_rounds = 0;
        args.build_rounds = 0;
        args.rounds = 4;
        let output = args.output.clone();

        // Exit 0: the harness did not break, it was asked to stop.
        execute(args, &engine).unwrap();

        // The header, plus exactly the two samples that completed. The killed run
        // contributes nothing — a wrong run never enters the statistics.
        let written = std::fs::read_to_string(&output).unwrap();
        assert_eq!(written.lines().count(), 3, "header + 2 samples");
    }

    /// The interruption path must not become a way for real failures to exit 0.
    #[test]
    fn an_ordinary_failure_still_fails_the_campaign() {
        let root = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        benchmarks(root.path(), &["c-gcc"]);

        let mut engine = MockContainerEngine::new();
        engine.expect_build().returning(|_| Ok(()));
        engine
            .expect_run()
            .returning(|_| bail!("`docker run` failed for langbench/mandelbrot-c-gcc:strict"));

        let mut args = args(root.path(), out.path(), vec![FpMode::Strict]);
        args.warmup_rounds = 0;
        args.build_rounds = 0;
        args.rounds = 1;
        let error = execute(args, &engine).unwrap_err();
        assert!(error.to_string().contains("`docker run` failed"));
    }
}
