# langbench

A benchmark harness comparing **compiler and runtime backends**, not languages.

`langbench` discovers benchmark implementations on disk, builds one container per
implementation, runs them under a controlled protocol, and writes raw samples
plus a Markdown report.

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
implementation in all three floating-point modes, and measures with the machine's
thread count.

Common flags — every one of them also reads an environment variable:

| Flag | Env | Default | Purpose |
| --- | --- | --- | --- |
| `--algo` | `ALGO` | all discovered | Restrict to some algorithms |
| `--mode` | `FP_MODE` | `strict,fma,fast` | Floating-point semantics |
| `--cpu` | `CPUS` | machine parallelism | Threads for kernels *and* compilers |
| `--output-dir` | `OUTPUT_DIR` | `results` | Where samples and report land |
| `--benchmarks-dir` | `BENCHMARKS_DIR` | `benchmarks` | Root of the benchmark tree |
| `--grid-size` | `GRID_SIZE` | `4096` | Side of the N×N grid |
| `--max-iter` | `MAX_ITER` | `1000` | Iteration ceiling |
| `--rounds` | `ROUNDS` | `30` | Measured run rounds |
| `--build-rounds` | `BUILD_ROUNDS` | `5` | Measured build rounds |
| `--warmup-rounds` | `WARMUP_ROUNDS` | `2` | Rounds recorded but flagged |
| `--march` | `MARCH` | per-ISA baseline | ISA baseline. `native` is rejected |
| `--log-filter` | `LOG_FILTER` | `info` | [`tracing` filter directive][directives] |

[directives]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives

Example — only the strict mode, on four threads, for one algorithm:

```sh
langbench run --algo mandelbrot --mode strict --cpu 4 --output-dir results/strict-4
```

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

`--output-dir` receives three files:

- **`samples.ndjson`** — the source of truth. A header record with the full
  machine description and campaign parameters, then one line per measured
  invocation, flushed as it is produced. Aggregates are never stored: they are
  recomputed from here.
- **`samples.csv`** — the same records, flat, for a spreadsheet or a dataframe.
  Written in lockstep with the NDJSON. Missing values are **empty fields**, never
  `n/a`: a numeric column that sometimes holds a word breaks every parser that
  reads it. The campaign's context (machine, grid size, `-march`) has no room in
  a flat table and lives in the NDJSON header only, so keep the two together.
- **`report.md`** — a human-facing view, rendered from
  `templates/report.md.liquid`. It leads with any reason this host is a poor
  benchmark target.

```sh
# Median run time per mode, straight from the CSV.
awk -F, 'NR>1 && $6=="run" && $8=="false" { print $5, $10 }' results/samples.csv
```

## Adding an implementation

Convention over configuration. There is no manifest:

```
benchmarks/<algo>/<language>-<compiler>/Dockerfile
```

The image must expose an `ENTRYPOINT` taking either `build <threads>` or
`run <n> <max_iter> <threads>`, and print **exactly one JSON object on stdout**
(everything else goes to stderr):

```json
{"phase":"run","checksum":31415926535,"elapsed_ns":4102337891,"user_usec":32418004,"system_usec":118273}
```

The floating-point mode, the `-march` baseline and the job count arrive as build
args (`FP_MODE`, `MARCH`, `JOBS`), so one Dockerfile covers every mode.

Read [METHODOLOGY.md#container-contract](METHODOLOGY.md#container-contract) for
the full contract, including why the checksum must be an integer and why the
build directory is a tmpfs.

## Development

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check

pre-commit install   # runs the three above, plus hadolint and actionlint
```

## Status

Early. The harness runs end to end, and `mandelbrot/c-gcc` is the first
implementation: it builds in all three floating-point modes, produces three
distinct checksums, and emits no FMA instruction at all under `strict`.

Next: measure the noise floor of the target machine — nothing else is
trustworthy until that number exists — then C/clang and Rust/LLVM, to complete
the first triangle.

## License

MIT
