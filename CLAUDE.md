# CLAUDE.md

Instructions for `langbench`. Complements the global rules in
`~/.claude/CLAUDE.md`; nothing here overrides them.

**The reasoning behind every rule below lives in [METHODOLOGY.md](METHODOLOGY.md).**
Each rule links to its section. If a rule looks like excessive caution, read the
section before removing it — they all exist because the naive alternative
silently produces wrong numbers.

## What this is

A Rust CLI that discovers benchmark implementations on disk, builds one container
per implementation, runs them under a controlled protocol, and emits raw samples —
rendered afterwards as a CSV or a Markdown report by separate commands. The
subject is **compiler and runtime backends**, not languages.

## Terminology

These eight words mean exactly this, everywhere: in the code, in the manifests, on
the wire, in the report, on the site, in a commit message. Every rule below is
written in them.

- **workload** — the work itself, declared in a `workload.yaml`: what it is, how it
  is sized (`params`), what the right answer is (`checksum`), and which
  directories implement it. **A workload is not an algorithm.** Mandelbrot is one;
  a JSON parser, an HTTP server, a cold start are others. Nothing in the harness may
  assume the work is a computation over a grid.
- **backend** — `(language, compiler, interpreter)`: what executes. Either of the
  last two may be absent, and an absence is a published fact. This is the subject.
- **implementation** — a backend doing a given workload: one `bench.yaml`, one
  Dockerfile, one source file.
- **mode** — `strict` / `fma` / `fast`. The FP semantics, a build arg on one source.
- **unit** — `(implementation, mode)`. The atom of the schedule, and the grain of
  quarantine: a failure takes out a unit, never the campaign.
- **sample** — one measured invocation. One NDJSON line.
- **campaign** — every sample from one pass over the matrix, on **one machine, one
  workload**. It is what gets published, and it is what does not compare to another
  machine's.
- **matrix** — the definition of what a campaign will measure: the implementations a
  workload declares, crossed with the modes requested.

Two rules follow from the vocabulary rather than from the methodology, and they are
the ones a reader is most likely to think were violated:

- **A campaign is `(machine, workload)`, and its header carries the whole workload
  manifest**, snapshotted. Not the id: the manifest. `workload.yaml` will be edited —
  params retuned, a description rewritten, a reference added — and none of that is
  retroactive. A campaign says what it *ran*. The site only ever fetches samples, so
  the snapshot is also the only way it can know what the work was.
- **The path is still not metadata, and the declaration is why.** A workload lists
  the directories it is implemented in; discovery reads that list. It does *not*
  recurse and take whatever `bench.yaml` it finds — that would make the position of a
  directory decide whether it is measured, which is the path being metadata under
  another name. The one search left in the harness is the walk for `workload.yaml`
  files themselves. The cost is that a manifest can be forgotten, so `validate` walks
  the tree and fails on any `bench.yaml` no workload claims.

## Rules

