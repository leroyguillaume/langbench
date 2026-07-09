//! Orchestration: prepare every image, then measure round-robin.
//!
//! Deliberately sequential. Running two benchmarks concurrently would destroy
//! the measurement, so there is no async here and no `tokio`.

use std::fs;
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use chrono::Utc;
use tracing::{info, warn};

use crate::cli::{FpMode, RunArgs};
use crate::discovery::{Implementation, discover};
use crate::engine::{BuildSpec, ContainerEngine, RunSpec};
use crate::machine::Machine;
use crate::report;
use crate::sample::{Campaign, Phase, Sample, SampleWriter};

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

    let units: Vec<Unit> = implementations
        .into_iter()
        .flat_map(|implementation| {
            args.mode.iter().map(move |&mode| Unit {
                image: implementation.image(mode),
                implementation: implementation.clone(),
                mode,
            })
        })
        .collect();

    fs::create_dir_all(&args.output_dir)
        .with_context(|| format!("creating {}", args.output_dir.display()))?;

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

    let mut writer = SampleWriter::create(&args.output_dir)?;
    writer.write_header(&machine, &campaign)?;

    let mut runner = Runner {
        engine,
        args: &args,
        writer,
        samples: Vec::new(),
        strict_checksum: None,
    };

    runner.prepare(&units)?;
    runner.measure_phase(&units, Phase::Build, args.build_rounds)?;
    runner.measure_phase(&units, Phase::Run, args.rounds)?;

    let data = report::build(&machine, &campaign, &runner.samples, runner.strict_checksum);
    let markdown = report::render(&data)?;
    let report_path = args.output_dir.join("report.md");
    fs::write(&report_path, markdown)
        .with_context(|| format!("writing {}", report_path.display()))?;

    info!(
        samples = runner.samples.len(),
        output_dir = %args.output_dir.display(),
        "campaign complete",
    );
    Ok(())
}

struct Runner<'a, E: ContainerEngine> {
    engine: &'a E,
    args: &'a RunArgs,
    writer: SampleWriter,
    samples: Vec<Sample>,
    /// The one value every strict-mode run must agree on, bit for bit.
    strict_checksum: Option<u64>,
}

impl<E: ContainerEngine> Runner<'_, E> {
    /// Build every image. Never measured: this is where the network lives.
    fn prepare(&self, units: &[Unit]) -> Result<()> {
        for unit in units {
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
                let sample = self.measure(unit, phase, round, warmup)?;
                self.verify(&sample)?;
                self.writer.write_sample(&sample)?;
                // One line per invocation: a campaign that prints nothing for an
                // hour is indistinguishable from a campaign that has hung.
                info!(
                    phase = phase.as_str(),
                    round = round + 1,
                    of = total,
                    image = %unit.image,
                    warmup,
                    wall_ms = sample.wall_ns / 1_000_000,
                    "measured",
                );
                self.samples.push(sample);
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
            implementation: unit.implementation.name.clone(),
            language: unit.implementation.language.clone(),
            compiler: unit.implementation.compiler.clone(),
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
        match self.strict_checksum {
            None => {
                self.strict_checksum = Some(checksum);
                Ok(())
            }
            Some(reference) if reference == checksum => Ok(()),
            Some(reference) => bail!(
                "strict-mode checksum mismatch: {} produced {checksum}, the reference is \
                 {reference}. In strict mode every compiler, language and ISA must agree bit \
                 for bit; a divergence is a bug in the code or the flags, never a rounding \
                 difference.",
                sample.implementation,
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
        for name in names {
            let dir = root.join("mandelbrot").join(name);
            create_dir_all(&dir).unwrap();
            File::create(dir.join("Dockerfile")).unwrap();
        }
    }

    fn args(benchmarks_dir: &Path, output_dir: &Path, modes: Vec<FpMode>) -> RunArgs {
        RunArgs {
            algo: vec![],
            mode: modes,
            cpu: 4,
            output_dir: output_dir.to_path_buf(),
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

        assert!(out.path().join("samples.ndjson").is_file());
        assert!(out.path().join("report.md").is_file());
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
}
