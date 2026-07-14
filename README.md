# langbench

A benchmark harness comparing **compiler and runtime backends**, not languages.

`langbench` reads the workloads declared on disk, builds one container per
implementation, runs them under a controlled protocol, and writes raw samples — one
line per measured invocation, and nothing else.

The question it answers is *given the same source, how do different backends compare?*
— gcc versus clang on identical C, CPython versus PyPy versus Cython on identical
Python, HotSpot versus OpenJ9 versus an ahead-of-time GraalVM binary on identical
bytecode.

**This file is about the binary**: how to run it, how to add work for it to measure,
and what an image has to do to be measurable. The numbers themselves, and the
arguments about what may be concluded from them, live on the website:

- **[The results](https://leroyguillaume.github.io/langbench/)** — one page per
  campaign, recomputed in the browser from the samples this repository publishes.
- **[The methodology](site/src/content/methodology.md)** — what is measured, what is
  deliberately not, and why a benchmark that skips those questions produces confident
  nonsense. **Read it before trusting a number.** It is rendered
  [on the site](https://leroyguillaume.github.io/langbench/methodology/) too.

The campaigns themselves are in **[`samples/`](samples/)**, as
`samples/<workload>/<architecture>.ndjson`: the only artefact a run produces that
cannot be recomputed, and the source of every chart, table and ratio anywhere else.
They are never merged, because **an absolute timing does not cross an architecture**
([why](site/src/content/methodology.md#flags-and-the-architecture-baseline)).

## Requirements

- Rust 1.94+ (edition 2024)
- Docker, with a reachable daemon
- **Linux, for any result you intend to publish.** On macOS or Windows the containers
  run inside a VM, and the harness records that fact in the campaign header — the
  website then leads with it.

## Install

```sh
cargo build --release
# the binary lands in target/release/langbench
```

## Inspect the machine first

```sh
langbench machine
```

Prints exactly what a campaign would record in its header, and every reason this host
is a poor benchmark target. It costs a second, and it is worth doing before spending an
hour measuring on a machine that was never going to produce a trustworthy number.

```
PROPERTY                VALUE
---------------------   ----------------------------------------
Hostname                bench-01
Kernel                  6.8.0-45-generic
Virtualization          none detected
Scaling governor        performance
Turbo disabled          true
...

No warning: this host looks like a usable target.
```

## Run a campaign

A campaign is **one machine measuring one workload**, so `run` takes the workload:

```sh
langbench workload list             # what is there to measure?
langbench workload run mandelbrot
```

That builds every implementation the workload declares, in every floating-point mode
each one says it distinguishes, sized by the params the `workload.yaml` declares, and
measured with the machine's thread count. It writes `samples.ndjson` and nothing else.

Every flag also reads an environment variable:

| Flag | Env | Default | Purpose |
| --- | --- | --- | --- |
| *(positional)* | `WORKLOAD` | — | The workload to measure. Required, and exactly one |
| `--param` | — | as declared | Override a workload param: `--param grid_size=256`. Repeatable |
| `--mode` | `MODE` | `baseline,native` | The ISA target: a pinned baseline, the machine itself, or both |
| `--cpu` | `CPUS` | machine parallelism | Threads for the kernels *and* the compilers |
| `--output`, `-o` | `SAMPLES_OUTPUT` | `samples.ndjson` | The samples file the campaign writes |
| `--benchmarks-dir` | `BENCHMARKS_DIR` | `benchmarks` | Root of the benchmark tree |
| `--rounds` | `ROUNDS` | `10` | Measured run rounds |
| `--build-rounds` | `BUILD_ROUNDS` | `3` | Measured build rounds |
| `--warmup-rounds` | `WARMUP_ROUNDS` | `1` | Rounds recorded but flagged |
| `--march` | `MARCH` | per-architecture baseline | What `baseline` *means*. `native` is rejected here — it is a mode, not a baseline |
| `--memory-limit-mb` | `MEMORY_LIMIT_MB` | `8192` | Memory budget of every measured container |
| `--run-timeout` | `RUN_TIMEOUT` | `600` | Seconds before a container is killed |
| `--log-filter` | `LOG_FILTER` | `info` | [`tracing` filter directive][directives] |

[directives]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives

```sh
# The pinned baseline only, on four threads, into a file of its own.
langbench workload run mandelbrot --mode baseline --cpu 4 --output results/baseline-4.ndjson

# A quarter-size grid, to find out whether the Dockerfile has a typo before
# committing an hour to it. Overriding a param drops the declared checksum,
# because it is the answer to the declared work and not to this one — and the
# campaign says so.
langbench workload run mandelbrot --param grid_size=512 --rounds 3
```

Three behaviours are worth knowing before you start one:

- **The memory budget is part of the measurement, not a safety rail.** A
  garbage-collected runtime sizes its heap from what its cgroup shows it, so
  `--memory-limit-mb` is pinned identically for every container — and changing it
  changes the *timings* too. Campaigns run under different budgets do not compare, on
  any column
  ([why](site/src/content/methodology.md#how-a-run-is-measured)).
- **A backend that fails is quarantined, not propagated.** A build that fails, a
  container that crashes or hangs, a checksum that disagrees with the reference: each
  takes out that one `(implementation, mode)` unit, at the point it breaks. The
  campaign keeps measuring the rest, exits 0, and **publishes the failure** as a record
  beside the samples. Only a campaign where *every* unit failed exits non-zero
  ([why](site/src/content/methodology.md#sampling-and-what-may-be-concluded)).
- **Ctrl-C stops it cleanly.** The container in flight is killed — it runs on the
  daemon, in another process tree, and leaving it holding every core would bias
  whatever gets measured next — and the interrupted run is *not* written down. The
  samples already on disk are intact, the file still renders, and the harness exits 0.

## Run it in a container

An image is provided, mostly for CI. Running the harness in Docker means it has to
start Docker containers of its own, and there are two ways to do that. This image uses
the one that does not corrupt the measurement.

### Sibling containers, not nested ones

`langbench` orchestrates containers; it never runs the workload itself. So the image
ships a Docker **client** and talks to the **host's** daemon through its socket. The
benchmark containers it starts are therefore **siblings** of the harness container —
both are children of the host daemon — and the harness sits entirely outside the
measured path.

```
            host daemon
            ├── langbench          (the client — orchestrates, measures nothing)
            ├── mandelbrot-c-gcc   (measured)
            └── mandelbrot-cpython (measured)
```

The alternative — true Docker-in-Docker, `--privileged` with a second daemon nested
inside the harness container — would put the benchmark containers **inside** langbench.
**Do not do that here.** Every measured run would execute under a nested storage driver
and a second layer of cgroup and namespace indirection, and the CPU time the harness
reads from `cpu.stat` would come from a cgroup nested inside another container's
cgroup. That is a benchmark of your container runtime's overhead, wearing the costume
of a benchmark of gcc.

### Doing it

The benchmark tree ships **inside** the image, so a campaign needs only the host's
socket and somewhere to put the samples. Linux and macOS differ in exactly one flag —
`--group-add`, the group that owns the socket — and getting it wrong fails with
`permission denied while trying to connect to the docker API`.

```sh
docker build --tag langbench .
mkdir -p results
```

**Linux.** The socket is owned by the host's `docker` group, so hand the container that
gid. Look it up rather than hardcoding `999`; it varies by distribution:

```sh
docker run --rm \
  --hostname "$(hostname)" \
  --group-add "$(stat -c '%g' /var/run/docker.sock)" \
  --volume /var/run/docker.sock:/var/run/docker.sock \
  --volume "$PWD/results:/var/lib/langbench" \
  langbench workload run mandelbrot --mode baseline
```

**macOS** (Docker Desktop, OrbStack, Colima). The daemon lives in a Linux VM, and the
socket a container actually sees there is `root:root`, mode `660`, whatever the Mac says
about the file. So the gid is `0`:

```sh
docker run --rm \
  --hostname "$(hostname)" \
  --group-add 0 \
  --volume /var/run/docker.sock:/var/run/docker.sock \
  --volume "$PWD/results:/var/lib/langbench" \
  langbench workload run mandelbrot --mode baseline
```

Do not port the Linux line across by swapping `stat -c` for BSD's `stat -f '%g'`: it
*runs*, and it reports gid `1`, which grants nothing. Treat a macOS run as a smoke test
regardless — the whole workload sits in a hypervisor, and the campaign header says so.

`/var/lib/langbench` is the working directory, and everything a campaign writes lands
directly in it. That single mount is what carries the samples back out.

**Your own tree.** The bundled benchmarks are a default, not a constraint. Mount yours
over the image's, read-only — the harness never writes to it:

```sh
  --volume "$PWD/benchmarks:/usr/local/share/langbench/benchmarks:ro" \
```

`/usr/local/share/langbench/benchmarks` is where the image keeps them, and
`BENCHMARKS_DIR` points there. It sits outside `/var/lib/langbench` deliberately:
inputs are read-only data, outputs are the mount, and a benchmark tree nested under the
output directory would vanish the moment you mounted one over it.

Four things to know:

- **Mounting the socket grants root-equivalent access to the host.** Anything that can
  talk to that socket can start a privileged container and own the machine. Do not do
  this on a host you do not own.
- **Mount `/var/lib/langbench` itself.** With `--rm` and nothing mounted there, the
  samples die with the container — and the samples are the one artefact that cannot be
  recomputed.
- **Pass `--hostname`.** Otherwise the harness records the container's ID as the
  machine's hostname, and the campaign is harder to attribute later.
- **`--group-add` is needed because the image runs as a non-root user** (UID 1000),
  which is not in the host's `docker` group. Never work around this by running the
  harness as root.

## What a campaign writes

**One file**, the one `--output` names: a header record carrying the full machine
description, the campaign's parameters and a snapshot of the workload manifest, then
one line per measured invocation, flushed as it is produced. An interrupted campaign
keeps every sample it completed.

It is the source of truth and the only artefact that cannot be recomputed. Everything
else — every table, every chart, every ratio — is a *rendering*, derived from it
afterwards. The **website** is the human one, and it does the arithmetic with the
harness's own code compiled to WebAssembly. The machine-readable one is:

```sh
langbench sample convert                              # samples.ndjson -> samples.csv
langbench sample convert results/baseline-4.ndjson -o results/baseline-4.csv
langbench sample convert --format csv                 # csv is the only format today
```

It **aggregates nothing**: one row per sample, the columns the samples carry. Missing
values are empty fields, never `n/a` — a numeric column that sometimes holds a word
breaks every parser that reads it. The campaign's context (the machine, the params, the
`-march`) has no room in a flat table, which is exactly why the NDJSON stays the source
of truth.

`description` and `comments` are prose, so they are quoted and they contain commas: read
the file with a CSV parser, never with `cut -d,` or `awk -F,`.

```python
# Median run time per mode, straight from the samples.
import csv, statistics
rows = [r for r in csv.DictReader(open("samples.csv"))
        if r["phase"] == "run" and r["warmup"] == "false"]
for mode in {r["mode"] for r in rows}:
    ns = [int(r["wall_ns"]) for r in rows if r["mode"] == mode]
    print(mode, statistics.median(ns) / 1e6, "ms")
```

## Adding work

Two manifests, and they say different things. A **workload** declares the work: what it
is, how it is sized, what the right answer is, and which directories implement it.

```yaml
# benchmarks/mandelbrot/workload.yaml
id: mandelbrot
description: >-
  Escape-time Mandelbrot over a square grid, summing the iteration counts.
  What it puts under the light, and what it says nothing about.
params:                 # the order is the order the kernels receive them:
  - name: grid_size     #   run <grid_size> <max_iter> <threads>
    value: 2048
  - name: max_iter
    value: 1000
checksum: 1038538536    # optional, but declare it: it is the correctness gate
implementations:        # declared, never walked for
  - c-gcc
  - python-cython
```

An **implementation** declares a backend that does that work, in a `bench.yaml` beside
its `Dockerfile`:

```yaml
# benchmarks/mandelbrot/python-cython/bench.yaml
language: python
compiler: cython      # omit if nothing is compiled ahead of the run
interpreter: cpython  # omit if the binary runs on the bare CPU
source: mandelbrot.py # the one kernel file, beside this manifest
modes:
  - baseline          # or `modes: all`, or `[native]` for a JIT
architectures: all    # the default; omit unless the toolchain does not exist somewhere
description: >-
  The same mandelbrot.py as python-cpython, byte for byte, compiled by Cython to
  a C extension module instead of interpreted.
comments: >-
  Unannotated, the generated C manipulates PyFloat objects through the C-API
  rather than raw doubles.
```

Then add the directory to the workload's `implementations`. **Nothing is measured
because of where it sits**: the harness walks the tree for `workload.yaml` files — the
one search it does — and after that it reads declarations. A `bench.yaml` that no
workload lists is caught by `langbench validate`; it would otherwise be a backend that
is never built, never measured, and never missed.

A few rules the manifests enforce, each of which has cost somebody an afternoon:

- **An implementation is `(workload, language, compiler, interpreter)`.** There is no
  name. `compiler` and `interpreter` are each optional — but not both — and an absence
  is a published fact rather than a gap: gcc compiles and nothing interprets, CPython
  interprets and nothing compiles ahead of the run, Cython does both. That last case is
  why there is no single "compiler" field.
- **`source` is declared, never guessed.** The only way to guess is to pattern-match a
  filename, and the path is not metadata.
- **`comments` describes the implementation, not the results.** What is pinned, what the
  entrypoint has to do, how this backend deviates. Never what to expect from the table:
  that changes every time a campaign runs, and the campaign is what says it.
- **`architectures` is `all`** unless the backend's toolchain *does not exist* for an
  architecture. Kotlin/Native publishes no `linux-aarch64` host compiler, so
  `architectures: [x86_64]` is simply the truth about it; a campaign on the other
  machine skips the row and says why, rather than failing halfway through a `docker
  build`.
- **`modes: all`**, or the list of modes the backend actually distinguishes. The modes are
  ISA targets — `baseline` (a pinned instruction set) and `native` (this CPU) — and *which
  ones a backend can offer is the point*. An ahead-of-time compiler must choose a machine
  to emit code for, so it has both. A JIT compiles on the machine it is running on and
  cannot do otherwise, so it declares `[native]` alone: a `baseline` image would be the same
  run under another tag. That is not a hole in the table, it is what a JIT sells. A mode
  requested but not declared is skipped with a warning; a *misspelled* one fails the
  campaign.

Check it before you spend an hour on it:

```sh
langbench validate            # every failure a campaign would hit at discovery
```

It parses every manifest on disk and reports **all** the problems at once — a misspelled
key, an unknown FP mode, a backend that neither compiles nor interprets, two manifests
claiming the same identity, a `bench.yaml` no workload lists — without building a single
image. It needs the whole tree: two backends collide with *each other*, and an
undeclared manifest is only visible to someone holding both the tree and every
workload's list of what it claims.

`bench.schema.json` and `workload.schema.json`, at the repo root, are the two manifests'
JSON Schemas. Both are **generated** from the Rust structs the harness actually
deserializes, never written by hand, and a pre-commit hook fails on drift:

```sh
langbench implementation jsonschema   # rewrites bench.schema.json
langbench workload jsonschema         # rewrites workload.schema.json
```

Point your editor at them and get completion as you type a manifest — for VS Code's
YAML extension:

```json
{"yaml.schemas": {
  "./bench.schema.json": "**/bench.yaml",
  "./workload.schema.json": "**/workload.yaml"
}}
```

## The container contract

This is what makes an image measurable, and it is the same for every backend.

Every image exposes one `ENTRYPOINT`, taking one of two subcommands:

- **`build <threads>`** — recompile from a clean state, and discard the artefacts. It
  exists only to be timed.
- **`run <params…> <threads>`** — execute the binary the image already ships. The params
  arrive in the order the `workload.yaml` declares them.

Each invocation prints **exactly one JSON object on stdout**, and nothing else.
Compilers and runtimes write to stderr; stdout is reserved for the record, and the
harness rejects any other shape rather than measure noise.

```json
{"phase":"run","checksum":31415926535,"elapsed_ns":4102337891,"user_usec":32418004,"system_usec":118273,"peak_bytes":13160448}
{"phase":"build","elapsed_ns":812004221,"user_usec":2914000,"system_usec":204000,"binary_bytes":312840,"binary_stripped_bytes":248904,"text_bytes":41216,"peak_bytes":486539264}
```

Stdout rather than a bind-mounted file: it needs no volume, no per-invocation temporary
directory, and no reasoning about append ordering. Printing the checksum also happens to
be what stops dead-code elimination from deleting the hot loop.

Five things the entrypoint is responsible for:

- **The checksum is a JSON integer**, 64 bits wide. It is the correctness gate of the
  whole project, and anything that rounds it — a `float64`, a metrics system, a
  spreadsheet — destroys the invariant.
- **`user_usec`, `system_usec` and `peak_bytes` come from the container's own cgroup** —
  `/sys/fs/cgroup/cpu.stat` and `memory.peak`, read from inside, before the entrypoint
  returns. Never from `getrusage`: the workload runs under `containerd-shim`, in another
  process tree, and the harness's own rusage would measure argument parsing
  ([why](site/src/content/methodology.md#how-a-run-is-measured)).
  `peak_bytes` is `null` on a kernel that exposes neither file — `null`, never `0`.
- **`elapsed_ns` is the program's own clock**, around the compute alone. The harness adds
  the external wall-clock, which is the one number nothing inside the container can
  produce: nothing in there is alive to time its own creation. The gap between the two is
  the runtime's startup, and it is a result.
- **The thread count arrives on the command line and is used as given.** A kernel must
  never call `available_parallelism()`, `os.cpu_count()` or `runtime.NumCPU()` —
  runtimes disagree about cgroup quotas, and auto-detection would measure that
  disagreement instead of parallel speed.
- **The build directory is a `--tmpfs`, and the run has `--network=none`.** The harness
  imposes both. A build that tries to fetch fails loudly instead of silently adding four
  seconds to a number.

The ISA target and the job count arrive as **build args** (`MARCH`, `JOBS`), so one
Dockerfile covers both modes: `MARCH` is either the architecture's pinned baseline or the
literal `native`, and that choice *is* the mode. There is no floating-point build arg —
the arithmetic is strict in every mode, so there was nothing left to parameterize. Base
images are pinned
by digest, never by tag: a benchmark that silently changes when upstream pushes is not a
benchmark. Docker `LABEL`s are welcome as provenance for `docker inspect`, and the
harness never reads them — two sources of truth is one source of drift.

## The website

[`site/`](site/) is an [Astro][astro] site, prerendered to static files, that reads the
campaigns in `samples/` and recomputes every number from them **in the browser, with the
harness's own code** — `src/analysis.rs` and `src/compare.rs`, compiled to WebAssembly.
It measures nothing, and it re-implements nothing: a second definition of min-of-N in
TypeScript would be a second definition of what this project measures.

```sh
cd site
npm install
npm run dev      # builds the WASM, publishes the campaigns and the manifests, serves
npm run build    # what CI deploys to GitHub Pages
```

It needs a Rust toolchain: the WASM comes from this crate, and the workloads it
describes come from `langbench workload list --json` — the harness is the only thing in
this repository that reads a manifest.

```sh
SAMPLES_DIR=samples.local npm run dev   # look at a campaign you ran and are not committing
```

[astro]: https://astro.build/

## Development

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check

pre-commit install   # fmt and clippy, plus hadolint, actionlint, biome, tsc, and the
                     # two manifest hooks: `validate` runs whenever a bench.yaml moves,
                     # and the schemas are checked for drift
```

**The suites are not hooks.** `cargo test` and `vitest` are the slowest thing either
language has to offer, and a hook meant to be instant is the wrong place to pay for them
— the [`test`](.github/workflows/test.yaml) workflow runs them on every push.

The campaigns this repository publishes are produced by the
[`bench`](.github/workflows/bench.yaml) workflow, on `workflow_dispatch` only: a
campaign is a deliberate act, not a side effect of a push. A GitHub runner is shared,
virtualised and frequency-scaled — which is to say the worst benchmark target money can
rent — so its numbers are *indicative*, and they say so themselves. For a number worth
publishing, run a campaign on a real machine and commit its
`samples/<workload>/<architecture>.ndjson`; the site is a pure function of that file and
will render it unchanged.

## License

MIT.
