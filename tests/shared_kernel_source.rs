//! Two backends of the same language must compile the *same* kernel.
//!
//! `python` / `cython` / `cpython` and `python` / `cpython` differ only in their
//! compiler; if their sources drifted apart, the row-to-row comparison would
//! silently stop being the "same source, different backend" experiment and become
//! the confounded one.
//! See `METHODOLOGY.md#two-axes-two-tables-never-merged`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Files that describe how to build, run and declare a backend — not what it
/// computes. `bench.yaml` differs between two backends *by construction*: that is
/// what makes them two backends.
const NOT_KERNEL_SOURCE: &[&str] = &["Dockerfile", "entrypoint.sh", "bench.yaml"];

#[test]
fn implementations_of_one_language_share_their_kernel_source() {
    let benchmarks = Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmarks");
    let mut compared = 0usize;

    // Keyed on (workload, language) out of the *manifests*. The directory name is not
    // evidence of anything — the harness does not read it, and neither does this.
    let mut by_language: BTreeMap<(String, String), Vec<PathBuf>> = BTreeMap::new();
    for manifest in manifests(&benchmarks) {
        let dir = manifest.parent().expect("a manifest sits in a directory");
        let text = fs::read_to_string(&manifest).expect("the manifest is readable");
        by_language
            .entry((declared(&text, "workload"), declared(&text, "language")))
            .or_default()
            .push(dir.to_path_buf());
    }

    for ((workload, language), implementations) in by_language {
        let Some((reference, others)) = implementations.split_first() else {
            continue;
        };
        for other in others {
            assert_eq!(
                kernel_source(reference),
                kernel_source(other),
                "`{}` and `{}` are both {language} backends of {workload}, so they must compile a \
                 byte-identical kernel; otherwise their rows compare two programs, not two \
                 backends",
                reference.display(),
                other.display(),
            );
            compared += 1;
        }
    }

    // Guard against a vacuous pass once a second backend of some language exists.
    assert!(
        compared > 0,
        "no language has two backends yet; delete this guard when that changes",
    );
}

/// The value of a top-level scalar key, as the manifest spells it.
fn declared(manifest: &str, key: &str) -> String {
    manifest
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}: ")))
        .unwrap_or_else(|| panic!("every manifest declares `{key}`"))
        .trim()
        .to_owned()
}

/// Every file of an implementation except the plumbing, keyed by name.
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

/// Every `bench.yaml` under `root`, at any depth: the same criterion the harness
/// discovers by.
fn manifests(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let manifest = root.join("bench.yaml");
    if manifest.is_file() {
        found.push(manifest);
    }
    let mut children: Vec<PathBuf> = fs::read_dir(root)
        .unwrap_or_else(|error| panic!("reading {}: {error}", root.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    children.sort();
    for child in children {
        found.extend(manifests(&child));
    }
    found
}
