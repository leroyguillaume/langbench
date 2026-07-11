mod cli;
mod discovery;
mod engine;
mod machine;
mod output;
mod report;
mod runner;
mod sample;
mod stats;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::cli::{Cli, Command};
use crate::engine::DockerEngine;
use crate::machine::Machine;

fn main() -> Result<()> {
    let cli = Cli::parse();
    // Diagnostics on stderr. Stdout carries program output alone — the machine
    // report, the built-in template — so it stays pipeable.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&cli.log_filter))
        .with_writer(std::io::stderr)
        .init();

    match cli.command {
        Command::Run(args) => runner::execute(args, &DockerEngine),
        Command::Csv(args) => output::csv(&args),
        Command::Md(args) => output::markdown(&args),
        Command::Machine => {
            // Program output, not a diagnostic: stdout, not `tracing`.
            print!("{}", Machine::collect().console_report());
            Ok(())
        }
    }
}
