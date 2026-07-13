//! Discovery: every `bench.yaml` under the benchmark tree.
//!
//! The manifest is the only source of truth. The path is how we *find* a
//! benchmark, never what describes one: nothing is parsed out of a directory
//! name, and nothing is read back out of a built image. Move a directory, rename
//! it, nest it three levels deeper — the campaign is unchanged.
//!
//! An implementation is identified by *what it is* — the workload it computes,
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
use tracing::{debug, error, info, warn};

use crate::cli::Architecture;
use crate::mode::FpMode;
use crate::sample::backend_slug;
use crate::workload::{self, Workload};

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
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
#[schemars(title = "langbench benchmark manifest", rename_all = "kebab-case")]
struct Manifest {
    /// The language the kernel is written in.
    language: String,
    /// The compiler, when something is compiled ahead of the run.
    #[serde(default)]
    compiler: Option<String>,
    /// The interpreter or runtime that executes the result, when there is one.
    /// A backend can have both: Cython compiles, CPython then executes.
    #[serde(default)]
    interpreter: Option<String>,
    /// The kernel, relative to this manifest's directory. One file, because the
    /// benchmark rule is one file — and its size in bytes is a published metric.
    ///
    /// Declared rather than guessed: the harness would otherwise have to decide
    /// which of the files beside the manifest is *the source*, and the only way to
    /// do that is to pattern-match a name — which is parsing the path, under
    /// another name. See `METHODOLOGY.md#repository-layout`.
    source: String,
    /// The FP modes this backend distinguishes: `all`, or an explicit list.
    modes: Modes,
    /// The architectures this backend can be built on: `all`, or an explicit
    /// list. Defaults to `all`, which is the ordinary case — a toolchain that
    /// exists everywhere needs to say nothing.
    ///
    /// It is not a preference, it is a fact: Kotlin/Native ships no
    /// `linux-aarch64` host compiler, so that backend cannot be built on an
    /// AArch64 machine at all, and emulating one is forbidden. A campaign on the
    /// other architecture skips the row loudly instead of failing at `docker
    /// build`.
    #[serde(default)]
    architectures: Architectures,
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

/// `architectures: all`, or `architectures: [x86_64]`.
///
/// The same shape as `Modes`, and for the same reason: the list holds *strings*
/// so that a typo can be reported as the typo it is, while the schema is told the
/// list really holds `Architecture`s, so an editor completes them.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
enum Architectures {
    /// Every architecture the harness knows. The default, and the ordinary case.
    #[schemars(extend("enum" = [ALL_MODES]))]
    Keyword(String),
    /// The architectures this backend can actually be built on, e.g. `[x86_64]`.
    List(#[schemars(with = "Vec<Architecture>")] Vec<String>),
}

impl Default for Architectures {
    /// A manifest that says nothing about architecture is claiming to build
    /// anywhere — which is true of every toolchain here but one.
    fn default() -> Self {
        Self::Keyword(ALL_MODES.to_owned())
    }
}

/// One benchmark implementation, as its manifest declares it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Implementation {
    pub workload: String,
    pub language: String,
    pub compiler: Option<String>,
    pub interpreter: Option<String>,
    pub description: String,
    pub comments: Option<String>,
    /// The directory the manifest sits in. Where the Dockerfile is, and nothing
    /// more: it identifies no part of this implementation.
    pub context: PathBuf,
    /// The kernel's source file, as the manifest declared it.
    pub source: PathBuf,
    /// Its size. Read once, at discovery, and copied onto every sample — like the
    /// rest of the manifest, so a sample describes itself without a second file to
    /// join against.
    pub source_bytes: u64,
    /// The FP modes this implementation declares are meaningful for it. A
    /// compiled backend distinguishes all three; an interpreter has one FP
    /// semantics, so the other two would be the same run under another name.
    pub fp_modes: Vec<FpMode>,
    /// The architectures this implementation can be built on. Almost always both;
    /// a backend whose toolchain does not exist for an architecture says so here.
    pub architectures: Vec<Architecture>,
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
        format!("langbench/{}-{}:{mode}", self.workload, self.slug())
    }

