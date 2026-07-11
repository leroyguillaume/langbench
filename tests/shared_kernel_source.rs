//! Two backends of the same language must compile the *same* kernel.
//!
//! `python-cpython` and `python-cython` differ only in their compiler; if their
//! sources drifted apart, the row-to-row comparison would silently stop being
//! the "same source, different backend" experiment and become the confounded one.
//! See `METHODOLOGY.md#two-axes-two-tables-never-merged`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Files that describe how to build and run, not what to compute.
const NOT_KERNEL_SOURCE: &[&str] = &["Dockerfile", "entrypoint.sh"];

#[test]
fn implementations_of_one_language_share_their_kernel_source() {
    let benchmarks = Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmarks");
    let mut compared = 0usize;

    for algo in directories(&benchmarks) {
        let mut by_language: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
        for implementation in directories(&algo) {
            let name = implementation.file_name().unwrap().to_string_lossy();
            let Some((language, _compiler)) = name.split_once('-') else {
                continue;
            };
            by_language
                .entry(language.to_owned())
                .or_default()
                .push(implementation.clone());
        }

        for (language, implementations) in by_language {
            let Some((reference, others)) = implementations.split_first() else {
                continue;
            };
            for other in others {
                assert_eq!(
                    kernel_source(reference),
                    kernel_source(other),
                    "`{}` and `{}` are both {language} backends, so they must compile a \
                     byte-identical kernel; otherwise their rows compare two programs, not \
                     two backends",
                    reference.display(),
                    other.display(),
                );
                compared += 1;
            }
        }
    }

    // Guard against a vacuous pass once a second backend of some language exists.
    assert!(
        compared > 0,
        "no language has two backends yet; delete this guard when that changes",
    );
}

/// Every file of an implementation except the container plumbing, keyed by name.
fn kernel_source(implementation: &Path) -> BTreeMap<String, String> {
    fs::read_dir(implementation)
        .expect("implementation directory is readable")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?.to_owned();
            if NOT_KERNEL_SOURCE.contains(&name.as_str()) {
                return None;
            }
            Some((name, fs::read_to_string(&path).ok()?))
        })
        .collect()
}

fn directories(root: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("reading {}: {error}", root.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    dirs
}