**Benchmark kernels** ([why](METHODOLOGY.md#the-benchmark-mandelbrot))

- Zero third-party dependencies. One source file per implementation. Rust uses
  `std::thread` + `AtomicUsize`, never `rayon`.
- `n`, `max_iter` and the thread count come from `argv`. Never compile-time
  constants (constant folding), and the checksum is always printed (dead-code
  elimination).
- Kernels never auto-detect CPU count. The harness resolves a default and passes
  it explicitly. Runtimes disagree about cgroup quotas; auto-detection would
  measure that disagreement.
- Work chunks are handed out dynamically, at least `4 × threads` of them. The
  load is imbalanced by design.

**Flags** ([why](METHODOLOGY.md#compiler-flags))

- Never `-march=native`. Pin a baseline per architecture as a build arg.
- Three FP modes — `strict`, `fma`, `fast` — as build args on the same source.
- Pin `codegen-units`, `strip` and the linker explicitly.

**Correctness** ([why](METHODOLOGY.md#the-strict-mode-invariant))

- The checksum is a **64-bit integer**, everywhere, always. Never a float, never
  through a system that stores floats. A workload whose answer *is* a float bit-casts
  it; it does not print it.
- In `strict` mode the checksum is bit-identical across every compiler, language
  and architecture. One reference value. A divergence is a bug, never a rounding excuse.
- Verify the checksum on **every** run. A wrong run never enters the statistics.
- **A deterministic workload declares its `checksum`.** It is `Option` only because
  some work has no answer — a throughput, a cold start, anything the scheduler
  decides — and a campaign on a workload that declares none **warns**, loudly: without
  it there is no correctness gate at all, and a backend that computes nothing and
  returns instantly tops the table. It is not called `strict_checksum`, because
  `strict` is a floating-point mode and a JSON parser has no floating-point semantics
  to be strict about; the answer is the answer whatever the mode.
- **A backend that fails is quarantined, not propagated.**
  ([why](METHODOLOGY.md#a-backend-that-fails-is-not-a-campaign-that-fails)) A build
  that fails, a container that crashes or hangs past the timeout, unreadable
  stdout, a diverging checksum: each takes out that one `(implementation, mode)`
  unit, at the point it breaks, and never a second time — the campaign keeps
  measuring the rest and exits 0. Only a campaign where *every* unit failed exits
  non-zero: an empty table is a lie told quietly.
- **A failure is published, never swallowed.** It is a `failure` record in
  `samples.ndjson`, beside the samples, and every rendering shows it. A row missing
  from a report reads exactly like a backend nobody wrote.

**Layout** ([why](METHODOLOGY.md#repository-layout))

- Every workload declares itself in a `workload.yaml`: `id`, `description`, `params`,
  `implementations`, and an optional `checksum`. **The walk for
  `workload.yaml` is the only search the harness does.**
- Every implementation declares itself in a `bench.yaml` beside its Dockerfile:
  `language`, `compiler`, `interpreter`, `source`, `modes`, `architectures`,
  `description`, `comments`. It does **not** name its workload — the workload names
  it, and a manifest nobody names is caught by `validate`.
- `params` is an ordered **list**, never a mapping: the order is the `argv` order the
  kernels receive (`run <params…> <threads>`), and a list is the only YAML shape that
  is ordered by construction. The thread count is not a param — it is a property of
  the machine, resolved by the harness.
- **How the work is sized is a property of the work.** Never a flag of the harness:
  `--grid-size` was Mandelbrot leaking into the CLI. `--param name=value` overrides a
  declared param, and doing so drops the declared `checksum` — it is the answer
  to the declared work, not to this one.
- `source` names the one kernel file, and the manifest **declares** it — the harness
  never guesses which file beside the Dockerfile is the source. Guessing means
  pattern-matching a filename, which is parsing the path under another name. A
  `source` that is not a file on disk fails the campaign at discovery.
- **The path is not metadata.** Never parse a directory name, and never recurse from
  a workload to collect whatever `bench.yaml` sits beneath it. The tree is free-form:
  move a benchmark, nest it, rename it, and the campaign is unchanged as long as the
  workload still lists it.
- **An implementation is `(workload, language, compiler, interpreter)`.** No name, no
  slug in the data. `compiler` and `interpreter` are each optional — but not both,
  and an absence is a published fact, not a hole. The same tuple declared twice is
  an error.
- `modes: all`, or an explicit list. A misspelled mode fails the campaign; a mode
  that is requested but not declared is skipped with a warning.
- Docker `LABEL`s are image provenance for `docker inspect` (`.version`,
  `.flags`). **The harness never reads them** — two sources of truth is one source
  of drift. Anything the harness acts on lives in the manifest.
- Every sample carries its backend's manifest fields (language, compiler,
  interpreter, description, comments). Deliberate repetition: a sample must
  describe itself without joining against a file that will change.
- In telemetry, emit `language`, `compiler`, `interpreter` as separate fields —
  never a slug. A log line is filtered by field.
- `bench.schema.json` and `workload.schema.json` (repo root) are **generated** by
  `langbench implementation jsonschema` and `langbench workload jsonschema`, from
  the structs the harness deserializes. Never edit them by hand; a pre-commit hook
  fails on drift. `langbench validate` reports every invalid manifest at once,
  and a hook runs it whenever a `bench.yaml` moves.
- One Dockerfile per implementation. No templating. Base images pinned by digest,
  never by tag. Non-root `USER` in every benchmark Dockerfile.

**Measurement** ([why](METHODOLOGY.md#measurement-protocol))

- `docker build` prepares, `docker run` measures. Never time a `docker build`.
- `--network=none` and `--tmpfs` on every measured run. The former is a
  structural guarantee, not a convention — do not trade it away.
- CPU time comes from the container's `/sys/fs/cgroup/cpu.stat`. Never from
  `rusage` of the `docker` client process, which measures argument parsing.
- Peak memory comes from the container's `/sys/fs/cgroup/memory.peak`, the same way.
  It is the **whole container** — process tree, page cache, tmpfs — not one process's
  RSS. Min-of-N, and here the argument is exact rather than statistical: nothing can
  push a high-water mark below what the backend had to allocate.
- **`--memory` is pinned, identically, on every measured run, and swap is off.**
  ([why](METHODOLOGY.md#memory-is-only-comparable-under-a-pinned-budget)) It is part
  of the measurement, not a safety rail: a GC runtime sizes its heap from what its
  cgroup shows it, so an unpinned budget publishes a peak that describes the bench
  machine. Changing the budget changes the *timings* too — campaigns run under
  different budgets do not compare, on any column.
- **Parallel efficiency is a median, not a min-of-N.**
  ([why](METHODOLOGY.md#parallel-efficiency-is-a-median-not-a-minimum)) Min-of-N is
  licensed by one-sided noise; contention moves a core count in both directions. It is
  computed per sample, never as a ratio of two minima, and it is allowed to exceed the
  thread count — a runtime's JIT and GC threads burn CPU the kernel's own clock never
  sees, and clamping that would hide the result.
- Record the external wall-clock *and* the program's self-reported `elapsed_ns`.
  The gap is runtime startup cost, and it is a result.
- The **run** column headlines the external wall-clock; the **build** column
  headlines the internal `elapsed_ns`. A runtime's startup is a property of the
  backend; a container's startup is an artefact of our isolation choice, and it is
  several times a `gcc` invocation on one file.
  ([why](METHODOLOGY.md#the-build-column-reports-the-internal-clock-the-run-column-the-external-one))
- `Startup` is the smallest `wall − elapsed` gap *within a single sample*, never
  the difference of two minima drawn from different rounds — that would describe a
  run that never happened.
- Interleave round-robin: outer loop over rounds, inner loop over
  implementations. Never block by implementation.
- **Write raw samples, never aggregates.** One NDJSON line per run, flushed as it
  is produced. Aggregates are recomputed at report time.
- **`workload run` writes `samples.ndjson` and nothing else.** Rendering is not part of
  measuring: `langbench report csv` and `langbench report md` are separate commands, pure
  functions of the file. A report that a run could emit directly is a report that
  can outlive the samples it came from.
- Report min-of-N, not the median: contention noise is one-sided. Publish the
  dispersion beside it as a verdict on the campaign.

**Never** ([why](METHODOLOGY.md#never-push-benchmark-metrics-to-prometheus))

- Never push benchmark metrics to Prometheus, or any TSDB. Prometheus is for the
  bench machine's health (frequency, temperature, throttling), never for the
  measurement.
- Never publish an absolute cross-architecture timing. Within-architecture ratios only.
  ([why](METHODOLOGY.md#the-architecture-rule))
- Never run a benchmark under QEMU / `binfmt` emulation.
- **Never measure energy.** ([why](METHODOLOGY.md#why-there-is-no-energy-column)) The
  campaigns run on GitHub Actions runners, and RAPL is unreadable there: x86-only, and
  root-only on most kernels since PLATYPUS. Every sample of every campaign came back
  `null`. A column that is `n/a` on every row of every published campaign is not a
  measurement, it is a promise the bench machine cannot keep — and the code that reads
  it is code that has never once returned a number.

**The website** (`site/`)

- The site is a **third rendering**, beside `csv` and `md`, and obeys the same
  rule: a pure function of `samples.ndjson`. It measures nothing, and CI never
  measures anything either — a shared, virtualised, frequency-scaled runner is the
  worst benchmark target money can rent.
- **The site computes no statistic.** Min-of-N, the buckets, the definition of
  startup all live in `src/analysis.rs`, and what counts as a *difference* between
  two backends in `src/compare.rs` — both compiled to WebAssembly (`src/wasm.rs`)
  and called from the browser. `langbench report md` calls the same function. A
  re-implementation in TypeScript would be a second definition of what this
  project measures — the same drift `bench.schema.json` is generated to prevent.
  TypeScript sorts, formats and draws; it never does arithmetic on a sample.
- **A gap smaller than the dispersion is a tie, not a win.** The head-to-head
  compares two rows of one campaign; a gap that does not clear the worse of the two
  rows' dispersions is `indistinguishable`, whichever minimum came out lower.
  ([why](METHODOLOGY.md#a-difference-smaller-than-the-dispersion-is-not-a-difference))
- **The site is Astro, and every route is prerendered.** GitHub Pages is a file
  server: `output: 'static'`, so `/compare/` is a real `.html` and a deep link needs
  no `404.html` fallback. Anything that reads a campaign is a React island
  (`client:only`) — there is no campaign at build time, and a page that pretended
  otherwise would ship a chart of numbers nobody measured. `ClientRouter` swaps
  pages without reloading the document, so the module singleton in `campaigns.ts`
  keeps the WASM instance and the parsed campaigns across a navigation: the samples
  are fetched once per tab, not once per page.
- **`METHODOLOGY.md` is copied into the site, never re-written for it.**
  `scripts/data.js` copies the file at the repository root; a second, hand-maintained
  copy would be a methodology that drifts from the one the harness was written
  against.
- **A row is named by its triple, never by a slug.** `language`, `compiler`,
  `interpreter` — the columns `report.md` prints, and the fields a `bench.yaml`
  declares. The `backend` slug on the wire is the handle the WASM picks rows by, and
  the site's use of it stops at that function call: never displayed, never sorted
  on, never in a URL (`?a=java/native-image/-/strict`, not `?a=java-native:strict`).
  `java-native-image` reads as "java, native" and is in fact java + `native-image` +
  no interpreter; a name you have to decode is worse than three fields that say it.
  An absence is a published fact: it renders as `n/a` and is selectable as a filter.
- **The head-to-head asks for a language first**, then the toolchain that ran it,
  then the mode. That is the order a reader asks the questions in.
- **The site never calls `JSON.parse` on a campaign.** `checksum` is a 64-bit
  integer, a JavaScript number is a double, and `JSON.parse` silently rounds past
  2^53. `samples.ndjson` is fetched as *text* and parsed in Rust; checksums cross
  the wire as **strings** and are never added, only displayed and compared.
- The site's data files **are** the campaigns in `samples/<arch>.ndjson`, byte for
  byte, and each `reports/<arch>.md` is rendered from the campaign of the same
  name. No export format, no intermediate file: the raw samples are the only thing
  that cannot be recomputed, so they are what gets published.
- **One campaign per architecture, and the site shows one at a time.** An absolute timing
  never crosses an architecture, so two architectures are never in one chart, one bar group
  or one table. The site reads the architecture out of the machine record *inside* each
  campaign — never out of the filename, which is a label somebody typed. `bench`
  runs the matrix on native runners; never QEMU.
- `src/lib.rs` carves the crate in two: the `cli` feature owns everything that
  touches the machine (Docker, discovery, the campaign, Liquid); what is left is
  data and arithmetic, and it compiles to `wasm32-unknown-unknown`. Nothing that
  spawns a process belongs in `analysis`, `sample`, `stats` or `mode`.
- The wire is `snake_case` throughout — the sample's own vocabulary, from the
  NDJSON to the CSV to the browser. One vocabulary, no translation table. A kebab
  key on the wire is not even reachable: `jq '.elapsed-ns'` reads it as a
  subtraction.
- **A manifest is `kebab-case`.** `workload.yaml` and `bench.yaml` are the only two
  files in this project a *person* types, and that is how YAML is written wherever
  people write it. The two conventions meet in exactly one struct — `Workload`, which
  is both the file you write and the snapshot the campaign records — so it reads kebab
  and writes snake, and carries a serde `alias` so it can read back the header it
  wrote itself. `tests/key_conventions.rs` guards both sides; without it, the only
  symptom of a slip would be a campaign the harness can no longer read.

## Rust specifics

- **No `tokio`, no async.** The harness is deliberately sequential — running two
  benchmarks concurrently would destroy the measurement. The global rule mandates
  `tokio` *when async is needed*; here it is not.
- Samples are appended and flushed one at a time, so an interrupted campaign
  keeps every completed sample. That is durability, and it is only half of
  shutdown: it protects the data, not the machine.
- **Handle `SIGTERM` and `SIGINT`, and kill the container in flight.** The
  workload does not run in this process — it runs on the daemon, in another
  process tree, and `docker run` is only a client attached to it. Killing the
  harness leaves the benchmark running, holding every core it was given, with
  nobody watching. On a bench machine that orphan is not a leak, it is a bias in
  whatever gets measured next. The container is named so it can be reached.
- The interrupted run is **not** a sample. A killed container has no valid record
  to give, and a wrong run never enters the statistics. Stopping means refusing
  the next unit, never writing down what the current one half-said.
- An interrupted campaign **exits 0**. The samples on disk are as valid as they
  were a moment before, and the file still renders; a non-zero exit would claim
  the harness broke, and it did not.
- The default report template is `templates/report.md.liquid`, embedded with
  `include_str!` so the binary stays self-contained. `langbench report md --template`
  overrides it; the built-in one is always the fallback, never a required file.

## Testing

- Unit tests for discovery, manifest parsing, statistics and command construction.
- The kernels themselves are verified by the strict-mode checksum invariant, not
  by unit tests.

## Milestones

1. **Noise floor** on the target machine. Nothing else is trustworthy until this
   number exists. ([why](METHODOLOGY.md#where-it-runs))
2. The C/gcc, C/clang, Rust/LLVM triangle on Mandelbrot, `strict`, x86-64.
3. Everything else.
