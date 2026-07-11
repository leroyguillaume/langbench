# langbench

A benchmark harness comparing **compiler and runtime backends**, not languages.

`langbench` discovers benchmark implementations on disk, builds one container per
implementation, runs them under a controlled protocol, and writes raw samples —
which it then renders as a CSV or a Markdown report, on demand.

The question it answers is *given the same source, how do different backends
compare?* — gcc versus clang on identical C, rustc-LLVM versus rustc-cranelift on
identical Rust, CPython versus PyPy, OpenJDK versus GraalVM `native-image`.

> **Read [METHODOLOGY.md](METHODOLOGY.md) before trusting any number this
> produces.** It documents what is measured, what is deliberately not measured,
> and why a benchmark that skips those questions produces confident nonsense.

## Requirements

- Rust 1.94+ (edition 2024)
- Docker with a reachable daemon
- **Linux, for any result you intend to publish.** On macOS or Windows the
  containers run inside a VM, and the harness will say so at the top of the
  report.

## Install

```sh
cargo build --release
# the binary lands in target/release/langbench
```

## Inspect the machine first

```sh
langbench machine
```

Prints exactly what a campaign would record in its header, and every reason this
host is a poor benchmark target. Cheap, and worth doing before spending an hour
measuring on a machine that was never going to produce a trustworthy number.

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

## Run

```sh
langbench run
```

By default that discovers everything under `benchmarks/`, builds each
implementation in every floating-point mode its `bench.yaml` declares it
distinguishes (all three unless it says otherwise), and measures with the machine's
thread count. It writes `samples.ndjson` and nothing else; `langbench csv` and
`langbench md` turn that file into a table or a report, whenever you like.

Common flags — every one of them also reads an environment variable:

| Flag | Env | Default | Purpose |
| --- | --- | --- | --- |
| `--algo` | `ALGO` | all discovered | Restrict to some algorithms |
| `--mode` | `FP_MODE` | `strict,fma,fast` | Floating-point semantics |
| `--cpu` | `CPUS` | machine parallelism | Threads for kernels *and* compilers |
| `--output`, `-o` | `SAMPLES_OUTPUT` | `samples.ndjson` | Path of the samples file the campaign writes |
| `--benchmarks-dir` | `BENCHMARKS_DIR` | `benchmarks` | Root of the benchmark tree |
| `--grid-size` | `GRID_SIZE` | `2048` | Side of the N×N grid |
| `--max-iter` | `MAX_ITER` | `1000` | Iteration ceiling |
| `--rounds` | `ROUNDS` | `10` | Measured run rounds |
| `--build-rounds` | `BUILD_ROUNDS` | `3` | Measured build rounds |
| `--warmup-rounds` | `WARMUP_ROUNDS` | `1` | Rounds recorded but flagged |
| `--march` | `MARCH` | per-ISA baseline | ISA baseline. `native` is rejected |
| `--run-timeout` | `RUN_TIMEOUT` | `600` | Seconds before a container is killed |
| `--log-filter` | `LOG_FILTER` | `info` | [`tracing` filter directive][directives] |

[directives]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives

Example — only the strict mode, on four threads, for one algorithm:

```sh
langbench run --algo mandelbrot --mode strict --cpu 4 --output results/strict-4.ndjson
```

### Sizing a campaign

`--grid-size` and `--max-iter` are the same for every implementation, and they
have to be: the strict-mode checksum is a function of both, so a campaign cannot
give each backend its own grid without giving up the correctness gate.

**Size for the slowest backend.** The work scales as `grid_size² × max_iter`, and
what a campaign actually waits on is CPython, not C. At `4096` / `1000` a C run
takes about a second and a CPython run takes forty; multiply by the rounds and
the modes and you get an hour of CPython for a single campaign.

The defaults above are therefore sized for **iteration speed** — a quarter-grid,
ten rounds, one warmup — which puts a full three-mode campaign in the minutes. For
numbers you intend to publish, buy back the resolution:

```sh
langbench run --grid-size 4096 --rounds 30 --warmup-rounds 2 --build-rounds 5
```

Nothing about the smaller default is *wrong*: the estimate is a min-of-N, so more
rounds can only ever lower it, and the dispersion printed beside it tells you
whether N was large enough. A short campaign is pessimistic, never incorrect.

