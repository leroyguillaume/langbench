//! `docker build` prepares. `docker run` measures.
//!
//! See `METHODOLOGY.md#measurement-protocol`.

use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use tracing::debug;

use crate::sample::{ContainerRecord, parse_container_stdout};

/// Where the container compiles. A tmpfs, so codegen output never touches
/// overlayfs, and so the tree is empty on every run without a cleanup step.
const BUILD_DIR: &str = "/build";

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
        let output = Command::new("docker")
            .args(&args)
            .output()
            .context("spawning `docker build`")?;
        if !output.status.success() {
            bail!(
                "`docker build` failed for {}:\n{}",
                spec.image,
                String::from_utf8_lossy(&output.stderr),
            );
        }
        Ok(())
    }

    fn run(&self, spec: &RunSpec) -> Result<Execution> {
        let args = run_args(spec);
        debug!(image = %spec.image, ?args, "docker run");

        let started = Instant::now();
        let output = Command::new("docker")
            .args(&args)
            .output()
            .context("spawning `docker run`")?;
        let wall_ns = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);

        if !output.status.success() {
            bail!(
                "`docker run` failed for {}:\n{}",
                spec.image,
                String::from_utf8_lossy(&output.stderr),
            );
        }
        let record = parse_container_stdout(&String::from_utf8_lossy(&output.stdout))
            .with_context(|| format!("reading the record printed by {}", spec.image))?;
        Ok(Execution { wall_ns, record })
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

pub fn run_args(spec: &RunSpec) -> Vec<String> {
    let mut args = vec![
        "run".to_owned(),
        "--rm".to_owned(),
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
        let args = run_args(&RunSpec {
            image: "img".to_owned(),
            args: vec!["run".to_owned(), "4096".to_owned()],
            tmpfs_size_mb: 2048,
        });
        assert!(args.contains(&"--network=none".to_owned()));
        assert!(args.contains(&"--rm".to_owned()));
    }

    #[test]
    fn the_build_directory_is_a_sized_tmpfs() {
        let args = run_args(&RunSpec {
            image: "img".to_owned(),
            args: vec![],
            tmpfs_size_mb: 512,
        });
        let index = args.iter().position(|a| a == "--tmpfs").unwrap();
        assert_eq!(args[index + 1], "/build:rw,exec,size=512m");
    }

    #[test]
    fn container_arguments_come_after_the_image() {
        let args = run_args(&RunSpec {
            image: "img".to_owned(),
            args: vec!["run".to_owned(), "4096".to_owned(), "1000".to_owned()],
            tmpfs_size_mb: 1,
        });
        let image = args.iter().position(|a| a == "img").unwrap();
        assert_eq!(&args[image + 1..], ["run", "4096", "1000"]);
    }
}
