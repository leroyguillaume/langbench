//! Orchestration: prepare every image, then measure round-robin.
//!
//! Deliberately sequential. Running two benchmarks concurrently would destroy
//! the measurement, so there is no async here and no `tokio`.

use std::collections::HashSet;
use std::fs;
use std::time::Duration;

use anyhow::{Context, Result, bail, ensure};
use chrono::Utc;
use tracing::{error, info, warn};

use crate::cli::{Architecture, RunArgs};
use crate::discovery::{Implementation, discover, workloads};
use crate::engine::{BuildSpec, ContainerEngine, RunSpec};
use crate::machine::Machine;
use crate::mode::FpMode;
use crate::sample::{Campaign, Failure, Phase, Sample, SampleWriter, Stage};
use crate::shutdown;
use crate::workload::{self, Workload};

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

    // The workload as it will actually run: its declared params, with whatever the
    // command line overrode. A campaign records *this*, never the file — the file
    // will be edited, and these numbers will not change when it is.
    let declared = workloads(&args.benchmarks_dir)?
        .into_iter()
        .find(|root| root.workload.id == args.workload)
        .with_context(|| format!("no workload `{}`", args.workload))?
        .workload;
    let workload = declared.with_overrides(&workload::overrides(&args.params)?)?;

    // A campaign with no answer to check against has no correctness gate — only the
    // weaker claim that its backends agreed with each other, whatever they agreed on.
    // Both ways of ending up there are loud, and they are different mistakes: one is a
    // workload that never declared an answer, the other is a campaign that asked for
    // work the declared answer is not the answer to.
    match (declared.checksum, workload.checksum) {
        (Some(_), None) => warn!(
            workload = %workload.id,
            "params were overridden, so the workload's declared checksum does not apply to this \
             campaign — it is the answer to the declared work, not to this one. Correctness is \
             still enforced *within* the campaign: every backend must agree with the first one. \
             Nothing pins that agreement to a value from outside it. Publish from the declared \
             params.",
        ),
        (None, _) => warn!(
            workload = %workload.id,
            "this workload declares no `checksum`, so this campaign has NO correctness gate. It \
             can only establish that its backends agree with each other — which a campaign where \
             every one of them is wrong the same way passes, and which claims nothing at all \
             against any other campaign. A backend that computes nothing and returns instantly \
             would top this table. If the work is deterministic, declare the answer.",
        ),
        (Some(_), Some(_)) => {}
    }

    let implementations = discover(&args.benchmarks_dir, &args.workload)?;
    ensure!(
        !implementations.is_empty(),
        "the `{}` workload declares no implementation that exists on disk",
        workload.id,
    );

    let host = Architecture::current();
    let units = schedule(&implementations, &args.mode, host);
    ensure!(
        !units.is_empty(),
        // Two ways to schedule nothing, and they call for different fixes: a mode
        // nobody declares is a flag to change, an architecture nobody builds on is
        // a machine to change. Blaming `modes` for what the architecture did would send the
        // reader looking in the wrong file.
        "no implementation is buildable on {} in any of the requested modes ({}). Every \
         discovered implementation either declares a narrower `modes` list, or declares an \
         `architecture` list that does not include this machine.",
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
        workload: workload.clone(),
        cpu: args.cpu,
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
        strict_checksum: workload.checksum,
        reference_is_declared: workload.checksum.is_some(),
        workload,
        writer,
        written: 0,
        quarantined: HashSet::new(),
        failures: Vec::new(),
    };

    let campaign = (|| -> Result<()> {
        runner.prepare(&units)?;
        runner.measure_phase(&units, Phase::Build, args.build_rounds)?;
        runner.measure_phase(&units, Phase::Run, args.rounds)
    })();

    // The campaign produces the samples and stops there. The CSV and the report
    // are renderings, recomputed on demand from this file — including from a
    // campaign that was interrupted, which is exactly when you want them.
    let interrupted = match campaign {
        Ok(()) => {
            info!(
                samples = runner.written,
                quarantined = runner.failures.len(),
                path = %args.output.display(),
                "campaign complete; render it with `langbench report csv` or `langbench report md`",
            );
            false
        }
        // A signal is an answer, not a failure: exit 0, because the samples on
        // disk are as valid as they were a moment ago and the file renders. The
        // partial campaign is the user's to keep or discard — a non-zero exit
        // would say the harness broke, and it did not.
        Err(error) if shutdown::was_interrupted(&error) => {
            warn!(
                samples = runner.written,
                path = %args.output.display(),
                "campaign interrupted; the samples written so far are intact and \
                 render with `langbench report csv` or `langbench report md`",
            );
            true
        }
        Err(error) => return Err(error),
    };

    runner.report_failures();

    // Every unit failed: there is no campaign, only a list of things that broke.
    // That is the one unit failure the harness owns — a samples file with a header
    // and nothing under it renders into an empty table, and an empty table is a lie
    // told quietly. Anything short of that exits 0: the samples that *were*
    // measured are as valid as the backend beside them was broken.
    //
    // An interrupted campaign is exempt, and not as a courtesy: it wrote no sample
    // because it was told to stop, which is the one case where zero samples means
    // the harness did exactly what it was asked.
    ensure!(
        interrupted || runner.written > 0,
        "every unit of the campaign failed ({} of them); no sample was measured. The first \
         failure is usually the only one worth reading — a daemon that is not running, or an \
         image that does not build, fails identically for everyone.",
        runner.failures.len(),
    );
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
/// are simply not published for an architecture — Kotlin/Native has no `linux-aarch64`
/// host compiler — and the only ways to run one anyway are emulation, which this
/// project forbids, or cross-building, which would measure a build that never
/// happened here. So the manifest declares where it can be built, and a campaign
/// elsewhere drops the row *before* spending a `docker build` on discovering it.
fn schedule(
    implementations: &[Implementation],
    requested: &[FpMode],
    host: Option<Architecture>,
) -> Vec<Unit> {
    let mut units = Vec::new();
    for implementation in implementations {
        if !implementation.supports(host) {
            warn!(
                workload = %implementation.workload,
                language = %implementation.language,
                compiler = none_if_absent(implementation.compiler.as_deref()),
                interpreter = none_if_absent(implementation.interpreter.as_deref()),
                host = host.map_or("unknown", Architecture::as_str),
                declares = %implementation
                    .architectures
                    .iter()
                    .map(Architecture::to_string)
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
                    workload = %implementation.workload,
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
    /// The workload as it ran: its params are the kernels' `argv`, and its
    /// reference is what `strict` is checked against.
    workload: Workload,
    writer: SampleWriter,
    /// Samples are streamed to disk, never accumulated: the harness holds a
    /// count, and whoever renders reads them back.
    written: usize,
    /// The value every strict-mode run must agree on, bit for bit.
    ///
    /// Seeded from the workload's declared `checksum` when it has one — and
    /// then the *first* backend is checked against it, like every other. Without one,
    /// the first strict sample becomes the reference and the campaign can only
    /// establish that its backends agree with each other, which a campaign where they
    /// are all wrong the same way passes.
    strict_checksum: Option<u64>,
    /// Whether that reference came from the workload rather than from a run. It
    /// changes what a divergence *means*, so it changes what the error says.
    reference_is_declared: bool,
    /// The images of the units that failed, by [`Unit::image`] — the one token
    /// that is unique per `(implementation, mode)`.
    ///
    /// A unit is quarantined, never the campaign: a compiler that does not exist
    /// for this architecture, a kernel that segfaults, a run that hangs past the timeout,
    /// a checksum that diverges — each of those is one backend saying something
    /// about itself, and none of them is a reason to throw away the fifty
    /// measurements the other backends got right. A quarantined unit is dropped
    /// from every remaining round *and phase*: whatever broke in round one breaks
    /// in round nine too, and the campaign would spend an hour re-learning it.
    quarantined: HashSet<String>,
    failures: Vec<Failure>,
}

impl<E: ContainerEngine> Runner<'_, E> {
    /// Build every image. Never measured: this is where the network lives.
    ///
    /// An image that does not build takes its unit out of the campaign, and
    /// nothing else with it.
    fn prepare(&mut self, units: &[Unit]) -> Result<()> {
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
            let built = self.engine.build(&BuildSpec {
                image: unit.image.clone(),
                context: unit.implementation.context.clone(),
                build_args,
            });
            if let Err(error) = built {
                self.quarantine(unit, Stage::Prepare, None, None, error)?;
            }
        }
        Ok(())
    }

    /// Outer loop over rounds, inner loop over units. Never the reverse:
    /// blocking by implementation turns ambient noise into bias.
    fn measure_phase(&mut self, units: &[Unit], phase: Phase, rounds: u32) -> Result<()> {
        let total = self.args.warmup_rounds + rounds;
        let live = units
            .iter()
            .filter(|unit| !self.quarantined.contains(&unit.image))
            .count();
        info!(
            phase = phase.as_str(),
            rounds = total,
            units = live,
            "measuring",
        );

        for round in 0..total {
            let warmup = round < self.args.warmup_rounds;
            for unit in units {
                if self.quarantined.contains(&unit.image) {
                    continue;
                }
                // Between invocations, never inside one. A sample is written only
                // once it has been verified, so an interruption costs the run in
                // flight and nothing that came before it.
                shutdown::checkpoint()?;

                // A run that crashed, hung past the timeout, printed a record the
                // harness cannot read, or returned a checksum the workload does
                // not agree with, is a wrong run — and a wrong run never enters
                // the statistics. It writes no sample; it retires its unit.
                let measured = self
                    .measure(unit, phase, round, warmup)
                    .and_then(|sample| self.verify(&sample).map(|()| sample));
                let sample = match measured {
                    Ok(sample) => sample,
                    Err(error) => {
                        self.quarantine(unit, Stage::Measure, Some(phase), Some(round), error)?;
                        continue;
                    }
                };

                // The writer is the campaign's, not the unit's: a file that cannot
                // be appended to is the one failure quarantine cannot contain.
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
                    workload = %sample.workload,
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

    /// Take one unit out of the campaign, loudly, and leave the rest alone.
    ///
    /// The failure is written to `samples.ndjson` beside the samples, because the
    /// report and the website are pure functions of that file and this is the only
    /// place they can learn that a backend broke. It is a record, not a sample: no
    /// timing, nothing to aggregate.
    ///
    /// `Err` only for an interruption, which is nobody's fault and everybody's
    /// business: a signal must not be mistaken for a backend that misbehaved and
    /// quietly filed away as one — it stops the campaign.
    fn quarantine(
        &mut self,
        unit: &Unit,
        stage: Stage,
        phase: Option<Phase>,
        round: Option<u32>,
        error: anyhow::Error,
    ) -> Result<()> {
        // Propagated as-is, type intact: `execute` recognises an interruption by
        // the `Interrupted` in its chain, and a re-wrapped message would look
        // like a crash and exit non-zero.
        if shutdown::was_interrupted(&error) {
            return Err(error);
        }
        let implementation = &unit.implementation;
        let failure = Failure {
            workload: implementation.workload.clone(),
            language: implementation.language.clone(),
            compiler: implementation.compiler.clone(),
            interpreter: implementation.interpreter.clone(),
            description: implementation.description.clone(),
            comments: implementation.comments.clone(),
            mode: unit.mode,
            stage,
            phase,
            round,
            // The full context chain: `docker run` failed *while* measuring *this*
            // image, and the reader needs all three to know where to look.
            error: format!("{error:#}"),
        };
        error!(
            stage = stage.as_str(),
            workload = %failure.workload,
            language = %failure.language,
            compiler = none_if_absent(failure.compiler.as_deref()),
            interpreter = none_if_absent(failure.interpreter.as_deref()),
            mode = %failure.mode,
            error = %failure.error,
            "quarantining this backend for the rest of the campaign; the others carry on",
        );
        self.writer.write_failure(&failure)?;
        self.quarantined.insert(unit.image.clone());
        self.failures.push(failure);
        Ok(())
    }

    /// The quarantined units, once more, at the end.
    ///
    /// A failure logged an hour ago has scrolled off the terminal, and the report
    /// that follows will not mention the backend at all — a row that is absent
    /// looks exactly like a row that was never written. This is where the campaign
    /// says which backends it lost, and to what.
    fn report_failures(&self) {
        for failure in &self.failures {
            error!(
                stage = failure.stage.as_str(),
                workload = %failure.workload,
                language = %failure.language,
                compiler = none_if_absent(failure.compiler.as_deref()),
                interpreter = none_if_absent(failure.interpreter.as_deref()),
                mode = %failure.mode,
                error = %failure.error,
                "quarantined: this backend contributed no sample and is absent from the report",
            );
        }
    }

    fn measure(&self, unit: &Unit, phase: Phase, round: u32, warmup: bool) -> Result<Sample> {
        let execution = self.engine.run(&RunSpec {
            image: unit.image.clone(),
            args: self.container_args(phase),
            tmpfs_size_mb: self.args.tmpfs_size_mb,
            memory_limit_mb: self.args.memory_limit_mb,
            timeout: Duration::from_secs(self.args.run_timeout),
        })?;
        let record = execution.record;
        Ok(Sample {
            workload: unit.implementation.workload.clone(),
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
            peak_bytes: record.peak_bytes,
            // Off the manifest, not off the container: the source is a fact about
            // the implementation, and the image never sees it.
            source_bytes: Some(unit.implementation.source_bytes),
            checksum: record.checksum,
            binary_bytes: record.binary_bytes,
            binary_stripped_bytes: record.binary_stripped_bytes,
            text_bytes: record.text_bytes,
        })
    }

    /// What the container is run with: the phase, then the workload's params in
    /// declaration order, then the thread count.
    ///
    /// The params come from the workload and not from a flag, because how the work
    /// is sized is a property of the work — a grid and an iteration ceiling are
    /// Mandelbrot's business, and mean nothing to a workload that parses JSON. The
    /// thread count is the harness's: it is a property of the machine, resolved here
    /// and passed explicitly, because a kernel that auto-detects would be measuring
    /// its runtime's opinion of a cgroup quota.
    fn container_args(&self, phase: Phase) -> Vec<String> {
        match phase {
            Phase::Build => vec!["build".to_owned(), self.args.cpu.to_string()],
            Phase::Run => std::iter::once("run".to_owned())
                .chain(self.workload.argv())
                .chain(std::iter::once(self.args.cpu.to_string()))
                .collect(),
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
            Some(reference) if self.reference_is_declared => bail!(
                "strict-mode checksum mismatch on {}: {} produced {checksum}, but the `{}` \
                 workload declares the answer is {reference}. This backend is wrong — not slow, \
                 wrong — and a wrong run never enters the statistics. In strict mode every \
                 compiler, language and architecture agrees bit for bit; a divergence is a bug in \
                 the code or the flags, never a rounding difference.",
                sample.workload,
                sample.backend(),
                self.workload.id,
            ),
            Some(reference) => bail!(
                "strict-mode checksum mismatch on {}: {} produced {checksum}, and the backends \
                 before it produced {reference}. In strict mode every compiler, language and \
                 architecture must agree bit for bit; a divergence is a bug in the code or the \
                 flags, never a rounding difference. Note that `{}` declares no `checksum`, \
                 so the reference here is simply whichever backend ran first — declare one and the \
                 answer stops depending on the schedule.",
                sample.workload,
                sample.backend(),
                self.workload.id,
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
            peak_bytes: Some(4_194_304),
            binary_bytes: None,
            binary_stripped_bytes: None,
            text_bytes: None,
        }
    }

    /// The workload every fixture runs, unless it says otherwise.
    const WORKLOAD: &str = "mandelbrot";

    fn benchmarks(root: &Path, names: &[&str]) {
        benchmarks_for(root, WORKLOAD, names);
    }

    /// Backends are spelled as their slug — `c-gcc` — and written out as the
    /// manifest the harness actually reads.
    fn benchmarks_for(root: &Path, workload: &str, names: &[&str]) {
        for name in names {
            benchmark_declaring_for(root, workload, name, "all");
        }
    }

    fn benchmark_declaring(root: &Path, name: &str, modes: &str) {
        benchmark_declaring_for(root, WORKLOAD, name, modes);
    }

    /// `modes` is `all`, or the modes a manifest would list.
    fn benchmark_declaring_for(root: &Path, workload: &str, name: &str, modes: &str) {
        let (language, compiler) = name.split_once('-').expect("<language>-<compiler>");
        let modes = if modes == "all" {
            modes.to_owned()
        } else {
            format!("[{modes}]")
        };
        let dir = root.join(workload).join(name);
        create_dir_all(&dir).unwrap();
        File::create(dir.join("Dockerfile")).unwrap();
        // The manifest declares a source, and the source is there: discovery reads
        // its size onto every sample, and refuses a manifest that points at nothing.
        std::fs::write(dir.join(SOURCE), "// the fixture's kernel\n").unwrap();
        std::fs::write(
            dir.join(crate::discovery::MANIFEST),
            format!(
                "language: {language}\n\
                 compiler: {compiler}\n\
                 source: {SOURCE}\n\
                 modes: {modes}\n\
                 description: {name}, as the fixture declares it.\n",
            ),
        )
        .unwrap();

        declare_workload(root, workload, None);
    }

    /// (Re)write the workload manifest, declaring every implementation that exists
    /// beside it.
    ///
    /// The fixture is allowed to look at the directory; the harness is not. That is
    /// the whole point of the change these tests cover — a campaign reads the list a
    /// workload declares, so the fixture has to *build* that list, and it rebuilds it
    /// each time a benchmark is added so that tests can compose them freely.
    fn declare_workload(root: &Path, workload: &str, strict_checksum: Option<u64>) {
        let dir = root.join(workload);
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.join(crate::discovery::MANIFEST).is_file())
            .filter_map(|path| path.file_name()?.to_str().map(str::to_owned))
            .collect();
        names.sort();

        let mut yaml = format!(
            "id: {workload}\n\
             description: {workload}, as the fixture declares it.\n\
             params:\n\
             \x20 - name: grid_size\n\
             \x20   value: 64\n\
             \x20 - name: max_iter\n\
             \x20   value: 10\n",
        );
        if let Some(checksum) = strict_checksum {
            yaml.push_str(&format!("checksum: {checksum}\n"));
        }
        yaml.push_str("implementations:\n");
        for name in names {
            yaml.push_str(&format!("  - {name}\n"));
        }
        std::fs::write(dir.join(crate::workload::MANIFEST), yaml).unwrap();
    }

    /// The kernel every fixture declares, and its size on disk.
    const SOURCE: &str = "kernel.txt";
    const SOURCE_BYTES: u64 = 24;

    /// The two metrics come from two different places, and the sample is where they
    /// meet: the peak memory from inside the container, the source size from the
    /// manifest on disk. A sample must describe itself without a second file to join
    /// against — so both travel on the line, whatever produced them.
    #[test]
    fn a_sample_carries_its_memory_and_the_size_of_its_source() {
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
        args.warmup_rounds = 0;
        args.build_rounds = 0;
        args.rounds = 1;
        let output = args.output.clone();
        execute(args, &engine).unwrap();

        let recording = crate::sample::load(&output).unwrap();
        let sample = &recording.samples[0];
        assert_eq!(sample.peak_bytes, Some(4_194_304));
        assert_eq!(sample.source_bytes, Some(SOURCE_BYTES));
        assert_eq!(sample.wall_ns, 2_000);
        assert_eq!(sample.checksum, Some(42));
    }

    fn args(benchmarks_dir: &Path, output: &Path, modes: Vec<FpMode>) -> RunArgs {
        RunArgs {
            workload: WORKLOAD.to_owned(),
            params: Vec::new(),
            mode: modes,
            cpu: 4,
            output: output.join("samples.ndjson"),
            benchmarks_dir: benchmarks_dir.to_path_buf(),
            rounds: 2,
            build_rounds: 1,
            warmup_rounds: 1,
            march: "x86-64-v3".to_owned(),
            tmpfs_size_mb: 16,
            memory_limit_mb: 1024,
            run_timeout: 60,
        }
    }

    #[test]
    fn a_workload_that_declares_no_implementation_is_an_error() {
        let root = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        create_dir_all(root.path().join(WORKLOAD)).unwrap();
        declare_workload(root.path(), WORKLOAD, None);

        let engine = MockContainerEngine::new();
        let err =
            execute(args(root.path(), out.path(), vec![FpMode::Strict]), &engine).unwrap_err();
        assert!(
            err.to_string().contains("declares no implementation"),
            "{err}"
        );
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
        assert!(err.to_string().contains("architecture"), "{err}");
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

    /// A divergence is a bug in the backend, and the backend is what it costs:
    /// the campaign keeps every other row it was measuring.
    #[test]
    fn a_strict_mode_checksum_divergence_quarantines_the_backend_not_the_campaign() {
        let root = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        benchmarks(root.path(), &["c-gcc", "c-clang"]);

        let mut engine = MockContainerEngine::new();
        engine.expect_build().returning(|_| Ok(()));
        // gcc lands first and sets the reference; clang disagrees, every time.
        engine.expect_run().returning(|spec| {
            let checksum = if spec.image.contains("clang") { 9 } else { 7 };
            Ok(Execution {
                wall_ns: 2_000,
                record: record(Some(checksum)),
            })
        });

        let mut args = args(root.path(), out.path(), vec![FpMode::Strict]);
        args.warmup_rounds = 0;
        args.build_rounds = 0;
        args.rounds = 3;
        let output = args.output.clone();
        execute(args, &engine).unwrap();

        let recording = crate::sample::load(&output).unwrap();

        // Whichever of the two ran first is the reference — the harness cannot know
        // which of two disagreeing compilers is the wrong one, and does not pretend
        // to. What it *can* guarantee is that they never end up in the same table:
        // three rounds of one backend, nothing at all from the other.
        assert_eq!(recording.samples.len(), 3);
        let survivor = recording.samples[0].backend();
        assert!(
            recording
                .samples
                .iter()
                .all(|sample| sample.backend() == survivor),
        );

        // And the campaign says so, in the file — the report reads the reason from
        // here, not from a log line that scrolled past an hour ago.
        assert_eq!(recording.failures.len(), 1);
        let failure = &recording.failures[0];
        assert_ne!(failure.backend(), survivor);
        assert_eq!(failure.stage, Stage::Measure);
        assert!(failure.error.contains("checksum mismatch"), "{failure:?}");
    }

    #[test]
    fn two_algorithms_are_each_verified_against_their_own_reference() {
        // The checksum is a property of (workload, grid size, max_iter), so two
        // workloads legitimately disagree. A campaign-wide reference would abort
        // on the first strict run of the second workload.
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

    /// A backend whose toolchain does not exist for this architecture is dropped from the
    /// schedule *before* a `docker build` discovers it the hard way — and the drop
    /// is loud, because a row that silently vanishes from a report is worse than a
    /// row that failed.
    #[test]
    fn an_implementation_is_skipped_on_an_architecture_it_cannot_build_on() {
        let root = TempDir::new().unwrap();
        let dir = root.path().join("mandelbrot").join("kotlin-kotlin-native");
        create_dir_all(&dir).unwrap();
        File::create(dir.join("Dockerfile")).unwrap();
        std::fs::write(dir.join(SOURCE), "// the fixture's kernel\n").unwrap();
        std::fs::write(
            dir.join(crate::discovery::MANIFEST),
            format!(
                "language: kotlin\n\
                 compiler: kotlin-native\n\
                 source: {SOURCE}\n\
                 modes: [strict]\n\
                 architectures: [x86_64]\n\
                 description: No linux-aarch64 host compiler exists.\n",
            ),
        )
        .unwrap();
        benchmarks(root.path(), &["c-gcc"]);

        let implementations = discover(root.path(), WORKLOAD).unwrap();
        assert_eq!(implementations.len(), 2);

        // On x86-64 both are scheduled; on AArch64 only the C one survives.
        let on_x86 = schedule(
            &implementations,
            &[FpMode::Strict],
            Some(Architecture::X86_64),
        );
        assert_eq!(on_x86.len(), 2);

        let on_arm = schedule(
            &implementations,
            &[FpMode::Strict],
            Some(Architecture::Aarch64),
        );
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

    /// One backend crashing is one backend's news. The campaign goes on, and the
    /// rows it *could* measure are still worth the hour it spent measuring them.
    #[test]
    fn a_crashing_backend_is_quarantined_and_the_others_carry_on() {
        let root = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        benchmarks(root.path(), &["c-gcc", "rust-llvm"]);

        let mut engine = MockContainerEngine::new();
        engine.expect_build().returning(|_| Ok(()));
        engine.expect_run().returning(|spec| {
            if spec.image.contains("rust") {
                bail!(
                    "`docker run` failed for {}:\nSegmentation fault",
                    spec.image
                );
            }
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

        // Exit 0: the harness did not break, one of the things it measures did.
        execute(args, &engine).unwrap();

        let recording = crate::sample::load(&output).unwrap();
        assert_eq!(recording.samples.len(), 4, "four rounds of the C backend");
        assert_eq!(recording.failures.len(), 1, "the crash is recorded once");
        assert_eq!(recording.failures[0].backend(), "rust-llvm");
        assert_eq!(recording.failures[0].round, Some(0));
    }

    /// A quarantined unit is not retried: whatever broke in round one breaks in
    /// round nine, and re-learning it costs the campaign an hour of wall-clock.
    #[test]
    fn a_quarantined_unit_is_never_run_again() {
        let root = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        benchmarks(root.path(), &["c-gcc", "rust-llvm"]);

        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);

        let mut engine = MockContainerEngine::new();
        engine.expect_build().returning(|_| Ok(()));
        engine.expect_run().returning(move |spec| {
            recorded.lock().unwrap().push(spec.image.clone());
            if spec.image.contains("rust") {
                bail!("`docker run` failed");
            }
            Ok(Execution {
                wall_ns: 2_000,
                record: record(Some(42)),
            })
        });

        let mut args = args(root.path(), out.path(), vec![FpMode::Strict]);
        args.warmup_rounds = 0;
        // The build phase kills it, and the run phase must not resurrect it.
        args.build_rounds = 2;
        args.rounds = 2;
        execute(args, &engine).unwrap();

        let images = seen.lock().unwrap().clone();
        let rust = images.iter().filter(|image| image.contains("rust")).count();
        assert_eq!(rust, 1, "the broken unit is asked exactly once: {images:?}");
    }

    /// An image that does not build takes its unit out of the campaign, and only
    /// its unit: the others are already queued behind it.
    #[test]
    fn an_image_that_does_not_build_is_quarantined_before_it_is_ever_run() {
        let root = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        benchmarks(root.path(), &["c-gcc", "rust-llvm"]);

        let mut engine = MockContainerEngine::new();
        engine.expect_build().returning(|spec| {
            if spec.image.contains("rust") {
                bail!("`docker build` failed for {}", spec.image);
            }
            Ok(())
        });
        // Never run: the image does not exist. Only the C unit reaches the daemon.
        engine.expect_run().times(2).returning(|spec| {
            assert!(spec.image.contains("c-gcc"), "ran a unit that never built");
            Ok(Execution {
                wall_ns: 2_000,
                record: record(Some(42)),
            })
        });

        let mut args = args(root.path(), out.path(), vec![FpMode::Strict]);
        args.warmup_rounds = 0;
        args.build_rounds = 0;
        args.rounds = 2;
        let output = args.output.clone();
        execute(args, &engine).unwrap();

        let recording = crate::sample::load(&output).unwrap();
        assert_eq!(recording.failures.len(), 1);
        assert_eq!(recording.failures[0].stage, Stage::Prepare);
        // It never got a round: there was no round to fail in.
        assert_eq!(recording.failures[0].round, None);
    }

    /// Quarantine is per `(implementation, mode)`, never per implementation: a
    /// backend whose `fast` build is broken still has a `strict` one to publish.
    #[test]
    fn quarantine_takes_the_unit_and_not_the_whole_implementation() {
        let root = TempDir::new().unwrap();
        let out = TempDir::new().unwrap();
        benchmarks(root.path(), &["c-gcc"]);

        let mut engine = MockContainerEngine::new();
        engine.expect_build().returning(|spec| {
            if spec.image.ends_with(":fast") {
                bail!("`docker build` failed: -ffast-math is not a flag this fixture likes");
            }
            Ok(())
        });
        engine.expect_run().returning(|_| {
            Ok(Execution {
                wall_ns: 2_000,
                record: record(Some(42)),
            })
        });

        let mut args = args(root.path(), out.path(), vec![FpMode::Strict, FpMode::Fast]);
        args.warmup_rounds = 0;
        args.build_rounds = 0;
        args.rounds = 1;
        let output = args.output.clone();
        execute(args, &engine).unwrap();

        let recording = crate::sample::load(&output).unwrap();
        assert_eq!(recording.samples.len(), 1);
        assert_eq!(recording.samples[0].mode, FpMode::Strict);
        assert_eq!(recording.failures.len(), 1);
        assert_eq!(recording.failures[0].mode, FpMode::Fast);
    }

    /// Quarantine is not a way to smile through a campaign that measured nothing.
    /// A samples file with a header and no sample renders into an empty table, and
    /// an empty table is a lie told quietly.
    #[test]
    fn a_campaign_where_every_unit_failed_still_fails() {
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
        assert!(error.to_string().contains("every unit"), "{error}");
    }
}
