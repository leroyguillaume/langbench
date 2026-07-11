//! Discovery: every `bench.yaml` under the benchmark tree.
//!
//! The manifest is the only source of truth. The path is how we *find* a
//! benchmark, never what describes one: nothing is parsed out of a directory
//! name, and nothing is read back out of a built image. Move a directory, rename
//! it, nest it three levels deeper — the campaign is unchanged.
//!
//! An implementation is identified by *what it is* — the algorithm it computes,
//! and the (language, compiler, interpreter) triple that turns it into
//! instructions. Two directories declaring the same triple are the same
//! implementation declared twice, and that is an error.
//! See `METHODOLOGY.md#repository-layout`.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};

use crate::cli::FpMode;
use crate::sample::backend_slug;

/// The file that declares an implementation. Its presence is the discovery
/// criterion: no manifest, no benchmark.
pub const MANIFEST: &str = "bench.yaml";

/// The `modes: all` keyword, as opposed to an explicit list.
const ALL_MODES: &str = "all";

/// What `bench.yaml` declares, as written on disk.
///
/// `deny_unknown_fields` on purpose: a misspelled key must fail the campaign, not
/// be quietly ignored. A manifest that is half-read is worse than no manifest,
/// because the numbers still come out — they are just wrong about what produced
/// them.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(title = "langbench benchmark manifest")]
struct Manifest {
    /// The algorithm this implementation computes. Implementations of the same
    /// algorithm are comparable; implementations of different ones are not.
    algo: String,
    /// The language the kernel is written in.
    language: String,
    /// The compiler, when something is compiled ahead of the run.
    #[serde(default)]
    compiler: Option<String>,
    /// The interpreter or runtime that executes the result, when there is one.
    /// A backend can have both: Cython compiles, CPython then executes.
    #[serde(default)]
    interpreter: Option<String>,
    /// The FP modes this backend distinguishes: `all`, or an explicit list.
    modes: Modes,
    /// What this backend is, in one paragraph. It is printed beside its rows.
    description: String,
    /// Free-form caveats: what a reader needs to know before quoting this row.
    #[serde(default)]
    comments: Option<String>,
}

/// The manifest's JSON Schema, pretty-printed.
///
/// Generated from the very struct the harness deserializes, never hand-written:
/// a schema maintained by hand is a second declaration of the format, and it
/// drifts. A pre-commit hook regenerates `bench.schema.json` and fails if the
/// checked-in copy has moved, so the schema an editor reads and the struct the
/// campaign parses cannot disagree.
pub fn schema() -> Result<String> {
    serde_json::to_string_pretty(&schema_for!(Manifest)).context("serializing the manifest schema")
}

/// `modes: all`, or `modes: [strict, fma]`.
///
/// The list holds *strings*, and the modes are parsed out of them by hand, for
/// one reason: serde's `untagged` reports a failure as "data did not match any
/// variant of untagged enum Modes", which tells the author of a typo neither
/// what they typed nor what was expected. The schema, meanwhile, is told the
/// list really holds `FpMode`s — so an editor completes the three modes and
/// flags a fourth, while the harness keeps a message a human can act on.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
enum Modes {
    /// Every mode the harness knows.
    #[schemars(extend("enum" = [ALL_MODES]))]
    Keyword(String),
    /// The modes this backend actually distinguishes, e.g. `[strict]`.
    List(#[schemars(with = "Vec<FpMode>")] Vec<String>),
}

/// One benchmark implementation, as its manifest declares it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Implementation {
    pub algo: String,
    pub language: String,
    pub compiler: Option<String>,
    pub interpreter: Option<String>,
    pub description: String,
    pub comments: Option<String>,
    /// The directory the manifest sits in. Where the Dockerfile is, and nothing
    /// more: it identifies no part of this implementation.
    pub context: PathBuf,
    /// The FP modes this implementation declares are meaningful for it. A
    /// compiled backend distinguishes all three; an interpreter has one FP
    /// semantics, so the other two would be the same run under another name.
    pub fp_modes: Vec<FpMode>,
}

impl Implementation {
    /// This implementation's identity, as one token. See `backend_slug`.
    pub fn slug(&self) -> String {
        backend_slug(
            &self.language,
            self.compiler.as_deref(),
            self.interpreter.as_deref(),
        )
    }

