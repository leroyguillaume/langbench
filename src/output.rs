//! Reading a recorded campaign back: `langbench sample convert`.
//!
//! It measures nothing. It reads a `samples.ndjson` and is a pure function of it,
//! which is what makes a conversion reproducible: the same file always converts to
//! the same table, on any host, months later.
//!
//! **It aggregates nothing either.** One row per sample, the columns the samples
//! carry — because the human rendering of a campaign is the website, and the website
//! recomputes min-of-N and the rest from these very samples with the harness's own
//! code compiled to WebAssembly. A second aggregator here would be a second
//! definition of what this project measures, and the two would drift the first time
//! one of them was fixed.
//!
//! The conversion is a file, not a stream: `SAMPLES_OUTPUT` names the campaign — the
//! same value `run` wrote to — while `CONVERT_OUTPUT` names what comes out of it. Two
//! names, two meanings, no redirection to remember, and nothing on stdout.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::cli::{
    ConvertArgs, DEFAULT_BENCH_SCHEMA_OUTPUT, DEFAULT_WORKLOAD_SCHEMA_OUTPUT,
    ImplementationListArgs, JsonSchemaArgs, ListArgs, SampleFormat,
};
use crate::discovery;
use crate::sample;
use crate::workload::{self, Workload};

pub fn convert(args: &ConvertArgs) -> Result<()> {
    let recording = sample::load(&args.samples)?;
    let converted = match args.format {
        SampleFormat::Csv => sample::to_csv(&recording.samples),
    };
    write(&args.output(), &converted)
}

/// A manifest schema, written where an editor and the pre-commit hook expect it.
/// Not a rendering of a campaign, but the same contract: a pure function of the
/// code, on stdout-free output.
///
/// Two manifests, two schemas, one per resource — and both generated from the very
/// struct the harness deserializes, so neither can drift from what is actually
/// accepted.
pub fn bench_schema(args: &JsonSchemaArgs) -> Result<()> {
    // Trailing newline: the file is checked in, and `end-of-file-fixer` would
    // otherwise rewrite what this command just wrote, failing the hook forever.
    write(
        &args.output_or(DEFAULT_BENCH_SCHEMA_OUTPUT),
        &format!("{}\n", discovery::schema()?),
    )
}

pub fn workload_schema(args: &JsonSchemaArgs) -> Result<()> {
    write(
        &args.output_or(DEFAULT_WORKLOAD_SCHEMA_OUTPUT),
        &format!("{}\n", workload::schema()?),
    )
}

/// The workloads on disk: what could be measured, before anything is.
///
/// A pure function of the tree — no Docker, no samples. It is the answer to "what
/// can I run?", and `--json` is there because the answer is read by scripts as often
/// as by people.
pub fn list_workloads(args: &ListArgs) -> Result<()> {
    let roots = discovery::workloads(&args.benchmarks_dir)?;

    if args.json {
        let workloads: Vec<&Workload> = roots.iter().map(|root| &root.workload).collect();
        println!("{}", serde_json::to_string_pretty(&workloads)?);
        return Ok(());
    }

    for root in &roots {
        let params: Vec<String> = root
            .workload
            .params
            .iter()
            .map(|param| format!("{}={}", param.name, param.value))
            .collect();
        println!(
            "{}\n  {}\n  implementations: {}\n  params: {}\n",
            root.workload.id,
            root.workload.description,
            root.workload.implementations.len(),
            params.join(" "),
        );
    }
    Ok(())
}

/// The implementations of one workload: the backends that race to do the work.
pub fn list_implementations(args: &ImplementationListArgs) -> Result<()> {
    let implementations = discovery::discover(&args.list.benchmarks_dir, &args.workload)?;

    if args.list.json {
        println!("{}", serde_json::to_string_pretty(&implementations)?);
        return Ok(());
    }

    for implementation in &implementations {
        println!(
            "{}\n  language: {}\n  compiler: {}\n  interpreter: {}\n  modes: {}\n",
            implementation.slug(),
            implementation.language,
            implementation.compiler.as_deref().unwrap_or("n/a"),
            implementation.interpreter.as_deref().unwrap_or("n/a"),
            implementation
                .fp_modes
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    Ok(())
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
    use std::path::PathBuf;

    use crate::workload::Workload;
    use tempfile::TempDir;

    use super::*;
    use crate::machine::Machine;
    use crate::mode::FpMode;
    use crate::sample::{Campaign, Phase, Sample, SampleWriter};

    /// A one-sample campaign on disk, as `run` would have left it.
    fn campaign(dir: &TempDir) -> PathBuf {
        let samples = dir.path().join("samples.ndjson");
        let mut writer = SampleWriter::create(&samples).unwrap();
        writer
            .write_header(
                &Machine::default(),
                &Campaign {
                    langbench_version: "0.1.0".to_owned(),
                    timestamp: "2026-07-11T12:00:00Z".to_owned(),
                    cpu: 8,
                    workload: Workload::fixture(),
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
                workload: "mandelbrot".to_owned(),
                language: "c".to_owned(),
                compiler: Some("gcc".to_owned()),
                interpreter: None,
                description: "The reference C kernel.".to_owned(),
                comments: None,
                mode: FpMode::Strict,
                phase: Phase::Run,
                round: 1,
                warmup: false,
                cpu: 8,
                wall_ns: 313_600_000,
                elapsed_ns: 213_300_000,
                user_usec: 860_000,
                system_usec: 4_000,
                peak_bytes: Some(12_582_912),
                source_bytes: Some(2_048),
                checksum: Some(42),
                binary_bytes: None,
                binary_stripped_bytes: None,
                text_bytes: None,
            })
            .unwrap();
        samples
    }

    #[test]
    fn a_conversion_writes_its_own_file_and_never_stdout() {
        let dir = TempDir::new().unwrap();
        let output = dir.path().join("out/samples.csv");
        convert(&ConvertArgs {
            samples: campaign(&dir),
            format: SampleFormat::Csv,
            output: Some(output.clone()),
        })
        .unwrap();

        let converted = fs::read_to_string(&output).unwrap();
        assert!(converted.starts_with("workload,"), "{converted}");
        assert!(converted.contains("mandelbrot,c,gcc,"), "{converted}");
    }

    /// One row per sample, and nothing else: a conversion is not an analysis. The
    /// header plus the one sample this campaign holds — never a min-of-N of it.
    #[test]
    fn a_conversion_aggregates_nothing() {
        let dir = TempDir::new().unwrap();
        let output = dir.path().join("samples.csv");
        convert(&ConvertArgs {
            samples: campaign(&dir),
            format: SampleFormat::Csv,
            output: Some(output.clone()),
        })
        .unwrap();

        assert_eq!(fs::read_to_string(&output).unwrap().lines().count(), 2);
    }

    /// The default is the format's, not the caller's: a CSV lands in `samples.csv`.
    #[test]
    fn a_conversion_that_names_no_output_lands_where_its_format_says() {
        assert_eq!(
            ConvertArgs {
                samples: PathBuf::from("samples.ndjson"),
                format: SampleFormat::Csv,
                output: None,
            }
            .output(),
            PathBuf::from("samples.csv"),
        );
    }
}
