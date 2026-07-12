//! The binary. Every module it drives lives behind the `cli` feature of the
//! library beside it — see `src/lib.rs` for why the crate is split at all.

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use langbench::cli::{Cli, Command};
use langbench::engine::DockerEngine;
use langbench::machine::Machine;
use langbench::{discovery, output, runner, shutdown};

fn main() -> Result<()> {
    let cli = Cli::parse();
    // Diagnostics on stderr. Stdout carries program output alone — the machine
    // report, the built-in template — so it stays pipeable.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&cli.log_filter))
        .with_writer(std::io::stderr)
        .init();

    // Armed before the first container exists. `run` is the command that starts
    // them, but the others are cheap and a handler that is always on cannot be
    // the one that was forgotten.
    shutdown::install()?;

    match cli.command {
        Command::Run(args) => runner::execute(args, &DockerEngine::new()),
        Command::Csv(args) => output::csv(&args),
        Command::Md(args) => output::markdown(&args),
        Command::Validate(args) => discovery::validate(&args.paths).map(|_| ()),
        Command::Jsonschema(args) => output::jsonschema(&args),
        Command::Machine => {
            // Program output, not a diagnostic: stdout, not `tracing`.
            print!("{}", Machine::collect().console_report());
            Ok(())
        }
    }
}
