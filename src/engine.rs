//! `docker build` prepares. `docker run` measures.
//!
//! See `METHODOLOGY.md#measurement-protocol`.

use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use tracing::debug;

use crate::sample::{ContainerRecord, parse_container_stdout};
use crate::shutdown;

/// Where the container compiles. A tmpfs, so codegen output never touches
/// overlayfs, and so the tree is empty on every run without a cleanup step.
const BUILD_DIR: &str = "/build";

/// Distinguishes concurrent campaigns on the same daemon. Not random: the
/// harness is sequential and a PID plus a counter is already unique.
static INVOCATION: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildSpec {
    pub image: String,
    pub context: PathBuf,
    pub build_args: Vec<(String, String)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunSpec {
    pub image: String,
    pub args: Vec<String>,
    pub tmpfs_size_mb: u64,
    /// A wall-clock ceiling for one invocation. A container that exceeds it is
    /// killed and the campaign fails: a hung run must not be mistaken for a slow
    /// one, and the harness has no other way to tell them apart.
    pub timeout: Duration,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Execution {
    /// External wall-clock, measured around the child process.
    pub wall_ns: u64,
    pub record: ContainerRecord,
}

#[cfg_attr(test, mockall::automock)]
pub trait ContainerEngine {
    /// Prepare an image. Never measured: this is where the network lives.
    fn build(&self, spec: &BuildSpec) -> Result<()>;

    /// Run a prepared image and time it from the outside.
    fn run(&self, spec: &RunSpec) -> Result<Execution>;
}

pub struct DockerEngine;

impl ContainerEngine for DockerEngine {
    fn build(&self, spec: &BuildSpec) -> Result<()> {
        let args = build_args(spec);
        debug!(image = %spec.image, ?args, "docker build");
        let mut child = Command::new("docker")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("spawning `docker build`")?;

        // Piped, so drained: a compiler's stderr can outgrow the pipe buffer, and
        // a full pipe would deadlock the child against our wait.
        let stderr = drain(child.stderr.take().expect("stderr is piped"));
        let stdout = drain(child.stdout.take().expect("stdout is piped"));

        // Polled rather than waited on, for the same reason as a measured run: a
        // build is minutes long, and a signal that is only noticed when it ends
        // is a signal Docker has already escalated to `SIGKILL`.
        loop {
            if let Some(status) = child.try_wait().context("waiting on `docker build`")? {
                let stderr = stderr.join().unwrap_or_default();
                let _ = stdout.join();
                if !status.success() {
                    bail!("`docker build` failed for {}:\n{stderr}", spec.image);
                }
                return Ok(());
            }
            if shutdown::requested() {
                // Only the client is ours to kill. Unlike a run, a build has no
                // name to reach it by — BuildKit cancels the build when the
                // client disconnects, and the legacy builder finishes the step it
                // is on. Neither is measured, and both are bounded.
                let _ = child.kill();
                let _ = child.wait();
                return Err(anyhow::Error::from(shutdown::Interrupted)
                    .context(format!("building {}", spec.image)));
            }
            thread::sleep(shutdown::TICK);
        }
    }

    fn run(&self, spec: &RunSpec) -> Result<Execution> {
        let name = container_name();
        let args = run_args(spec, &name);
        debug!(image = %spec.image, %name, ?args, "docker run");

        let started = Instant::now();
        let mut child = Command::new("docker")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("spawning `docker run`")?;

        // Drain both pipes concurrently: a compiler's stderr can outgrow the pipe
        // buffer, and a full pipe would deadlock the child against our wait.
        let stdout = drain(child.stdout.take().expect("stdout is piped"));
        let stderr = drain(child.stderr.take().expect("stderr is piped"));

        let (status, wall_ns) = wait_or_kill(child, &name, started, spec.timeout)?;
        let stdout = stdout.join().unwrap_or_default();
        let stderr = stderr.join().unwrap_or_default();

        if !status.success() {
            bail!("`docker run` failed for {}:\n{}", spec.image, stderr);
        }
        let record = parse_container_stdout(&stdout)
            .with_context(|| format!("reading the record printed by {}", spec.image))?;
        Ok(Execution { wall_ns, record })
    }
}

fn container_name() -> String {
    let invocation = INVOCATION.fetch_add(1, Ordering::Relaxed);
    format!("langbench-{}-{invocation}", std::process::id())
}

fn drain(mut pipe: impl Read + Send + 'static) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut buffer = String::new();
        let _ = pipe.read_to_string(&mut buffer);
        buffer
    })
}

