//! Convention over configuration: `benchmarks/<algo>/<language>-<compiler>/Dockerfile`.
//!
//! There is no manifest. See `METHODOLOGY.md#repository-layout`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use tracing::{debug, warn};

use crate::cli::FpMode;

/// The label an implementation uses to declare the FP modes it can be built
/// under. Absent means all of them.
const FP_MODES_LABEL: &str = "langbench.fp_modes";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Implementation {
    pub algo: String,
    /// Directory name, e.g. `c-gcc`.
    pub name: String,
    pub language: String,
    pub compiler: String,
    pub context: PathBuf,
    /// The FP modes this implementation declares are meaningful for it, from
    /// its `langbench.fp_modes` label. A compiled backend has nothing to
    /// declare and gets every mode; an interpreter has one FP semantics, so the
    /// other two modes would be the same run under another name.
    pub fp_modes: Vec<FpMode>,
}

impl Implementation {
    /// One image per (implementation, FP mode). The mode is a build arg, so the
    /// Dockerfile is shared — only the tag differs.
    pub fn image(&self, mode: FpMode) -> String {
        format!("langbench/{}-{}:{mode}", self.algo, self.name)
    }

    /// The requested modes this implementation actually distinguishes, in the
    /// order they were requested.
    pub fn selected_modes(&self, requested: &[FpMode]) -> Vec<FpMode> {
        requested
            .iter()
            .copied()
            .filter(|mode| self.fp_modes.contains(mode))
            .collect()
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
            fp_modes: declared_fp_modes(&dir.join("Dockerfile"), &name),
            name,
            context: dir.to_path_buf(),
        })
    }
}

/// Read `langbench.fp_modes` out of a Dockerfile, before anything is built.
///
/// The label is read from the source and not from `docker inspect`, unlike every
/// other piece of metadata: it decides *which images to build*, so it has to be
/// known before there is an image to inspect. It is a constant in the file —
/// never a build arg — precisely so that this is possible.
///
/// Anything unreadable, absent or unparseable falls back to every mode: the
/// campaign a missing label produces is redundant, never wrong.
fn declared_fp_modes(dockerfile: &Path, name: &str) -> Vec<FpMode> {
    let Ok(text) = fs::read_to_string(dockerfile) else {
        return FpMode::ALL.to_vec();
    };
    let Some(declared) = label_value(&text, FP_MODES_LABEL) else {
        return FpMode::ALL.to_vec();
    };

    let mut modes = Vec::new();
    for token in declared.split(',').map(str::trim).filter(|t| !t.is_empty()) {
        match FpMode::parse(token) {
            Some(mode) if !modes.contains(&mode) => modes.push(mode),
            Some(_) => {}
            None => warn!(
                %name,
                %token,
                "ignoring an unknown mode in the `{FP_MODES_LABEL}` label",
            ),
        }
    }

    if modes.is_empty() {
        warn!(
            %name,
            %declared,
            "the `{FP_MODES_LABEL}` label names no known mode; assuming every mode",
        );
        return FpMode::ALL.to_vec();
    }
    debug!(%name, ?modes, "declared FP modes");
    modes
}

/// The value of `<key>="…"` in a Dockerfile, ignoring comments.
fn label_value(dockerfile: &str, key: &str) -> Option<String> {
    dockerfile
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .find_map(|line| line.split_once(&format!("{key}=\""))?.1.split_once('"'))
        .map(|(value, _)| value.to_owned())
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
    use std::fs::{File, create_dir_all, write};

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

    /// A tree with one implementation whose Dockerfile has the given content.
    fn tree_with_dockerfile(name: &str, content: &str) -> TempDir {
        let root = TempDir::new().unwrap();
        let dir = root.path().join("mandelbrot").join(name);
        create_dir_all(&dir).unwrap();
        write(dir.join("Dockerfile"), content).unwrap();
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

    #[test]
    fn an_implementation_without_the_label_is_built_under_every_mode() {
        let root = tree(&[("mandelbrot", "c-gcc", true)]);
        let found = discover(root.path(), &[]).unwrap();
        assert_eq!(found[0].fp_modes, FpMode::ALL);
    }

    #[test]
    fn the_fp_modes_label_is_read_from_the_dockerfile() {
        // As a Python backend writes it: one FP semantics, so `fma` and `fast`
        // would be the same run under another name.
        let root = tree_with_dockerfile(
            "python-cpython",
            "FROM python\n\
             LABEL langbench.language=\"python\" \\\n      \
                   langbench.fp_modes=\"strict\" \\\n      \
                   langbench.flags=\"fp=${FP_MODE}\"\n",
        );
        let found = discover(root.path(), &[]).unwrap();
        assert_eq!(found[0].fp_modes, [FpMode::Strict]);
    }

    #[test]
    fn the_fp_modes_label_takes_a_list() {
        let root = tree_with_dockerfile("go-gc", "LABEL langbench.fp_modes=\"strict, fma\"\n");
        let found = discover(root.path(), &[]).unwrap();
        assert_eq!(found[0].fp_modes, [FpMode::Strict, FpMode::Fma]);
    }

    #[test]
    fn a_comment_mentioning_the_label_does_not_declare_it() {
        // The Python Dockerfiles explain the label in prose right above it.
        let root = tree_with_dockerfile(
            "c-gcc",
            "# langbench.fp_modes=\"strict\" would be wrong here: gcc contracts.\n\
             FROM gcc\n",
        );
        let found = discover(root.path(), &[]).unwrap();
        assert_eq!(found[0].fp_modes, FpMode::ALL);
    }

    #[test]
    fn a_label_naming_no_known_mode_falls_back_to_every_mode() {
        // Redundant beats wrong: a typo must not silently drop an implementation
        // out of the campaign.
        let root = tree_with_dockerfile("c-gcc", "LABEL langbench.fp_modes=\"stcirt\"\n");
        let found = discover(root.path(), &[]).unwrap();
        assert_eq!(found[0].fp_modes, FpMode::ALL);
    }

    #[test]
    fn selected_modes_intersect_the_request_with_the_declaration() {
        let root = tree_with_dockerfile("python-cpython", "LABEL langbench.fp_modes=\"strict\"\n");
        let found = discover(root.path(), &[]).unwrap();

        assert_eq!(found[0].selected_modes(&FpMode::ALL), [FpMode::Strict]);
        assert!(found[0].selected_modes(&[FpMode::Fast]).is_empty());
    }
}
