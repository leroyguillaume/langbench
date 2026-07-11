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

- Never `-march=native`. Pin a baseline per ISA as a build arg.
- Three FP modes — `strict`, `fma`, `fast` — as build args on the same source.
- Pin `codegen-units`, `strip` and the linker explicitly.

**Correctness** ([why](METHODOLOGY.md#the-strict-mode-invariant))

- The checksum is a **64-bit integer**, everywhere, always. Never a float, never
  through a system that stores floats.
- In `strict` mode the checksum is bit-identical across every compiler, language
  and ISA. One reference value. A divergence is a bug, never a rounding excuse.
- Verify the checksum on **every** run. A wrong run never enters the statistics.
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

- Every implementation declares itself in a `bench.yaml` beside its Dockerfile:
  `algo`, `language`, `compiler`, `interpreter`, `modes`, `description`,
  `comments`. **Discovery is a walk for `bench.yaml`; nothing else is read.**
- **The path is not metadata.** Never parse a directory name. The tree is
  free-form: move a benchmark, nest it, rename it — the campaign is unchanged.
- **An implementation is `(algo, language, compiler, interpreter)`.** No name, no
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
- `bench.schema.json` (repo root) is **generated** by `langbench jsonschema` from
  the struct the harness deserializes. Never edit it by hand; a pre-commit hook
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
- **`run` writes `samples.ndjson` and nothing else.** Rendering is not part of
  measuring: `langbench csv` and `langbench md` are separate commands, pure
  functions of the file. A report that a run could emit directly is a report that
  can outlive the samples it came from.
- Report min-of-N, not the median: contention noise is one-sided. Publish the
  dispersion beside it as a verdict on the campaign.

**Never** ([why](METHODOLOGY.md#never-push-benchmark-metrics-to-prometheus))

- Never push benchmark metrics to Prometheus, or any TSDB. Prometheus is for the
  bench machine's health (frequency, temperature, throttling), never for the
  measurement.
- Never publish an absolute cross-ISA timing. Within-ISA ratios only.
  ([why](METHODOLOGY.md#the-isa-rule))
- Never run a benchmark under QEMU / `binfmt` emulation.

**The website** (`site/`)

- The site is a **third rendering**, beside `csv` and `md`, and obeys the same
  rule: a pure function of `samples.ndjson`. It measures nothing, and CI never
  measures anything either — a shared, virtualised, frequency-scaled runner is the
  worst benchmark target money can rent.
- **The site computes no statistic.** Min-of-N, the buckets, the definition of
  startup all live in `src/analysis.rs`, compiled to WebAssembly (`src/wasm.rs`)
  and called from the browser. `langbench md` calls the same function. A
  re-implementation in TypeScript would be a second definition of what this
  project measures — the same drift `bench.schema.json` is generated to prevent.
  TypeScript sorts, formats and draws; it never does arithmetic on a sample.
- **The site never calls `JSON.parse` on a campaign.** `checksum` is a 64-bit
  integer, a JavaScript number is a double, and `JSON.parse` silently rounds past
  2^53. `samples.ndjson` is fetched as *text* and parsed in Rust; checksums cross
  the wire as **strings** and are never added, only displayed and compared.
- The site's data files **are** the campaigns in `samples/<arch>.ndjson`, byte for
  byte, and each `reports/<arch>.md` is rendered from the campaign of the same
  name. No export format, no intermediate file: the raw samples are the only thing
  that cannot be recomputed, so they are what gets published.
- **One campaign per ISA, and the site shows one at a time.** An absolute timing
  never crosses an ISA, so two architectures are never in one chart, one bar group
  or one table. The site reads the ISA out of the machine record *inside* each
  campaign — never out of the filename, which is a label somebody typed. `bench`
  runs the matrix on native runners; never QEMU.
- `src/lib.rs` carves the crate in two: the `cli` feature owns everything that
  touches the machine (Docker, discovery, the campaign, Liquid); what is left is
  data and arithmetic, and it compiles to `wasm32-unknown-unknown`. Nothing that
  spawns a process belongs in `analysis`, `sample`, `stats` or `mode`.
- The wire is `snake_case` throughout — the sample's own vocabulary, from the
  NDJSON to the CSV to the browser. One vocabulary, no translation table.

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
  `include_str!` so the binary stays self-contained. `langbench md --template`
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