The harness logs one line per invocation so you can watch this happen; if nothing
has moved after `--run-timeout` seconds, the container is killed and the campaign
fails rather than hanging.

Sizing for the slowest backend makes the fastest one's *wall-clock* mostly
container startup — which is why the report also carries `Compute min`, timed
inside the program and unaffected.

## Run in a container

An image is provided, mostly for CI. `langbench` orchestrates containers rather
than running the workload itself, so the benchmark containers are **siblings** on
the host daemon, not children — the image sits outside the measured path.

It therefore needs the host's Docker socket:

```sh
docker build --tag langbench .

docker run --rm \
  --hostname "$(hostname)" \
  --group-add "$(stat -c '%g' /var/run/docker.sock)" \
  --volume /var/run/docker.sock:/var/run/docker.sock \
  --volume "$PWD/benchmarks:/var/lib/langbench/benchmarks:ro" \
  --volume "$PWD/results:/var/lib/langbench/results" \
  langbench run --mode strict
```

Three things to know:

- **Mounting the socket grants root-equivalent access to the host.** Do not do
  this on a machine you do not own.
- **Pass `--hostname`.** Otherwise the harness records the container's ID as the
  machine's hostname, and the campaign is harder to attribute later.
- **`--group-add` is needed because the image runs as a non-root user** (UID
  1000). Never work around this by running as root.

The harness detects that it is containerized and says so in the report, along
with any hypervisor it can find. That detection is a runtime check, not a
compile-time one — inside a Linux container a compile-time check would report
"Linux" on any host, including the macOS laptop it was never meant to trust.

## Output

A campaign writes **one** file, the one `--output` (`SAMPLES_OUTPUT`) names: a
header record with the full machine description and campaign parameters, then one
line per measured invocation, flushed as it is produced. It is the source of
truth, and the only artifact that cannot be recomputed — an interrupted campaign
keeps every sample it completed.

Everything else is a *rendering*, produced afterwards by a separate command. That
is the point: a report can only ever show what a run actually recorded, and the
same file re-renders identically on any host, months later.

```sh
langbench csv       # the samples, flat, into samples.csv
langbench md        # the samples, as a report, into report.md

langbench md results/strict-4.ndjson --output results/strict-4.md
```

Each command reads the samples the campaign wrote and writes a file of its own.
No rendering goes to stdout, so none is lost to a forgotten redirection; stdout
carries only what has no file of its own, and logs go to stderr:

| Command | Flag | Env | Default |
| --- | --- | --- | --- |
| `run` | `--output`, `-o` | `SAMPLES_OUTPUT` | `samples.ndjson` |
| `csv` | `--output`, `-o` | `CSV_OUTPUT` | `samples.csv` |
| `md` | `--output`, `-o` | `MD_OUTPUT` | `report.md` |

The samples path is the positional argument of `csv` and `md`, and it defaults to
the same `SAMPLES_OUTPUT` a `run` wrote — so a campaign and the report that
follows agree without being told twice. Missing parent directories are created.

**CSV.** The same records, flat, for a spreadsheet or a dataframe. Missing values
are **empty fields**, never `n/a`: a numeric column that sometimes holds a word
breaks every parser that reads it. The campaign's context (machine, grid size,
`-march`) has no room in a flat table, which is exactly why the NDJSON stays the
source of truth.

```sh
# Median run time per mode, straight from the samples.
langbench csv
awk -F, 'NR>1 && $6=="run" && $8=="false" { print $5, $10 }' samples.csv
```

**Markdown.** A human-facing view that leads with any reason this host is a poor
benchmark target. It renders `templates/report.md.liquid`, embedded in the binary;
`--template` swaps in your own [Liquid][liquid] template, which receives exactly
the same variables:

```sh
cp templates/report.md.liquid mine.liquid   # the built-in one, as a starting point
langbench md --template mine.liquid         # renders into report.md
```

[liquid]: https://shopify.github.io/liquid/

## Adding an implementation

Drop a `bench.yaml` next to a `Dockerfile`, anywhere under `benchmarks/`:

