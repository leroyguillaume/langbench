# CLAUDE.md

Instructions for `langbench`. Complements the global rules in `~/.claude/CLAUDE.md`;
nothing here overrides them.

**There are two documents, and they answer two different questions.**

- [**The methodology**](site/src/content/methodology.md) — published as a page of the
  website — says **how a number is produced**, and what may be concluded from it. It is
  what a sceptic reads before disputing a result. Every rule below that constrains a
  *measurement* links to the section that justifies it. If one of them looks like
  excessive caution, read the section before removing it: they all exist because the
  naive alternative silently produces wrong numbers.
- **This file** says **how this repository is built** — the manifests, the harness, the
  site, the vocabulary. None of it is a measurement decision. All of it is a decision
  somebody already made badly once.

When you are unsure which document a rule belongs in, ask whether changing it would
change a *number*. If it would, it is methodology. If it would only change the code, it
is this file.

## What this is

A Rust CLI that reads the workloads declared on disk, builds one container per
implementation, runs them under a controlled protocol, and emits raw samples — and stops
there. The **website** is the human rendering of a campaign; `langbench sample convert`
is the machine-readable one. The subject is **compiler and runtime backends**, not
languages.

## Terminology

These eight words mean exactly this, everywhere: in the code, in the manifests, on the
wire, on the site, in a commit message. Every rule below is written in them.

- **workload** — the work itself, declared in a `workload.yaml`: what it is, how it is
  sized (`params`), what the right answer is (`checksum`), and which directories
  implement it. **A workload is not an algorithm.** Mandelbrot is one; a JSON parser, an
  HTTP server, a cold start are others. Nothing in the harness may assume the work is a
  computation over a grid.
- **backend** — `(language, compiler, interpreter)`: what executes. Either of the last
  two may be absent, and an absence is a published fact. This is the subject.
- **implementation** — a backend doing a given workload: one `bench.yaml`, one
  Dockerfile, one source file.
- **mode** — `baseline` / `native`. The **ISA target**: which machine the code is for, as
  one build arg on one source. Not floating-point semantics — that axis existed, and it
  is gone. Every mode is `-O3` and every mode is strict IEEE 754, so a wider vector
  reorders no arithmetic and the checksum gates *both*. Which modes a backend *can*
  offer is the subject: an ahead-of-time compiler must choose a machine and so has both;
  a JIT compiles on the machine it runs on and has only `native`, which is not its
  limitation but its selling point.
- **unit** — `(implementation, mode)`. The atom of the schedule, and the grain of
  quarantine: a failure takes out a unit, never the campaign.
- **sample** — one measured invocation. One NDJSON line.
- **campaign** — every sample from one pass over the matrix, on **one machine, one
  workload**. It is what gets published, and it is what does not compare to another
  machine's.
- **matrix** — what a campaign will measure: the implementations a workload declares,
  crossed with the modes requested.

## Declaring the work

Two manifests, because there are two things to declare, and they are not the same thing.
A workload is the work; an implementation is a backend that does it. **The manifests are
the only thing the harness reads.**

- Every workload declares itself in a `workload.yaml`: `id`, `description`, `params`,
  `implementations`, and an optional `checksum`. **The walk for `workload.yaml` is the
  only search the harness does.**
- Every implementation declares itself in a `bench.yaml` beside its Dockerfile:
  `language`, `compiler`, `interpreter`, `source`, `modes`, `architectures`,
  `description`, `comments`. It does **not** name its workload — the workload names it,
  and a manifest nobody names is caught by `validate`.
- **The path is not metadata.** Never parse a directory name, and never recurse from a
  workload to collect whatever `bench.yaml` sits beneath it. An earlier design inferred
  the language and the compiler from `<language>-<compiler>/` and read the rest back out
  of Docker labels; both are gone, and for the same reason. A path is a two-field record
  with no room for a third: `python-cython` is a directory name that *cannot say* that
  CPython also runs the result — and that omission is not cosmetic, because the whole
  value of that row is that it shares a language **and an interpreter** with
  `python-cpython` and differs only in the compiler. The tree is therefore free-form:
  move a benchmark, nest it, rename it, and the campaign is unchanged as long as the
  workload still lists it. The cost of declaring is that a manifest can be forgotten, so
  `langbench validate` walks the tree and fails on any `bench.yaml` no workload claims —
  a row absent from a table reads exactly like a backend nobody wrote.
