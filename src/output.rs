//! Rendering a recorded campaign: `langbench csv` and `langbench md`.
//!
//! Neither command measures anything. They read a `samples.ndjson` and are pure
//! functions of it, which is what makes a report reproducible: the same file
//! always renders the same table, on any host, months later.
//!
//! Everything goes to stdout. `OUTPUT` names the campaign — the same value `run`
//! wrote to — and a rendering that could also *write* to `OUTPUT` would give one
//! name two opposite meanings. Redirect instead: `langbench md > report.md`.

use std::fs;
use std::io::Write;

use anyhow::{Context, Result};

use crate::cli::{MarkdownArgs, RenderArgs};
use crate::report;
use crate::sample;

pub fn csv(args: &RenderArgs) -> Result<()> {
    let recording = sample::load(&args.samples)?;
    emit(&sample::to_csv(&recording.samples))
}

pub fn markdown(args: &MarkdownArgs) -> Result<()> {
    if args.print_template {
        return emit(report::DEFAULT_TEMPLATE);
    }

    let template = match &args.template {
        None => report::DEFAULT_TEMPLATE.to_owned(),
        Some(path) => fs::read_to_string(path)
            .with_context(|| format!("reading the template {}", path.display()))?,
    };
    let recording = sample::load(&args.render.samples)?;
    emit(&report::render(&report::build(&recording), &template)?)
}

fn emit(content: &str) -> Result<()> {
    std::io::stdout()
        .write_all(content.as_bytes())
        .context("writing to stdout")
}
