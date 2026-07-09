mod cli;
mod discovery;
mod engine;
mod machine;
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
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&cli.log_filter))
        .init();

    match cli.command {
        Command::Run(args) => runner::execute(args, &DockerEngine),
        Command::Machine => {
            // Program output, not a diagnostic: stdout, not `tracing`.
            print!("{}", Machine::collect().console_report());
            Ok(())
        }
    }
}
