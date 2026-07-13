//! Two backends of the same language must compile the *same* kernel.
//!
//! `python` / `cython` / `cpython` and `python` / `cpython` differ only in their
//! compiler; if their sources drifted apart, the row-to-row comparison would
//! silently stop being the "same source, different backend" experiment and become
//! the confounded one.
//! See `site/src/content/methodology/what-is-under-test.md#two-axes-two-tables-never-merged`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use langbench::discovery;

/// Files that describe how to build, run and declare a backend — not what it
/// computes. `bench.yaml` differs between two backends *by construction*: that is
/// what makes them two backends.
const NOT_KERNEL_SOURCE: &[&str] = &["Dockerfile", "entrypoint.sh", "bench.yaml"];

#[test]
fn implementations_of_one_language_share_their_kernel_source() {
    let benchmarks = Path::new(env!("CARGO_MANIFEST_DIR")).join("benchmarks");
    let mut compared = 0usize;

    // Through the harness's own discovery, and not by re-reading the YAML here. A
    // second parser is a second definition of what a benchmark is, and it would drift
    // from the first one the day the manifest changed shape — which is precisely what
    // happened to this test when the workload moved out of `bench.yaml`.
    //
    // Keyed on (workload, language): the directory name is not evidence of anything.
    let mut by_language: BTreeMap<(String, String), Vec<PathBuf>> = BTreeMap::new();
    for root in discovery::workloads(&benchmarks).expect("the workloads are declared") {
        let implementations = discovery::discover(&benchmarks, &root.workload.id)
            .expect("every implementation the workload declares is valid");
        for implementation in implementations {
            by_language
                .entry((implementation.workload, implementation.language))
                .or_default()
                .push(implementation.context);
        }
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