```yaml
algo: mandelbrot
language: python
compiler: cython      # omit if nothing is compiled ahead of the run
interpreter: cpython  # omit if the binary runs on the bare CPU
modes:
  - strict            # or `modes: all`
description: >-
  The same mandelbrot.py as python-cpython, byte for byte, compiled by Cython to
  a C extension module instead of interpreted.
comments: >-
  It is slower than the interpreter it compiles, and that is a result, not a bug.
```

**The manifest is the only thing the harness reads.** Discovery walks the tree
for `bench.yaml` files — the directory layout is yours to choose, and the
directory *name* means nothing. An implementation is identified by what it is:
`(algo, language, compiler, interpreter)`. Declare the same tuple twice and the
campaign refuses to start.

`compiler` and `interpreter` are each optional, and each absence is a published
fact rather than a gap: gcc compiles and nothing interprets, CPython interprets
and nothing compiles ahead of the run, Cython does both. That last case is why
there is no single "compiler" field — `python-cython` and `python-cpython` share
a language *and an interpreter*, and differ only in the compiler. That is the
clean experiment, and a directory name could not have expressed it.

`modes: all` — the normal case for a compiled backend — or the list of modes the
backend actually distinguishes. An interpreter has one floating-point semantics,
so `fma` and `fast` would be the same image under another tag; declare `strict`
alone and the harness measures that one, warning about each mode it skips. A
misspelled mode fails the campaign rather than quietly building something nobody
asked for.

The image must expose an `ENTRYPOINT` taking either `build <threads>` or
`run <n> <max_iter> <threads>`, and print **exactly one JSON object on stdout**
(everything else goes to stderr):

```json
{"phase":"run","checksum":31415926535,"elapsed_ns":4102337891,"user_usec":32418004,"system_usec":118273}
```

The floating-point mode, the `-march` baseline and the job count arrive as build
args (`FP_MODE`, `MARCH`, `JOBS`), so one Dockerfile covers every mode.

Docker `LABEL`s are still welcome — `langbench.version`, `langbench.flags` — but
they are provenance for `docker inspect`, and the harness never reads them. Two
sources of truth is one source of drift.

Read [METHODOLOGY.md#container-contract](METHODOLOGY.md#container-contract) for
the full contract, including why the checksum must be an integer and why the
build directory is a tmpfs.

### Checking a manifest before you spend an hour on it

```sh
langbench validate            # every failure a campaign would hit at discovery
```

It parses every `bench.yaml` on disk and reports **all** the problems at once —
a misspelled key, an unknown FP mode, a backend that neither compiles nor
interprets, two manifests claiming the same identity — without building a single
image. Point it at a file or a directory; it defaults to the whole tree.

`bench.schema.json`, at the repo root, is the manifest's JSON Schema. It is
generated from the Rust struct the harness actually deserializes, never written
by hand:

```sh
langbench jsonschema          # rewrites bench.schema.json
```

A pre-commit hook regenerates it and fails if the checked-in copy has drifted, so
the schema your editor completes from cannot disagree with what the campaign
accepts. To get that completion, point your editor at it — for VS Code's YAML
extension:

```json
{"yaml.schemas": {"./bench.schema.json": "**/bench.yaml"}}
```

## Development

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check

pre-commit install   # the three above, plus hadolint, actionlint, and the two
                     # manifest hooks: `validate` runs whenever a bench.yaml
                     # moves, and bench.schema.json is checked for drift
```

## Status

Early, but the central claims are demonstrated. Three implementations exist —
`c-gcc`, `python-cpython` and `python-cython` — and **all three agree on the
strict-mode checksum bit for bit**. The gate is not decorative: reassociating one
expression in the Python kernel aborts the campaign.

The two Python rows compile a byte-identical source (a test enforces it), so they
are the clean "same source, different backend" experiment. The first result is
counter-intuitive and worth the whole harness: **Cython is 1.3× slower than the
CPython interpreter it compiles.** Without type annotations the generated C
manipulates `PyFloat` objects through the C-API — its hot loop holds 142 call
instructions and one `fadd`, where the C kernel has none and fourteen — while
CPython 3.13's specializing interpreter takes a fast path for `float * float`.

Next: measure the noise floor of the target machine — nothing else is
trustworthy until that number exists — then C/clang and Rust/LLVM, to complete
the first triangle of compiled backends.

## License

MIT