    /// One image per (implementation, FP mode). The mode is a build arg, so the
    /// Dockerfile is shared — only the tag differs.
    pub fn image(&self, mode: FpMode) -> String {
        format!("langbench/{}-{}:{mode}", self.algo, self.slug())
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

    /// Read a manifest the walk has already found.
    ///
    /// Every failure here is loud. Unlike a heuristic over a directory name, a
    /// manifest is a deliberate statement: if it does not parse, the honest
    /// outcome is a failed campaign — never an implementation quietly dropped
    /// from the schedule, or measured under a description that is not its own.
    fn load(path: &Path) -> Result<Self> {
        let dir = path
            .parent()
            .with_context(|| format!("{} has no directory", path.display()))?;
        let text =
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let manifest: Manifest =
            serde_norway::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;

        // A backend that neither compiles nor interprets does not exist. This is
        // the one cross-field rule serde cannot express, and it catches the
        // manifest where both keys were forgotten rather than deliberately
        // omitted — which would otherwise produce a nameless, sourceless row.
        if manifest.compiler.is_none() && manifest.interpreter.is_none() {
            bail!(
                "{}: declare a `compiler`, an `interpreter`, or both — \
                 something has to turn the source into instructions",
                path.display(),
            );
        }

        let fp_modes = fp_modes(&manifest.modes)
            .with_context(|| format!("reading `modes` from {}", path.display()))?;

        if !dir.join("Dockerfile").is_file() {
            bail!(
                "{} declares an implementation, but {} holds no Dockerfile to build it from",
                path.display(),
                dir.display(),
            );
        }

        Ok(Self {
            algo: manifest.algo,
            language: manifest.language,
            compiler: manifest.compiler,
            interpreter: manifest.interpreter,
            description: manifest.description,
            comments: manifest.comments,
            context: dir.to_path_buf(),
            fp_modes,
        })
    }
}

/// `all`, or an explicit list. Anything else is an error: a typo in a mode name
/// drops an implementation out of a mode, and a missing row is exactly the kind
/// of absence nobody notices in a table.
fn fp_modes(modes: &Modes) -> Result<Vec<FpMode>> {
    let declared = match modes {
        Modes::Keyword(keyword) if keyword.eq_ignore_ascii_case(ALL_MODES) => {
            return Ok(FpMode::ALL.to_vec());
        }
        Modes::Keyword(keyword) => bail!(
            "`{keyword}` is not a mode list; write `{ALL_MODES}` or a list such as [strict, fma]",
        ),
        Modes::List(declared) => declared,
    };

    // Deduplicated, not rejected: `[strict, strict]` is a manifest that says the
    // same true thing twice, and building the image twice would be the only harm.
    let mut parsed: Vec<FpMode> = Vec::new();
    for token in declared {
        let mode = FpMode::parse(token.trim()).with_context(|| {
            format!("`{token}` is not a known mode; expected one of strict, fma, fast")
        })?;
        if !parsed.contains(&mode) {
            parsed.push(mode);
        }
    }
    ensure!(
        !parsed.is_empty(),
        "no mode declared; write `{ALL_MODES}` or a list such as [strict, fma]",
    );
    Ok(parsed)
}

/// Every implementation declared under `root`, whatever the shape of the tree.
///
/// `algos` filters on the algorithm the *manifest* names; empty means "every
/// algorithm found".
pub fn discover(root: &Path, algos: &[String]) -> Result<Vec<Implementation>> {
    let mut manifests = Vec::new();
    collect_manifests(root, &mut manifests)?;

    let mut found = Vec::new();
    for path in manifests {
        let implementation = Implementation::load(&path)?;
        if !algos.is_empty() && !algos.contains(&implementation.algo) {
            debug!(algo = %implementation.algo, "skipping: not selected by --algo");
            continue;
        }
        debug!(
            algo = %implementation.algo,
            language = %implementation.language,
            compiler = implementation.compiler.as_deref().unwrap_or("none"),
            interpreter = implementation.interpreter.as_deref().unwrap_or("none"),
            path = %path.display(),
            "discovered an implementation",
        );
        found.push(implementation);
    }

    reject_duplicates(&found)?;

    // A stable schedule, ordered by identity rather than by where the files
    // happen to sit: moving a directory must not reorder a campaign.
    found.sort_by_key(|implementation| (implementation.algo.clone(), implementation.slug()));
    Ok(found)
}

/// The same (algo, language, compiler, interpreter) declared twice is not two
/// implementations — it is one, described in two places, and the two would share
/// an image tag and collapse into a single row. Which of them the table would be
/// describing is a coin toss, so refuse instead.
fn reject_duplicates(found: &[Implementation]) -> Result<()> {
    let mut seen: HashMap<(String, String), &Path> = HashMap::new();
    for implementation in found {
        let identity = (implementation.algo.clone(), implementation.slug());
        if let Some(first) = seen.insert(identity, &implementation.context) {
            bail!(
                "{} and {} declare the same implementation ({} / {}): \
                 an implementation is its (language, compiler, interpreter), and there is \
                 exactly one of it",
                first.display(),
                implementation.context.display(),
                implementation.algo,
                implementation.slug(),
            );
        }
    }
    Ok(())
}

/// Parse every manifest under `roots` and say what is wrong with **all** of them.
///
/// Not `discover()` with the result thrown away: discovery stops at the first bad
/// manifest, because a campaign cannot proceed anyway. A *check* has the opposite
/// duty — fix one typo, run again, find the next typo is a miserable way to spend
/// an afternoon — so every error is collected and reported in one pass.
///
/// A path may be a manifest or a directory to walk. The duplicate-identity rule
/// needs the whole tree at once (a backend can only collide with another one),
/// which is why the pre-commit hook re-checks everything whenever *any* manifest
/// moves, rather than only the files that changed.
pub fn validate(roots: &[PathBuf]) -> Result<usize> {
    let mut manifests = Vec::new();
    for root in roots {
        if root.is_file() {
            manifests.push(root.clone());
        } else {
            collect_manifests(root, &mut manifests)?;
        }
    }

    let mut implementations = Vec::new();
    let mut failed = 0usize;
    for path in &manifests {
        match Implementation::load(path) {
            Ok(implementation) => implementations.push(implementation),
            Err(error) => {
                failed += 1;
                error!("{error:#}");
            }
        }
    }

    // Only worth checking once every manifest parses: a collision between two
    // backends, one of which is malformed, is not the error to report.
    if failed == 0
        && let Err(error) = reject_duplicates(&implementations)
    {
        failed += 1;
        error!("{error:#}");
    }

    ensure!(failed == 0, "{failed} manifest(s) rejected");
    info!(manifests = manifests.len(), "every manifest is valid");
    Ok(manifests.len())
}

/// Depth-first walk for manifests. Sorted at every level so that a failing
/// manifest fails the same campaign twice in a row; the resulting order is
/// discarded anyway, since `discover` sorts on identity.
fn collect_manifests(dir: &Path, found: &mut Vec<PathBuf>) -> Result<()> {
    let manifest = dir.join(MANIFEST);
    if manifest.is_file() {
        found.push(manifest);
    }

    let mut children: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    children.sort();

    for child in children {
        collect_manifests(&child, found)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs::{File, create_dir_all, write};

    use tempfile::TempDir;

    use super::*;

    const C_GCC: &str = "algo: mandelbrot\n\
                         language: c\n\
                         compiler: gcc\n\
                         modes: all\n\
                         description: The reference C kernel.\n";

    /// A manifest at an arbitrary depth, with the Dockerfile beside it.
    fn tree(spec: &[(&str, &str)]) -> TempDir {
        let root = TempDir::new().unwrap();
        for (path, manifest) in spec {
            let dir = root.path().join(path);
            create_dir_all(&dir).unwrap();
            File::create(dir.join("Dockerfile")).unwrap();
            write(dir.join(MANIFEST), manifest).unwrap();
        }
        root
    }

    fn one(manifest: &str) -> Result<Vec<Implementation>> {
        discover(tree(&[("anywhere", manifest)]).path(), &[])
    }

    /// The path locates the manifest and says nothing else. Every fact about the
    /// implementation — including which algorithm it computes — comes out of the
    /// file, at whatever depth the file happens to sit.
    #[test]
    fn the_manifest_and_not_the_path_describes_the_implementation() {
        let root = tree(&[(
            "some/deeply/nested/folder",
            "algo: mandelbrot\n\
             language: python\n\
             compiler: cython\n\
             interpreter: cpython\n\
             modes: [strict]\n\
             description: Cython compiles the kernel; CPython runs the result.\n\
             comments: Slower than the interpreter it compiles.\n",
        )]);
        let found = discover(root.path(), &[]).unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].algo, "mandelbrot");
        assert_eq!(found[0].language, "python");
        assert_eq!(found[0].compiler.as_deref(), Some("cython"));
        assert_eq!(found[0].interpreter.as_deref(), Some("cpython"));
        assert!(found[0].description.starts_with("Cython compiles"));
        assert!(found[0].comments.is_some());
        assert_eq!(found[0].fp_modes, [FpMode::Strict]);
    }