    /// Can this implementation be built on this machine at all?
    ///
    /// A `None` host is an architecture the harness does not know, and nothing can
    /// be claimed about it — not even by a manifest that says `all`, because `all`
    /// means "both of the two", not "whatever you happen to be running".
    pub fn supports(&self, host: Option<Architecture>) -> bool {
        host.is_some_and(|host| self.architectures.contains(&host))
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
    fn load(path: &Path, workload: &str) -> Result<Self> {
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

        let architectures = architectures(&manifest.architectures)
            .with_context(|| format!("reading `architectures` from {}", path.display()))?;

        if !dir.join("Dockerfile").is_file() {
            bail!(
                "{} declares an implementation, but {} holds no Dockerfile to build it from",
                path.display(),
                dir.display(),
            );
        }

        // The manifest names one file, and it has to *be* a file. A `source:` that
        // resolves to nothing publishes a byte count for a kernel nobody read; an
        // empty one resolves to the directory itself, which `fs::metadata` is
        // perfectly happy to size — and a report would then quote the size of a
        // directory entry as the length of a Mandelbrot kernel. Both are caught
        // here, at discovery, and not an hour into a campaign.
        let source = dir.join(manifest.source.trim());
        let source_bytes = fs::metadata(&source)
            .ok()
            .filter(std::fs::Metadata::is_file)
            .map(|metadata| metadata.len())
            .with_context(|| {
                format!(
                    "{} declares `source: {}`, which is not a file in {}. It names the one \
                     kernel this implementation compiles, and its size is a published metric — \
                     so it has to exist and it has to be a file.",
                    path.display(),
                    manifest.source,
                    dir.display(),
                )
            })?;

        Ok(Self {
            workload: workload.to_owned(),
            language: manifest.language,
            compiler: manifest.compiler,
            interpreter: manifest.interpreter,
            description: manifest.description,
            comments: manifest.comments,
            context: dir.to_path_buf(),
            source,
            source_bytes,
            fp_modes,
            architectures,
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

/// `all`, or an explicit list. Anything else is an error, for the same reason a
/// misspelled mode is: a typo here silently drops a backend out of every campaign
/// on one architecture, and a missing row is exactly the kind of absence nobody
/// notices in a table.
fn architectures(architectures: &Architectures) -> Result<Vec<Architecture>> {
    let declared = match architectures {
        Architectures::Keyword(keyword) if keyword.eq_ignore_ascii_case(ALL_MODES) => {
            return Ok(Architecture::ALL.to_vec());
        }
        Architectures::Keyword(keyword) => bail!(
            "`{keyword}` is not an architecture list; write `{ALL_MODES}` or a list such as \
             [x86_64]",
        ),
        Architectures::List(declared) => declared,
    };

    let mut parsed: Vec<Architecture> = Vec::new();
    for token in declared {
        let architecture = Architecture::parse(token.trim()).with_context(|| {
            format!("`{token}` is not a known architecture; expected one of x86_64, aarch64")
        })?;
        if !parsed.contains(&architecture) {
            parsed.push(architecture);
        }
    }
    ensure!(
        !parsed.is_empty(),
        "no architecture declared; write `{ALL_MODES}` or a list such as [x86_64]",
    );
    Ok(parsed)
}

/// A workload, and the directory whose subtree its implementations live in.
#[derive(Clone, Debug)]
pub struct Root {
    pub workload: Workload,
    /// The directory the `workload.yaml` sits in. It owns every `bench.yaml`
    /// beneath it — and, like an implementation's directory, it *identifies*
    /// nothing: the id comes out of the file.
    pub dir: PathBuf,
}

/// Every workload declared under `root`.
///
/// The walk finds the files; the files say what they are. Two workloads with the
/// same `id` are one workload declared twice — the campaigns would overwrite each
/// other's samples — so that is an error.
pub fn workloads(root: &Path) -> Result<Vec<Root>> {
    let mut manifests = Vec::new();
    collect(root, workload::MANIFEST, &mut manifests)?;

    let mut found: Vec<Root> = Vec::new();
    for path in manifests {
        let text =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let workload =
            Workload::parse(&text).with_context(|| format!("parsing {}", path.display()))?;

        if let Some(first) = found.iter().find(|root| root.workload.id == workload.id) {
            bail!(
                "{} and {} both declare the workload `{}`: a workload is declared once, and \
                 its id is what names the campaign that measures it",
                first.dir.display(),
                path.display(),
                workload.id,
            );
        }

        debug!(workload = %workload.id, path = %path.display(), "discovered a workload");
        found.push(Root {
            workload,
            dir: path
                .parent()
                .with_context(|| format!("{} has no directory", path.display()))?
                .to_path_buf(),
        });
    }

    found.sort_by(|left, right| left.workload.id.cmp(&right.workload.id));
    Ok(found)
}

impl Root {
    /// The `bench.yaml` paths this workload declares, in declaration order.
    ///
    /// Each is a directory the manifest names, resolved against the manifest's own
    /// directory. Nothing is walked for, and nothing is inferred from where a
    /// directory sits: a workload measures what it says it measures.
    ///
    /// A declared directory with no `bench.yaml` in it — moved, renamed, not
    /// written yet — is **warned about and skipped**. It is an absence somebody
    /// created on purpose or by accident, and either way a campaign should say so
    /// loudly and go on measuring the rest, not refuse to start.
    fn manifests(&self) -> Vec<PathBuf> {
        let mut found = Vec::new();
        for declared in &self.workload.implementations {
            let dir = self.dir.join(declared.trim());
            let manifest = dir.join(MANIFEST);
            if manifest.is_file() {
                found.push(manifest);
            } else {
                warn!(
                    workload = %self.workload.id,
                    path = %dir.display(),
                    "declared as an implementation of this workload, but holds no {MANIFEST}: \
                     skipping it. Nothing here will be built or measured.",
                );
            }
        }
        found
    }
}

/// Every implementation of one workload, as the workload declares them.
pub fn discover(root: &Path, workload: &str) -> Result<Vec<Implementation>> {
    let roots = workloads(root)?;
    let chosen = roots
        .iter()
        .find(|candidate| candidate.workload.id == workload)
        .with_context(|| {
            format!(
                "no workload `{workload}` under {}; there is {}",
                root.display(),
                declared(&roots),
            )
        })?;

    let mut found = Vec::new();
    for path in chosen.manifests() {
        let implementation = Implementation::load(&path, &chosen.workload.id)?;
        debug!(
            workload = %implementation.workload,
            language = %implementation.language,
            compiler = implementation.compiler.as_deref().unwrap_or("none"),
            interpreter = implementation.interpreter.as_deref().unwrap_or("none"),
            path = %path.display(),
            "discovered an implementation",
        );
        found.push(implementation);
    }

    reject_duplicates(&found)?;

    // A stable schedule, ordered by identity rather than by the order the manifest
    // happens to list them in: reordering the list must not reorder a campaign.
    found.sort_by_key(Implementation::slug);
    Ok(found)
}

/// The workloads on disk, for an error message that tells the reader what they
/// *could* have asked for.
fn declared(roots: &[Root]) -> String {
    if roots.is_empty() {
        return "none".to_owned();
    }
    roots
        .iter()
        .map(|root| root.workload.id.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The same (workload, language, compiler, interpreter) declared twice is not two
/// implementations — it is one, described in two places, and the two would share
/// an image tag and collapse into a single row. Which of them the table would be
/// describing is a coin toss, so refuse instead.
fn reject_duplicates(found: &[Implementation]) -> Result<()> {
    let mut seen: HashMap<(String, String), &Path> = HashMap::new();
    for implementation in found {
        let identity = (implementation.workload.clone(), implementation.slug());
        if let Some(first) = seen.insert(identity, &implementation.context) {
            bail!(
                "{} and {} declare the same implementation ({} / {}): \
                 an implementation is its (language, compiler, interpreter), and there is \
                 exactly one of it",
                first.display(),
                implementation.context.display(),
                implementation.workload,
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
pub fn validate(paths: &[PathBuf]) -> Result<usize> {
    // Not counted among the rejected manifests: being handed the wrong *kind* of
    // argument is not a finding about the tree, and reporting it as one would tell
    // the reader a manifest is broken when none of them is.
    let roots = workloads_of(paths)?;
    let mut failed = 0usize;

    // Every `bench.yaml` on disk, and every `bench.yaml` some workload claims. The
    // two sets have to be the same one.
    //
    // A campaign never walks the tree — it reads the list a workload declares — so
    // a manifest that no workload lists would never be built, never measured, and
    // never missed. That absence is invisible from the results, which is precisely
    // the failure this project refuses: a row that is not in the table reads exactly
    // like a backend nobody wrote. Discovery cannot catch it; the check can, and
    // this is why it exists.
    let mut on_disk = Vec::new();
    for path in paths {
        if path.is_file() {
            on_disk.push(path.clone());
        } else {
            collect(path, MANIFEST, &mut on_disk)?;
        }
    }

    let mut claimed = Vec::new();
    let mut implementations = Vec::new();
    for root in &roots {
        for manifest in root.manifests() {
            claimed.push(manifest.clone());
            match Implementation::load(&manifest, &root.workload.id) {
                Ok(implementation) => implementations.push(implementation),
                Err(error) => {
                    failed += 1;
                    error!("{error:#}");
                }
            }
        }
    }

    for orphan in on_disk.iter().filter(|path| !claimed.contains(path)) {
        failed += 1;
        error!(
            "{} declares an implementation that no workload lists. It will never be built and \
             never measured, and nothing in a report would say so — a row that is missing from a \
             table reads exactly like a backend nobody wrote. Add its directory to a workload's \
             `implementations`, or delete it.",
            orphan.display(),
        );
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
    info!(
        workloads = roots.len(),
        implementations = implementations.len(),
        "every manifest is valid",
    );
    Ok(implementations.len())
}

/// The workloads declared under a set of directories.
///
/// Directories, and not a hand-picked list of changed files, for two reasons that
/// both come down to *a check only sees what it is shown*: two backends collide
/// with each other, so a duplicate identity is invisible unless both halves are in
/// view; and a `bench.yaml` that no workload lists can only be spotted by someone
/// holding the whole tree and the whole list of declarations at once. This is why
/// the pre-commit hook re-checks everything whenever any manifest moves.
fn workloads_of(paths: &[PathBuf]) -> Result<Vec<Root>> {
    let mut roots = Vec::new();
    for path in paths {
        ensure!(
            !path.is_file(),
            "{} is a file; `validate` takes the directories to walk. A single manifest cannot be \
             checked on its own: a duplicate identity is a collision between *two* backends, and a \
             manifest that no workload declares can only be seen by looking at both the tree and \
             the declarations.",
            path.display(),
        );
        roots.extend(workloads(path)?);
    }
    Ok(roots)
}

/// Depth-first walk for a manifest of the given name. Sorted at every level so
/// that a failing manifest fails the same check twice in a row.
///
/// The one place the harness still *searches* rather than reads a declaration:
/// finding the `workload.yaml` files, which are the roots everything else hangs
/// off, and finding stray `bench.yaml` files that no workload claims.
fn collect(dir: &Path, name: &str, found: &mut Vec<PathBuf>) -> Result<()> {
    let manifest = dir.join(name);
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
        collect(&child, name, found)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs::{File, create_dir_all, write};

    use tempfile::TempDir;

    use super::*;

    const C_GCC: &str = "language: c\n\
                         compiler: gcc\n\
                         source: kernel.txt\n\
                         modes: all\n\
                         description: The reference C kernel.\n";

    const RUST: &str = "language: rust\n\
                        compiler: rustc\n\
                        source: kernel.txt\n\
                        modes: all\n\
                        description: The same kernel, in Rust.\n";

    /// The kernel every fixture ships, and its size on disk.
    const SOURCE: &str = "kernel.txt";
    const SOURCE_TEXT: &str = "// the fixture's kernel\n";
    const SOURCE_BYTES: u64 = 24;

    /// A workload declaring the directories it is implemented in — the list, not a
    /// walk. `WORKLOAD` is the id every fixture below runs under.
    const WORKLOAD: &str = "mandelbrot";

    fn workload_manifest(id: &str, implementations: &[&str]) -> String {
        let mut yaml = format!(
            "id: {id}\n\
             description: Escape-time Mandelbrot over a fixed grid.\n\
             params:\n\
             \x20 - name: grid_size\n\
             \x20   value: 2048\n\
             implementations:\n",
        );
        for path in implementations {
            yaml.push_str(&format!("  - {path}\n"));
        }
        yaml
    }

    /// One workload at the root, and its implementations at arbitrary depths beneath
    /// it — each with its Dockerfile and its kernel beside the manifest.
    ///
    /// The workload declares every directory in `spec`, because that is how a
    /// campaign finds them: it reads the list, it does not go looking. A fixture that
    /// does not spell `source:` gets one, pointing at the kernel written here — the
    /// tests below are about discovery's *other* rules, and repeating the same line
    /// in every one of them would bury what each is actually asserting.
    fn tree(spec: &[(&str, &str)]) -> TempDir {
        workload_tree(&[(WORKLOAD, spec)])
    }

    /// The same, for more than one workload: each declares only its own directories.
    fn workload_tree(spec: &[(&str, &[(&str, &str)])]) -> TempDir {
        let root = TempDir::new().unwrap();
        for (id, implementations) in spec {
            let workload_dir = root.path().join(id);
            create_dir_all(&workload_dir).unwrap();

            for (path, manifest) in *implementations {
                let dir = workload_dir.join(path);
                create_dir_all(&dir).unwrap();
                File::create(dir.join("Dockerfile")).unwrap();
                write(dir.join(SOURCE), SOURCE_TEXT).unwrap();

                let manifest = if manifest.contains("source:") {
                    (*manifest).to_owned()
                } else {
                    format!("{manifest}source: {SOURCE}\n")
                };
                write(dir.join(MANIFEST), manifest).unwrap();
            }

            let declared: Vec<&str> = implementations.iter().map(|(path, _)| *path).collect();
            write(
                workload_dir.join(workload::MANIFEST),
                workload_manifest(id, &declared),
            )
            .unwrap();
        }
        root
    }

    fn one(manifest: &str) -> Result<Vec<Implementation>> {
        discover(tree(&[("anywhere", manifest)]).path(), WORKLOAD)
    }

    #[test]
    fn the_source_is_read_off_disk_and_its_size_recorded() {
        let found = one(C_GCC).unwrap();
        assert_eq!(found[0].source.file_name().unwrap(), SOURCE);
        assert_eq!(found[0].source_bytes, SOURCE_BYTES);
    }

    /// A `source:` that resolves to nothing would publish a byte count of zero for
    /// a kernel that plainly exists — and a zero is the one answer a size column
    /// must never invent. Loud, at discovery, and not an hour into the campaign.
    #[test]
    fn a_source_that_is_not_there_fails_the_campaign() {
        let error = one("language: c\n\
             compiler: gcc\n\
             source: typo.c\n\
             modes: all\n\
             description: The reference C kernel.\n")
        .unwrap_err();
        let error = format!("{error:#}");
        assert!(error.contains("typo.c"), "{error}");
    }

    /// The manifest must say which file is the kernel. The alternative is for the
    /// harness to guess from the names beside it — which is parsing the path, under
    /// another name, and the path is not metadata.
    #[test]
    fn a_manifest_that_declares_no_source_fails_the_campaign() {
        let error = one("language: c\n\
             compiler: gcc\n\
             source:\n\
             modes: all\n\
             description: The reference C kernel.\n")
        .unwrap_err();
        assert!(format!("{error:#}").contains("source"));
    }

    /// The path locates the manifest and says nothing else. Every fact about the
    /// implementation — including which workload it computes — comes out of the
    /// file, at whatever depth the file happens to sit.
    #[test]
    fn the_manifest_and_not_the_path_describes_the_implementation() {
        let root = tree(&[(
            "some/deeply/nested/folder",
            "language: python\n\
             compiler: cython\n\
             interpreter: cpython\n\
             modes: [strict]\n\
             description: Cython compiles the kernel; CPython runs the result.\n\
             comments: Slower than the interpreter it compiles.\n",
        )]);
        let found = discover(root.path(), WORKLOAD).unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].workload, "mandelbrot");
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

        let interpreted = one("language: python\n\
                               interpreter: cpython\n\
                               modes: [strict]\n\
                               description: CPython, no compiler.\n")
        .unwrap();
        assert_eq!(interpreted[0].slug(), "python-cpython");

        let both = one("language: python\n\
                        compiler: cython\n\
                        interpreter: cpython\n\
                        modes: [strict]\n\
                        description: Both.\n")
        .unwrap();
        assert_eq!(both[0].slug(), "python-cython-cpython");
    }

    /// The ordinary case, and the reason the field has a default: a toolchain that
    /// exists everywhere says nothing at all.
    #[test]
    fn a_manifest_that_says_nothing_about_arch_builds_on_both() {
        let found = one(C_GCC).unwrap();
        assert_eq!(found[0].architectures, Architecture::ALL);
        assert!(found[0].supports(Some(Architecture::X86_64)));
        assert!(found[0].supports(Some(Architecture::Aarch64)));
    }

    /// Kotlin/Native publishes no `linux-aarch64` host compiler. That is not a
    /// preference to be overridden, it is a toolchain that does not exist — so the
    /// backend declares where it can be built, and an AArch64 campaign skips it.
    #[test]
    fn a_backend_may_declare_it_only_builds_on_one_architecture() {
        let found = one("language: kotlin\n\
                         compiler: kotlin-native\n\
                         modes: [strict]\n\
                         architectures: [x86_64]\n\
                         description: No linux-aarch64 host compiler exists.\n")
        .unwrap();

        assert_eq!(found[0].architectures, [Architecture::X86_64]);
        assert!(found[0].supports(Some(Architecture::X86_64)));
        assert!(!found[0].supports(Some(Architecture::Aarch64)));
    }

    /// `all` means "both of the two the harness knows", never "whatever this host
    /// happens to be": a third architecture has no architecture baseline to pin, so nothing
    /// can be claimed about it.
    #[test]
    fn an_unknown_host_architecture_supports_nothing() {
        assert!(!one(C_GCC).unwrap()[0].supports(None));
    }

    #[test]
    fn a_misspelled_architecture_fails_the_campaign() {
        let error = one("language: c\n\
                         compiler: gcc\n\
                         modes: all\n\
                         architectures: [x86-64]\n\
                         description: An architecture spelled the -march way, not the uname way.\n")
        .unwrap_err();
        assert!(format!("{error:#}").contains("x86-64"), "{error:#}");
    }

    #[test]
    fn a_directory_without_a_manifest_is_not_an_implementation() {
        let root = tree(&[("benchmarks/c-gcc", C_GCC)]);
        create_dir_all(root.path().join("benchmarks/notes")).unwrap();
        write(root.path().join("benchmarks/notes/README.md"), "hi").unwrap();

        let found = discover(root.path(), WORKLOAD).unwrap();
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn the_schedule_is_ordered_by_identity_not_by_where_the_files_sit() {
        let root = tree(&[
            ("zzz", C_GCC),
            (
                "aaa",
                "language: python\n\
                 interpreter: cpython\n\
                 modes: [strict]\n\
                 description: CPython.\n",
            ),
        ]);
        let slugs: Vec<String> = discover(root.path(), WORKLOAD)
            .unwrap()
            .iter()
            .map(Implementation::slug)
            .collect();
        assert_eq!(slugs, ["c-gcc", "python-cpython"]);
    }

    /// A campaign measures **one** workload, and sees only the implementations that
    /// workload declares. The other workload's are not filtered out afterwards —
    /// they are never read: a campaign asks a workload what it is implemented by.
    #[test]
    fn a_campaign_sees_only_the_implementations_its_workload_declares() {
        let root = workload_tree(&[
            ("mandelbrot", &[("c", C_GCC)]),
            ("nbody", &[("c", C_GCC), ("rust", RUST)]),
        ]);

        let mandelbrot = discover(root.path(), "mandelbrot").unwrap();
        assert_eq!(mandelbrot.len(), 1);
        assert_eq!(mandelbrot[0].workload, "mandelbrot");

        let nbody = discover(root.path(), "nbody").unwrap();
        assert_eq!(nbody.len(), 2);
        assert!(nbody.iter().all(|found| found.workload == "nbody"));
    }

    /// The same triple under a different workload is a different implementation:
    /// `c-gcc` computing Mandelbrot and `c-gcc` computing n-body are two rows, in two
    /// campaigns, and neither collides with the other.
    #[test]
    fn the_same_backend_may_implement_two_workloads() {
        let root = workload_tree(&[("mandelbrot", &[("c", C_GCC)]), ("nbody", &[("c", C_GCC)])]);

        assert_eq!(
            discover(root.path(), "mandelbrot").unwrap()[0].slug(),
            "c-gcc"
        );
        assert_eq!(discover(root.path(), "nbody").unwrap()[0].slug(), "c-gcc");
    }

    /// A workload asked for by a name nobody declares is a typo, and the error says
    /// what *was* declared — the reader is one letter away from the answer.
    #[test]
    fn an_unknown_workload_fails_the_campaign() {
        let root = tree(&[("anywhere", C_GCC)]);
        let error = discover(root.path(), "mandlebrot").unwrap_err();
        let error = format!("{error:#}");
        assert!(error.contains("mandlebrot"), "{error}");
        assert!(error.contains("mandelbrot"), "{error}");
    }

    /// A directory a workload declares, with no `bench.yaml` in it — moved, renamed,
    /// not written yet. It is warned about and skipped: a campaign is not held
    /// hostage by an empty folder, and the rest of the row still gets measured.
    #[test]
    fn a_declared_directory_with_no_manifest_is_skipped() {
        let root = tree(&[("here", C_GCC)]);
        write(
            root.path().join(WORKLOAD).join(workload::MANIFEST),
            workload_manifest(WORKLOAD, &["here", "gone"]),
        )
        .unwrap();

        let found = discover(root.path(), WORKLOAD).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].slug(), "c-gcc");
    }

    #[test]
    fn the_all_keyword_declares_every_mode() {
        assert_eq!(one(C_GCC).unwrap()[0].fp_modes, FpMode::ALL);
    }

    #[test]
    fn modes_may_be_an_explicit_list() {
        let found = one("language: go\n\
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
        let error = one("language: c\n\
                         compiler: gcc\n\
                         modes: [stcirt]\n\
                         description: Typo.\n")
        .unwrap_err();
        assert!(format!("{error:#}").contains("stcirt"), "{error:#}");
    }

    #[test]
    fn a_backend_that_neither_compiles_nor_interprets_fails_the_campaign() {
        let error = one("language: c\n\
                         modes: all\n\
                         description: Nothing turns this into instructions.\n")
        .unwrap_err();
        assert!(format!("{error:#}").contains("compiler"), "{error:#}");
    }

    #[test]
    fn an_unknown_key_fails_the_campaign() {
        let error = one("language: c\n\
                         compiler: gcc\n\
                         modes: all\n\
                         description: Fine.\n\
                         compilr: gcc\n")
        .unwrap_err();
        assert!(format!("{error:#}").contains("compilr"), "{error:#}");
    }

    #[test]
    fn a_missing_description_fails_the_campaign() {
        let error = one("language: c\ncompiler: gcc\nmodes: all\n").unwrap_err();
        assert!(format!("{error:#}").contains("description"), "{error:#}");
    }

    /// The manifest declares an implementation; without a Dockerfile there is
    /// nothing to build it from, and a missing row would be the only symptom.
    #[test]
    fn a_manifest_without_a_dockerfile_fails_the_campaign() {
        let root = TempDir::new().unwrap();
        let workload_dir = root.path().join(WORKLOAD);
        let dir = workload_dir.join("c-gcc");
        create_dir_all(&dir).unwrap();
        write(dir.join(MANIFEST), C_GCC).unwrap();
        write(
            workload_dir.join(workload::MANIFEST),
            workload_manifest(WORKLOAD, &["c-gcc"]),
        )
        .unwrap();

        let error = discover(root.path(), WORKLOAD).unwrap_err();
        assert!(format!("{error:#}").contains("Dockerfile"), "{error:#}");
    }

    /// Two manifests, one identity: they would build the same image tag and
    /// collapse into one row, and which of the two descriptions the report would
    /// print is a coin toss.
    #[test]
    fn the_same_identity_declared_twice_fails_the_campaign() {
        let root = tree(&[("here", C_GCC), ("there", C_GCC)]);
        let error = discover(root.path(), WORKLOAD).unwrap_err();
        assert!(format!("{error:#}").contains("c-gcc"), "{error:#}");
    }

    #[test]
    fn selected_modes_intersect_the_request_with_the_declaration() {
        let found = one("language: python\n\
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
                "language: rust\n\
                 compiler: llvm\n\
                 modes: [stcirt]\n\
                 description: A misspelled mode.\n",
            ),
            (
                "nameless",
                "language: go\n\
                 modes: all\n\
                 description: Neither compiled nor interpreted.\n",
            ),
        ]);

        let error = validate(&[root.path().to_path_buf()]).unwrap_err();
        assert!(format!("{error:#}").contains('2'), "{error:#}");
    }

    #[test]
    fn validate_walks_the_directories_it_is_given() {
        let root = tree(&[("c-gcc", C_GCC)]);
        assert_eq!(validate(&[root.path().to_path_buf()]).unwrap(), 1);
    }

    /// A single manifest cannot be checked on its own, and the error says why rather
    /// than quietly checking half of what was asked.
    #[test]
    fn validate_refuses_a_single_manifest() {
        let root = tree(&[("c-gcc", C_GCC)]);
        let manifest = root.path().join(WORKLOAD).join("c-gcc").join(MANIFEST);
        let error = validate(&[manifest]).unwrap_err();
        assert!(format!("{error:#}").contains("directories"), "{error:#}");
    }

    /// The one absence a walk can see and a declaration cannot: a `bench.yaml` on
    /// disk that no workload lists. A campaign reads the list, so it would never
    /// build this backend, never measure it, and never miss it — and a row that is
    /// not in the table reads exactly like a backend nobody wrote.
    #[test]
    fn validate_catches_a_manifest_no_workload_declares() {
        let root = tree(&[("c-gcc", C_GCC)]);

        // A second implementation on disk, absent from the workload's list.
        let stray = root.path().join(WORKLOAD).join("rust-rustc");
        create_dir_all(&stray).unwrap();
        File::create(stray.join("Dockerfile")).unwrap();
        write(stray.join(SOURCE), SOURCE_TEXT).unwrap();
        write(stray.join(MANIFEST), RUST).unwrap();

        let error = validate(&[root.path().to_path_buf()]).unwrap_err();
        assert!(format!("{error:#}").contains("rejected"), "{error:#}");
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
        for key in ["language", "compiler", "interpreter", "description"] {
            assert!(schema.contains(&format!("\"{key}\"")), "{key} is missing");
        }
        // The workload is *not* here: an implementation no longer names the workload
        // it implements — the workload names its implementations.
        assert!(!schema.contains("\"workload\""), "{schema}");
        // The three modes are offered to an editor as constants, not as "a
        // string": completing `strict` is the point of shipping a schema.
        for mode in FpMode::ALL {
            assert!(schema.contains(&format!("\"const\": \"{mode}\"")), "{mode}");
        }
        // A misspelled key must fail the campaign, and the schema must say so.
        assert!(schema.contains("\"additionalProperties\": false"));
        // The two architectures are offered as constants too, for the same reason.
        for architecture in Architecture::ALL {
            assert!(
                schema.contains(&format!("\"const\": \"{architecture}\"")),
                "{architecture}"
            );
        }
    }
}