- **An implementation is `(workload, language, compiler, interpreter)`.** No name, no
  slug in the data — a name is a second thing to keep in sync with the first. `compiler`
  and `interpreter` are each optional, but not both, and an absence is a published fact:
  gcc compiles and nothing interprets, CPython interprets and nothing compiles ahead of
  the run, Cython does both. The same tuple declared twice is an error: the two would
  build the same image tag and collapse into one row, and which description got printed
  would be a coin toss.
- **The answer belongs to the work.** `checksum` is a property of `(workload, params)`,
  so it lives with the workload — not in the harness, and not in a backend. Without it a
  campaign can only check that its backends agree *with each other*, which passes a
  campaign where every backend is wrong the same way. `--param name=value` overrides a
  declared param and **drops the declared checksum**, because it is the answer to the
  declared work and not to this one; the campaign says so and falls back to the weaker
  check. It is not called `strict_checksum`, and no longer could be: `strict` was a
  floating-point mode, and that axis is gone. The answer is the answer whatever the mode,
  and **every mode is now held to it** — the old `fma` and `fast` rows were licensed to
  diverge, which meant the correctness gate was switched off for the two modes most
  likely to expose a miscompilation.
- **How the work is sized is a property of the work**, never a flag of the harness:
  `--grid-size` was Mandelbrot leaking into the CLI. `params` is an ordered **list**,
  never a mapping — the order is the `argv` order the kernels receive (`run <params…>
  <threads>`), and a list is the only YAML shape that is ordered by construction. The
  thread count is not a param: it is a property of the machine, resolved by the harness.
- `source` names the one kernel file, and the manifest **declares** it. The harness never
  guesses which file beside the Dockerfile is the source — guessing means pattern-matching
  a filename, which is parsing the path under another name. A `source` that is not a file
  on disk fails the campaign at discovery.
- `modes: all`, or an explicit list. A JIT has no ISA target to choose — it generates code
  on the machine it is running on — so a `baseline` image would be *the same run under
  another tag*, and building it would put two rows in a table whose only difference is
  noise. Julia is the exception that proves it is a *capability* and not a category: it is
  a JIT, it takes `--cpu-target`, and it therefore declares both. CPython is the other
  exception, and the only row that is neither: it compiles nothing and has no JIT, so its
  hot loop is an interpreter somebody else built — it declares `baseline` and reports the
  ISA it actually got. A mode that is requested but
  not declared is skipped with a warning; a **misspelled** one fails the campaign — a
  manifest is a deliberate statement, and building three images where the author asked
  for one is a table carrying rows nobody meant to publish.
