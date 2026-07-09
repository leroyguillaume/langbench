//! Convention over configuration: `benchmarks/<algo>/<language>-<compiler>/Dockerfile`.
//!
//! There is no manifest. See `METHODOLOGY.md#repository-layout`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use tracing::{debug, warn};

use crate::cli::FpMode;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Implementation {
    pub algo: String,
    /// Directory name, e.g. `c-gcc`.
    pub name: String,
    pub language: String,
    pub compiler: String,
    pub context: PathBuf,
}

impl Implementation {
    /// One image per (implementation, FP mode). The mode is a build arg, so the
    /// Dockerfile is shared — only the tag differs.
    pub fn image(&self, mode: FpMode) -> String {
        format!("langbench/{}-{}:{mode}", self.algo, self.name)
    }

    fn from_dir(algo: &str, dir: &Path) -> Option<Self> {
        let name = dir.file_name()?.to_str()?.to_owned();
        let Some((language, compiler)) = name.split_once('-') else {
            warn!(
                %name,
                "skipping: directory must be named `<language>-<compiler>`",
            );
            return None;
        };
        Some(Self {
            algo: algo.to_owned(),
            language: language.to_owned(),
            compiler: compiler.to_owned(),
            name,
            context: dir.to_path_buf(),
        })
    }
}

/// Walk the benchmark tree, keeping only directories that hold a `Dockerfile`.
///
/// `algos` filters by algorithm; empty means "every algorithm found".
pub fn discover(root: &Path, algos: &[String]) -> Result<Vec<Implementation>> {
    let mut found = Vec::new();

    for algo_entry in sorted_dirs(root)? {
        let Some(algo) = algo_entry.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !algos.is_empty() && !algos.iter().any(|wanted| wanted == algo) {
            debug!(%algo, "skipping: not selected by --algo");
            continue;
        }
        for impl_entry in sorted_dirs(&algo_entry)? {
            if !impl_entry.join("Dockerfile").is_file() {
                debug!(path = %impl_entry.display(), "skipping: no Dockerfile");
                continue;
            }
            if let Some(implementation) = Implementation::from_dir(algo, &impl_entry) {
                found.push(implementation);
            }
        }
    }

    debug!(count = found.len(), "discovered implementations");
    Ok(found)
}

fn sorted_dirs(root: &Path) -> Result<Vec<PathBuf>> {
    let mut dirs: Vec<_> = fs::read_dir(root)
        .with_context(|| format!("reading {}", root.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    // Stable order: the round-robin schedule must be reproducible.
    dirs.sort();
    Ok(dirs)
}

#[cfg(test)]
mod tests {
    use std::fs::{File, create_dir_all};

    use tempfile::TempDir;

    use super::*;

    fn tree(spec: &[(&str, &str, bool)]) -> TempDir {
        let root = TempDir::new().unwrap();
        for (algo, implementation, dockerfile) in spec {
            let dir = root.path().join(algo).join(implementation);
            create_dir_all(&dir).unwrap();
            if *dockerfile {
                File::create(dir.join("Dockerfile")).unwrap();
            }
        }
        root
    }

    #[test]
    fn discovers_implementations_and_splits_the_directory_name() {
        let root = tree(&[
            ("mandelbrot", "c-gcc", true),
            ("mandelbrot", "rust-llvm", true),
        ]);
        let found = discover(root.path(), &[]).unwrap();

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].language, "c");
        assert_eq!(found[0].compiler, "gcc");
        assert_eq!(found[1].language, "rust");
        assert_eq!(found[1].compiler, "llvm");
    }

    #[test]
    fn skips_directories_without_a_dockerfile() {
        let root = tree(&[
            ("mandelbrot", "c-gcc", true),
            ("mandelbrot", "go-gc", false),
        ]);
        let found = discover(root.path(), &[]).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "c-gcc");
    }

    #[test]
    fn skips_directories_that_do_not_name_a_compiler() {
        let root = tree(&[("mandelbrot", "rust", true)]);
        assert!(discover(root.path(), &[]).unwrap().is_empty());
    }

    #[test]
    fn an_empty_filter_discovers_every_algorithm_in_the_tree() {
        let root = tree(&[("mandelbrot", "c-gcc", true), ("nbody", "c-gcc", true)]);
        let found = discover(root.path(), &[]).unwrap();
        let algos: Vec<_> = found.iter().map(|i| i.algo.as_str()).collect();
        assert_eq!(algos, ["mandelbrot", "nbody"]);
    }

    #[test]
    fn filters_by_algorithm() {
        let root = tree(&[("mandelbrot", "c-gcc", true), ("nbody", "c-gcc", true)]);
        let found = discover(root.path(), &["nbody".to_owned()]).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].algo, "nbody");
    }

    #[test]
    fn the_image_tag_carries_the_fp_mode() {
        let root = tree(&[("mandelbrot", "c-gcc", true)]);
        let found = discover(root.path(), &[]).unwrap();
        assert_eq!(
            found[0].image(FpMode::Strict),
            "langbench/mandelbrot-c-gcc:strict"
        );
    }
}
