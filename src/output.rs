//! Rendering a recorded campaign: `langbench csv` and `langbench md`.
//!
//! Neither command measures anything. They read a `samples.ndjson` and are pure
//! functions of it, which is what makes a report reproducible: the same file
//! always renders the same table, on any host, months later.
//!
//! Each rendering is a file, not a stream: `SAMPLES_OUTPUT` names the campaign —
//! the same value `run` wrote to — while `CSV_OUTPUT` and `MD_OUTPUT` name the
//! artifacts rendered from it. Three names, three meanings, no redirection to
//! remember, and nothing on stdout.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::cli::{CsvArgs, MarkdownArgs};
use crate::report;
use crate::sample;

pub fn csv(args: &CsvArgs) -> Result<()> {
    let recording = sample::load(&args.render.samples)?;
    write(&args.output, &sample::to_csv(&recording.samples))
}

pub fn markdown(args: &MarkdownArgs) -> Result<()> {
    let template = match &args.template {
        None => report::DEFAULT_TEMPLATE.to_owned(),
        Some(path) => fs::read_to_string(path)
            .with_context(|| format!("reading the template {}", path.display()))?,
    };
    let recording = sample::load(&args.render.samples)?;
    write(
        &args.output,
        &report::render(&report::build(&recording), &template)?,
    )
}

/// Write a rendering, creating the directories it is addressed into.
fn write(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(path, content).with_context(|| format!("writing {}", path.display()))?;
    tracing::info!(path = %path.display(), bytes = content.len(), "rendered");
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::cli::{FpMode, RenderArgs};
    use crate::machine::Machine;
    use crate::sample::{Campaign, Phase, Sample, SampleWriter};

    /// A one-sample campaign on disk, as `run` would have left it.
    fn campaign(dir: &TempDir) -> RenderArgs {
        let samples = dir.path().join("samples.ndjson");
        let mut writer = SampleWriter::create(&samples).unwrap();
        writer
            .write_header(
                &Machine::default(),
                &Campaign {
                    langbench_version: "0.1.0".to_owned(),
                    timestamp: "2026-07-11T12:00:00Z".to_owned(),
                    cpu: 8,
                    grid_size: 2048,
                    max_iter: 1000,
                    rounds: 10,
                    build_rounds: 3,
                    warmup_rounds: 1,
                    march: "x86-64-v3".to_owned(),
                    modes: vec!["strict".to_owned()],
                },
            )
            .unwrap();
        writer
            .write_sample(&Sample {
                algo: "mandelbrot".to_owned(),
                implementation: "c-gcc".to_owned(),
                language: "c".to_owned(),
                compiler: "gcc".to_owned(),
                mode: FpMode::Strict,
                phase: Phase::Run,
                round: 1,
                warmup: false,
                cpu: 8,
                wall_ns: 313_600_000,
                elapsed_ns: 213_300_000,
                user_usec: 860_000,
                system_usec: 4_000,
                checksum: Some(42),
                binary_bytes: None,
                binary_stripped_bytes: None,
                text_bytes: None,
            })
            .unwrap();
        RenderArgs { samples }
    }

    #[test]
    fn csv_renders_to_its_own_file_and_not_to_stdout() {
        let dir = TempDir::new().unwrap();
        let output = dir.path().join("out/samples.csv");
        csv(&CsvArgs {
            render: campaign(&dir),
            output: output.clone(),
        })
        .unwrap();

        let rendered = fs::read_to_string(&output).unwrap();
        assert!(rendered.starts_with("algo,"), "{rendered}");
        assert!(rendered.contains("mandelbrot,c-gcc"), "{rendered}");
    }

    #[test]
    fn markdown_renders_to_its_own_file_and_not_to_stdout() {
        let dir = TempDir::new().unwrap();
        let output = dir.path().join("out/report.md");
        markdown(&MarkdownArgs {
            render: campaign(&dir),
            output: output.clone(),
            template: None,
        })
        .unwrap();

        let rendered = fs::read_to_string(&output).unwrap();
        assert!(rendered.contains("mandelbrot"), "{rendered}");
    }
}