    /// The identity is the triple, so the token that carries it is derived from
    /// the triple — a compiled-then-interpreted backend names both halves.
    #[test]
    fn the_slug_is_derived_from_the_identity() {
        let compiled = one(C_GCC).unwrap();
        assert_eq!(compiled[0].slug(), "c-gcc");
        assert_eq!(
            compiled[0].image(FpMode::Strict),
            "langbench/mandelbrot-c-gcc:strict"
        );

        let interpreted = one("algo: mandelbrot\n\
                               language: python\n\
                               interpreter: cpython\n\
                               modes: [strict]\n\
                               description: CPython, no compiler.\n")
        .unwrap();
        assert_eq!(interpreted[0].slug(), "python-cpython");

        let both = one("algo: mandelbrot\n\
                        language: python\n\
                        compiler: cython\n\
                        interpreter: cpython\n\
                        modes: [strict]\n\
                        description: Both.\n")
        .unwrap();
        assert_eq!(both[0].slug(), "python-cython-cpython");
    }

    #[test]
    fn a_directory_without_a_manifest_is_not_an_implementation() {
        let root = tree(&[("benchmarks/c-gcc", C_GCC)]);
        create_dir_all(root.path().join("benchmarks/notes")).unwrap();
        write(root.path().join("benchmarks/notes/README.md"), "hi").unwrap();

        let found = discover(root.path(), &[]).unwrap();
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn the_schedule_is_ordered_by_identity_not_by_where_the_files_sit() {
        let root = tree(&[
            ("zzz", C_GCC),
            (
                "aaa",
                "algo: mandelbrot\n\
                 language: python\n\
                 interpreter: cpython\n\
                 modes: [strict]\n\
                 description: CPython.\n",
            ),
        ]);
        let slugs: Vec<String> = discover(root.path(), &[])
            .unwrap()
            .iter()
            .map(Implementation::slug)
            .collect();
        assert_eq!(slugs, ["c-gcc", "python-cpython"]);
    }

    #[test]
    fn filters_on_the_algorithm_the_manifest_names() {
        let root = tree(&[
            ("one", C_GCC),
            ("two", &C_GCC.replace("algo: mandelbrot", "algo: nbody")),
        ]);

        let found = discover(root.path(), &["nbody".to_owned()]).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].algo, "nbody");