- `architectures: all`, unless the backend's **toolchain does not exist** for an
  architecture — a fact, not a preference. A campaign on the other machine skips the row
  loudly at discovery rather than failing halfway through a `docker build`.
  ([why](site/src/content/methodology.md#flags-and-the-architecture-baseline))
- **A manifest describes the work, never the results.** An implementation's `comments`
  are what is pinned, what its entrypoint has to do, how it deviates, what its build
  phase actually *is* — never what to expect from the table ("read this against c-gcc",
  "expect them to land close together", "it is slower, and that is a result"). Those are
  claims about a campaign, they change every time one runs, and the campaign is what says
  them. Same for a workload's `description`: it says what the work is, what it puts under
  the light, and — just as loudly — what it says nothing about.
- **Labels are provenance, never input.** Docker `LABEL`s stay on the images
  (`langbench.version`, `langbench.flags`) and describe the artefact for whoever runs
  `docker inspect`. The harness never reads them: two sources of truth is one source of
  drift. There is a hard reason as well as an aesthetic one — `modes` decides *which
  images to build*, so it has to be known before an image exists to inspect.
- One Dockerfile per implementation. **No templating**: per-implementation variance
  (`cargo-chef`, `CGO_ENABLED=0`, `native-image`) lives exactly where templates are
  worst. The ISA target and the job count are **build args** — Docker's own
  parameterization, not a codegen layer of ours. One arg, not two: `MARCH` carries the
  whole mode, since `FP_MODE` had nothing left to say once the arithmetic stopped being
  negotiable. Base images are pinned **by digest**,
  never by tag: a benchmark that silently changes when upstream pushes is not a benchmark.
  Non-root `USER` in every benchmark Dockerfile.
- `bench.schema.json` and `workload.schema.json` (repo root) are **generated** by
  `langbench implementation jsonschema` and `langbench workload jsonschema`, from the
  structs the harness deserializes. Never edit them by hand; a pre-commit hook fails on
  drift, and `langbench validate` reports every invalid manifest at once.
- **A manifest is `kebab-case`; the wire is `snake_case`.** The two manifests are the only
  files in this project a *person* types, and that is how YAML is written wherever people
  write it. Everything downstream — the NDJSON, the CSV, the browser — is the sample's own
  vocabulary, with no translation table: a kebab key on the wire is not even reachable,
  since `jq '.elapsed-ns'` reads it as a subtraction. The two conventions meet in exactly
  one struct — `Workload`, which is both the file you write and the snapshot a campaign
  records — so it reads kebab and writes snake, and carries a serde `alias` so it can read
  back the header it wrote itself. `tests/key_conventions.rs` guards both sides.

## The harness

- **`workload run` writes `samples.ndjson` and nothing else.** Rendering is not part of
  measuring: the site and `langbench sample convert` are both pure functions of that file.
  A table a run could emit directly is a table that can outlive the samples it came from.
  The harness renders nothing for a human — no template engine, no Markdown writer.
- **`sample convert` converts; it never aggregates.** One row per sample, the columns the
  samples carry. The format is a *value* (`--format csv`), never a `--csv` flag: a boolean
  would have to be mandatory — convert to what, otherwise? — and a flag you always type
  says nothing.
- **Write raw samples, never aggregates.** One NDJSON line per run, flushed as it is
  produced, so an interrupted campaign keeps every completed sample. Aggregates are
  recomputed when the samples are read back; a discarded sample is gone forever.
  ([why](site/src/content/methodology.md#sampling-and-what-may-be-concluded))
- **Every sample carries its backend's manifest fields** (language, compiler, interpreter,
  description, comments). Deliberate repetition: a sample must describe itself without
  joining against a file that will change. A campaign's header carries the whole workload
  manifest, snapshotted — not the id, the manifest. A campaign says what it *ran*, and
  editing `workload.yaml` afterwards is not retroactive.
- **A backend that fails is quarantined, not propagated.** A build that fails, a container
  that crashes or hangs past the timeout, unreadable stdout, a diverging checksum: each
  takes out that one unit, at the point it breaks, and never a second time. The campaign
  keeps measuring the rest and exits 0. Only a campaign where *every* unit failed exits
  non-zero — an empty table is a lie told quietly. The failure is **published**, as a
  `failure` record beside the samples, and every rendering shows it.
  ([why](site/src/content/methodology.md#sampling-and-what-may-be-concluded))
- **Handle `SIGTERM` and `SIGINT`, and kill the container in flight.** The workload does
  not run in this process — it runs on the daemon, in another process tree, and `docker
  run` is only a client attached to it. Killing the harness leaves the benchmark running,
  holding every core it was given, with nobody watching: on a bench machine that orphan is
  not a leak, it is a bias in whatever gets measured next. The container is named so it can
  be reached.
- The interrupted run is **not** a sample, and an interrupted campaign **exits 0**. A
  killed container has no valid record to give; stopping means refusing the next unit,
  never writing down what the current one half-said. The samples on disk are as valid as
  they were a moment before, and a non-zero exit would claim the harness broke.
- **No `tokio`, no async.** The harness is deliberately sequential — running two benchmarks
  concurrently would destroy the measurement. The global rule mandates `tokio` *when async
  is needed*; here it is not.
- `src/lib.rs` carves the crate in two: the `cli` feature owns everything that touches the
  machine (Docker, discovery, the campaign); what is left is data and arithmetic, and it
  compiles to `wasm32-unknown-unknown`. **Nothing that spawns a process belongs in
  `analysis`, `sample`, `stats`, `compare` or `mode`** — the website calls those.
- In telemetry, emit `language`, `compiler`, `interpreter` as separate fields, never a
  slug. A log line is filtered by field.

## The measurement invariants the code has to uphold

The reasoning for every one of these is in the methodology; what is written here is what a
change to the harness must not break.

- `docker build` prepares, `docker run` measures. Never time a `docker build`.
- `--network=none` and `--tmpfs` on every measured run. The former is a structural
  guarantee, not a convention — do not trade it away.
- CPU time and peak memory come from the container's own cgroup (`cpu.stat`,
  `memory.peak`), read inside the container. Never from `rusage` of the `docker` client,
  which measures argument parsing.
  ([why](site/src/content/methodology.md#how-a-run-is-measured))
- **`--memory` is pinned, identically, on every measured run, and swap is off.** It is part
  of the measurement, not a safety rail; campaigns run under different budgets do not
  compare, on any column.
- Record the external wall-clock *and* the program's self-reported `elapsed_ns`. The gap is
  runtime startup, and it is a result. `Startup` is the smallest gap *within a single
  sample*, never the difference of two minima — that would describe a run that never
  happened. The **run** column headlines the external clock; the **build** column headlines
  the internal one.
- Report min-of-N, and publish the dispersion beside it as a verdict on the campaign.
  **Parallel efficiency is the exception**: a median, per sample, allowed to exceed the
  thread count.
- Verify the checksum on **every** run. A wrong run never enters the statistics. The
  checksum is a **64-bit integer**, everywhere, always — never a float, never through a
  system that stores floats.
- Interleave round-robin: outer loop over rounds, inner loop over implementations. Never
  block by implementation.

## The website (`site/`)

- The site is **the** human rendering of a campaign — there is no second one — and it obeys
  the rule the CSV does: a pure function of `samples.ndjson`. It measures nothing, and CI
  never measures anything either.
- **The site computes no statistic.** Min-of-N, the buckets, the definition of startup live
  in `src/analysis.rs`, and what counts as a *difference* between two backends in
  `src/compare.rs` — both compiled to WebAssembly and called from the browser. A
  re-implementation in TypeScript would be a second definition of what this project
  measures, the same drift `bench.schema.json` is generated to prevent. TypeScript sorts,
  formats and draws; it never does arithmetic on a sample.
- **The site's shape is the vocabulary's.** A sidebar of workloads, the campaigns of each
  nested under it, and one page apiece: the front page says what langbench does, a
  workload's page is its manifest, a campaign's page is its results. A campaign hangs under
  its workload because that is what a campaign *is* — one machine, one workload — and an
  architecture is never a top-level thing to pick.
- **The campaign is the route, never a query string.** `/workloads/<workload>/<arch>/`, and
  the island resolves it against the campaign's own *header* — never the filename, and with
  no fallback. A page whose address says `x86_64` and whose numbers came from an AArch64 run
  is the worst thing this project could publish, and every number on it would be internally
  consistent. What stays in the query string is how you are looking at the rows: the
  filters, the sort, the warmup toggle.
- **A measurement is explained on `/measurements/`, never under the table that prints it.**
  The columns are the same on every campaign — they *are* what this project measures — so
  `site/src/content/measurements.md` explains them once and every results table links to it.
  It is not called `/data/`: the data is `samples.ndjson`, served under that very path, and
  this page contains none of it.
- **A backend is described on the *workload's* page, never under a campaign's table.** What
  an implementation is comes from its `bench.yaml`: it does not change with the machine that
  ran it, and it exists before any campaign has. The declared set is not the measured set —
  a backend added today has no row yet, and one that crashed has a failure instead of one.
  Each row of a results table links back to the card, at the anchor of its triple. The site
  reads the manifests through the harness (`langbench workload list --json`,
  `langbench implementation list --json`), never with a YAML parser of its own.
- **A row is named by its triple, never by a slug.** The `backend` slug on the wire is the
  handle the WASM picks rows by, and the site's use of it stops at that function call: never
  displayed, never sorted on, never in a URL (`?a=java/native-image/-/native`, not
  `?a=java-native:native`). `java-native-image` reads as "java, native" and is in fact java +
  `native-image` + no interpreter; a name you have to decode is worse than three fields that
  say it. An absence is a published fact: it renders as `n/a` and is selectable as a filter.
- **A gap smaller than the dispersion is a tie, not a win.** The head-to-head asks for a
  language first, then the toolchain that ran it, then the mode — the order a reader asks the
  questions in — and a gap that does not clear the worse of the two rows' dispersions is
  `indistinguishable`, whichever minimum came out lower.
  ([why](site/src/content/methodology.md#sampling-and-what-may-be-concluded))
- **One campaign per architecture, and the site shows one at a time.** Two architectures are
  never in one chart, one bar group or one table. The architecture is read out of the machine
  record *inside* each campaign, never out of the filename, which is a label somebody typed.
- **The site never calls `JSON.parse` on a campaign.** `checksum` is a 64-bit integer, a
  JavaScript number is a double, and `JSON.parse` silently rounds past 2^53.
  `samples.ndjson` is fetched as *text* and parsed in Rust; checksums cross the wire as
  **strings** and are never added, only displayed and compared.
- **The site is Astro, and every route is prerendered.** GitHub Pages is a file server:
  `output: 'static'`, so a deep link needs no `404.html` fallback. Anything that reads a
  campaign is a React island (`client:only`) — there is no campaign at build time, and a page
  that pretended otherwise would ship a chart of numbers nobody measured. `ClientRouter` swaps
  pages without reloading the document, so the module singleton in `campaigns.ts` keeps the
  WASM instance and the parsed campaigns across a navigation. **It navigates by `pushState`,
  which does not recompute `:target`** — an anchor that has to be marked is marked in code.
- The site's data files **are** the campaigns in `samples/<workload>/<arch>.ndjson`, byte for
  byte. No export format, no intermediate file: the raw samples are the only thing that cannot
  be recomputed, so they are what gets published.

## Never

- Never push benchmark metrics to Prometheus, or any TSDB, and three independent reasons say
  so. **Pushing needs network**, and a network namespace cannot be added mid-run — giving the
  container network for its whole life trades the `--network=none` guarantee for a convention.
  **Prometheus stores `float64`**, and the checksum is a sum of 64-bit integers: past 2⁵³ the
  bit-identical checksum invariant would be silently lost. **A TSDB is lossy by design and
  pull-based**, and a container that lives four seconds is never scraped; keeping thirty
  repetitions would mean encoding the round number in a label, which is a time-series database
  used as a key-value store. Prometheus is for the bench machine's *health* — frequency,
  temperature, throttling — and never for the measurement.
- Never publish an absolute cross-architecture timing. Within-architecture ratios only.
  ([why](site/src/content/methodology.md#flags-and-the-architecture-baseline))
- Never run a benchmark under QEMU / `binfmt` emulation.
- Never let `native` be a **default**, and never let it stand in for the pinned baseline.
  It is asked for explicitly (`--mode native`), it builds its own image, and it gets its own
  row. `--march native` is rejected: that flag is the *baseline* mode's value, and a baseline
  that varies with the CPU that built it is not one.

  This rule used to read *"Never `-march=native`, in any toolchain, under any spelling"*, and
  deleting that line was the point of the ISA axis. It was **unenforceable**: a JIT compiles
  on the machine it runs on, so HotSpot, V8 and PyPy were native the whole time, whatever the
  rule said. The ban never stopped the JVM from getting the machine — it only stopped `gcc`
  from getting it, and then called the result a level playing field. What is forbidden is a
  baseline that quietly varies, never a toolchain using the CPU it was given.
  ([why](site/src/content/methodology.md#the-isa-target))
- **Never `-ffast-math`, in any toolchain, under any spelling.** It is the one that survives,
  and it is not the same rule: `-march=native` decides which *instructions* may be emitted,
  `-ffast-math` decides what *arithmetic means*. Widening a vector reorders nothing, so both
  modes compute the same bits and the checksum gates both. Reassociation computes a different
  number, and a benchmark that publishes a different number under the same heading is not
  measuring speed, it is measuring two programs.
- **Never measure energy.** ([why](site/src/content/methodology.md#what-this-does-not-tell-you))
  Every sample of every campaign came back `null`, on both architectures. A column the bench
  machine can never fill is not a measurement, it is a promise — and the code that reads it is
  code that has never once returned a number. When the machine that publishes changes, that
  section is the argument to re-open; not this code to un-delete.

## Testing

- Unit tests for discovery, manifest parsing, statistics and command construction.
- The kernels themselves are verified by the checksum invariant — which now covers every
  mode, not a third of them — and not by unit tests.
- The site's tests run the **real** WASM over a **real** committed campaign. A hand-written
  fixture would agree with the schema by construction, and agreeing with the schema is the one
  thing those tests must not assume.

## Milestones

1. **Noise floor** on the target machine. Nothing else is trustworthy until this number
   exists. ([why](site/src/content/methodology.md#where-it-runs))
2. The C/gcc, C/clang, Rust/LLVM triangle on Mandelbrot, `baseline`, x86-64.
3. Everything else.