/// Wait for the container, or kill it and fail.
///
/// The wall-clock is stamped in the waiting thread, the instant `wait` returns,
/// so it never picks up the latency of noticing that the child has exited.
///
/// Killing the `docker` client would **not** stop the container: the workload
/// runs on the daemon, in another process tree, and would keep burning CPU with
/// nobody watching. The container is named for exactly this reason.
fn wait_or_kill(
    mut child: Child,
    name: &str,
    started: Instant,
    timeout: Duration,
) -> Result<(std::process::ExitStatus, u64)> {
    let (sender, receiver) = mpsc::channel();
    let waiter = thread::spawn(move || {
        let status = child.wait();
        let elapsed = started.elapsed();
        let _ = sender.send((status, elapsed));
    });

    // Tick rather than block for the whole timeout: a signal has to be noticed
    // while the container is still running, and a `--run-timeout` of ten minutes
    // is ten minutes of not looking. The wall-clock is stamped in the waiter the
    // instant `wait` returns, so how often we look here never reaches the
    // measurement.
    let deadline = started + timeout;
    loop {
        match receiver.recv_timeout(shutdown::TICK) {
            Ok((status, elapsed)) => {
                let _ = waiter.join();
                let status = status.context("waiting on `docker run`")?;
                return Ok((
                    status,
                    u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX),
                ));
            }
            Err(RecvTimeoutError::Timeout) => {
                // The signal is the caller's, but the container is ours: nobody
                // else knows its name, so nobody else can stop it.
                if shutdown::requested() {
                    let killed = kill_container(name);
                    let _ = waiter.join();
                    return Err(anyhow::Error::from(shutdown::Interrupted).context(format!(
                        "container `{name}` killed on the way out ({killed})",
                    )));
                }
                if Instant::now() >= deadline {
                    let killed = kill_container(name);
                    // The client exits once the daemon reaps the container.
                    let _ = waiter.join();
                    bail!(
                        "`docker run` exceeded the {} s timeout and container `{name}` was \
                         killed ({killed}). A hung run is not a slow run: raise --run-timeout \
                         if the workload is genuinely this slow, or look for a deadlock.",
                        timeout.as_secs(),
                    )
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                let _ = waiter.join();
                bail!("the thread waiting on `docker run` died")
            }
        }
    }
}

/// Stop the container itself, by name — never merely its client.
///
/// The workload runs on the daemon, in another process tree. Killing the
/// `docker` client would leave it running with nobody watching, which is the
/// whole reason the container is named.
fn kill_container(name: &str) -> String {
    match Command::new("docker").args(["kill", name]).output() {
        Ok(output) if output.status.success() => "kill succeeded".to_owned(),
        Ok(output) => format!(
            "kill failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
        Err(error) => format!("kill could not run: {error}"),
    }
}

pub fn build_args(spec: &BuildSpec) -> Vec<String> {
    let mut args = vec!["build".to_owned()];
    for (key, value) in &spec.build_args {
        args.push("--build-arg".to_owned());
        args.push(format!("{key}={value}"));
    }
    args.push("--tag".to_owned());
    args.push(spec.image.clone());
    args.push(spec.context.display().to_string());
    args
}

pub fn run_args(spec: &RunSpec, name: &str) -> Vec<String> {
    let mut args = vec![
        "run".to_owned(),
        "--rm".to_owned(),
        // Named so a timeout can kill the container rather than just its client.
        "--name".to_owned(),
        name.to_owned(),
        // Not "hopefully no network": a build that tries to fetch fails loudly
        // instead of silently adding four seconds to the measurement.
        "--network=none".to_owned(),
        "--tmpfs".to_owned(),
        format!("{BUILD_DIR}:rw,exec,size={}m", spec.tmpfs_size_mb),
    ];
    args.push(spec.image.clone());
    args.extend(spec.args.iter().cloned());
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(args: &[&str], tmpfs_size_mb: u64) -> RunSpec {
        RunSpec {
            image: "img".to_owned(),
            args: args.iter().map(|arg| (*arg).to_owned()).collect(),
            tmpfs_size_mb,
            timeout: Duration::from_secs(60),
        }
    }

    #[test]
    fn build_args_carry_every_build_arg_then_the_context() {
        let args = build_args(&BuildSpec {
            image: "langbench/mandelbrot-c-gcc:strict".to_owned(),
            context: PathBuf::from("benchmarks/mandelbrot/c-gcc"),
            build_args: vec![
                ("FP_MODE".to_owned(), "strict".to_owned()),
                ("MARCH".to_owned(), "x86-64-v3".to_owned()),
            ],
        });
        assert_eq!(
            args,
            [
                "build",
                "--build-arg",
                "FP_MODE=strict",
                "--build-arg",
                "MARCH=x86-64-v3",
                "--tag",
                "langbench/mandelbrot-c-gcc:strict",
                "benchmarks/mandelbrot/c-gcc",
            ]
        );
    }

    #[test]
    fn a_measured_run_is_always_isolated_from_the_network() {
        let args = run_args(&spec(&["run", "4096"], 2048), "langbench-1-0");
        assert!(args.contains(&"--network=none".to_owned()));
        assert!(args.contains(&"--rm".to_owned()));
    }

    #[test]
    fn a_measured_run_is_named_so_a_timeout_can_kill_the_container() {
        let args = run_args(&spec(&[], 1), "langbench-42-7");
        let index = args.iter().position(|arg| arg == "--name").unwrap();
        assert_eq!(args[index + 1], "langbench-42-7");
    }

    #[test]
    fn the_build_directory_is_a_sized_tmpfs() {
        let args = run_args(&spec(&[], 512), "n");
        let index = args.iter().position(|arg| arg == "--tmpfs").unwrap();
        assert_eq!(args[index + 1], "/build:rw,exec,size=512m");
    }

    #[test]
    fn container_arguments_come_after_the_image() {
        let args = run_args(&spec(&["run", "4096", "1000"], 1), "n");
        let image = args.iter().position(|arg| arg == "img").unwrap();
        assert_eq!(&args[image + 1..], ["run", "4096", "1000"]);
    }

    #[test]
    fn container_names_are_unique_across_invocations() {
        assert_ne!(container_name(), container_name());
    }
}