        assert_eq!(discover(root.path(), &[]).unwrap().len(), 2);
    }

    #[test]
    fn the_all_keyword_declares_every_mode() {
        assert_eq!(one(C_GCC).unwrap()[0].fp_modes, FpMode::ALL);
    }

    #[test]
    fn modes_may_be_an_explicit_list() {
        let found = one("algo: mandelbrot\n\
                         language: go\n\
                         compiler: gc\n\
                         modes:\n\
                           - strict\n\
                           - fma\n\
                         description: Go, gc backend.\n")
        .unwrap();
        assert_eq!(found[0].fp_modes, [FpMode::Strict, FpMode::Fma]);
    }

    /// Label-based discovery fell back to every mode on a typo, on the grounds
    /// that a redundant campaign beats a wrong one. A manifest is a deliberate
    /// statement, so the same typo is now an error: building three images where
    /// the author asked for one is not "redundant", it is a table carrying rows
    /// nobody meant to publish.
    #[test]
    fn an_unknown_mode_fails_the_campaign() {
        let error = one("algo: mandelbrot\n\
                         language: c\n\
                         compiler: gcc\n\
                         modes: [stcirt]\n\
                         description: Typo.\n")
        .unwrap_err();
        assert!(format!("{error:#}").contains("stcirt"), "{error:#}");
    }

    #[test]
    fn a_backend_that_neither_compiles_nor_interprets_fails_the_campaign() {
        let error = one("algo: mandelbrot\n\
                         language: c\n\
                         modes: all\n\
                         description: Nothing turns this into instructions.\n")
        .unwrap_err();
        assert!(format!("{error:#}").contains("compiler"), "{error:#}");
    }

    #[test]
    fn an_unknown_key_fails_the_campaign() {
        let error = one("algo: mandelbrot\n\
                         language: c\n\
                         compiler: gcc\n\
                         modes: all\n\
                         description: Fine.\n\
                         compilr: gcc\n")
        .unwrap_err();
        assert!(format!("{error:#}").contains("compilr"), "{error:#}");
    }

    #[test]
    fn a_missing_description_fails_the_campaign() {
        let error = one("algo: mandelbrot\nlanguage: c\ncompiler: gcc\nmodes: all\n").unwrap_err();
        assert!(format!("{error:#}").contains("description"), "{error:#}");
    }

    /// The manifest declares an implementation; without a Dockerfile there is
    /// nothing to build it from, and a missing row would be the only symptom.
    #[test]
    fn a_manifest_without_a_dockerfile_fails_the_campaign() {
        let root = TempDir::new().unwrap();
        let dir = root.path().join("c-gcc");
        create_dir_all(&dir).unwrap();
        write(dir.join(MANIFEST), C_GCC).unwrap();

        let error = discover(root.path(), &[]).unwrap_err();
        assert!(format!("{error:#}").contains("Dockerfile"), "{error:#}");
    }

    /// Two manifests, one identity: they would build the same image tag and
    /// collapse into one row, and which of the two descriptions the report would
    /// print is a coin toss.
    #[test]
    fn the_same_identity_declared_twice_fails_the_campaign() {
        let root = tree(&[("here", C_GCC), ("there", C_GCC)]);
        let error = discover(root.path(), &[]).unwrap_err();
        assert!(format!("{error:#}").contains("c-gcc"), "{error:#}");
    }

    /// The same triple under a different algorithm is a different implementation.
    #[test]
    fn the_same_backend_may_compute_two_algorithms() {
        let root = tree(&[
            ("mandelbrot/c", C_GCC),
            ("nbody/c", &C_GCC.replace("algo: mandelbrot", "algo: nbody")),
        ]);
        assert_eq!(discover(root.path(), &[]).unwrap().len(), 2);
    }

    #[test]
    fn selected_modes_intersect_the_request_with_the_declaration() {
        let found = one("algo: mandelbrot\n\
                         language: python\n\
                         interpreter: cpython\n\
                         modes: [strict]\n\
                         description: CPython, no compiler.\n")
        .unwrap();

        assert!(found[0].compiler.is_none());
        assert_eq!(found[0].selected_modes(&FpMode::ALL), [FpMode::Strict]);
        assert!(found[0].selected_modes(&[FpMode::Fast]).is_empty());
    }

    /// A check exists to be run after a mistake, so it must report *every*
    /// mistake. Reporting the first one and stopping turns one bad afternoon
    /// into three.
    #[test]
    fn validate_reports_every_broken_manifest_not_only_the_first() {
        let root = tree(&[
            ("good", C_GCC),
            (
                "typo",
                "algo: mandelbrot\n\
                 language: rust\n\
                 compiler: llvm\n\
                 modes: [stcirt]\n\
                 description: A misspelled mode.\n",
            ),
            (
                "nameless",
                "algo: mandelbrot\n\
                 language: go\n\
                 modes: all\n\
                 description: Neither compiled nor interpreted.\n",
            ),
        ]);

        let error = validate(&[root.path().to_path_buf()]).unwrap_err();
        assert!(format!("{error:#}").contains('2'), "{error:#}");
    }

    #[test]
    fn validate_accepts_a_manifest_path_as_well_as_a_directory() {
        let root = tree(&[("c-gcc", C_GCC)]);
        let manifest = root.path().join("c-gcc").join(MANIFEST);
        assert_eq!(validate(&[manifest]).unwrap(), 1);
        assert_eq!(validate(&[root.path().to_path_buf()]).unwrap(), 1);
    }

    /// Two backends can only collide with *each other*, so the check needs both
    /// halves in view — which is why the hook validates the tree, not the diff.
    #[test]
    fn validate_catches_a_duplicate_identity_across_the_tree() {
        let root = tree(&[("here", C_GCC), ("there", C_GCC)]);
        let error = validate(&[root.path().to_path_buf()]).unwrap_err();
        assert!(format!("{error:#}").contains("rejected"), "{error:#}");
    }

    /// The schema is generated from the struct the harness deserializes. If this
    /// ever needs updating by hand, the generator has been bypassed.
    #[test]
    fn the_schema_describes_the_manifest_the_harness_parses() {
        let schema = schema().unwrap();
        for key in ["algo", "language", "compiler", "interpreter", "description"] {
            assert!(schema.contains(&format!("\"{key}\"")), "{key} is missing");
        }
        // The three modes are offered to an editor as constants, not as "a
        // string": completing `strict` is the point of shipping a schema.
        for mode in FpMode::ALL {
            assert!(schema.contains(&format!("\"const\": \"{mode}\"")), "{mode}");
        }
        // A misspelled key must fail the campaign, and the schema must say so.
        assert!(schema.contains("\"additionalProperties\": false"));
    }
}
