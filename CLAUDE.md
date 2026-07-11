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

**Layout** ([why](METHODOLOGY.md#repository-layout))

- `benchmarks/<algo>/<language>-<compiler>/Dockerfile`. No YAML manifest, no
  Dockerfile templating.
- Metadata in Docker `LABEL`s (`langbench.language`, `.compiler`, `.version`,
  `.flags`), read back with `docker inspect`. One exception: `.fp_modes` decides
  *which images to build*, so discovery reads it from the Dockerfile source —
  there is no image to inspect yet. It is a constant in the file, never a build
  arg. Absent means every mode; an interpreter declares `strict` alone.
- Base images pinned by digest, never by tag.
- Non-root `USER` in every benchmark Dockerfile.

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

## Rust specifics

- **No `tokio`, no async.** The harness is deliberately sequential — running two
  benchmarks concurrently would destroy the measurement. The global rule mandates
  `tokio` *when async is needed*; here it is not.
- Samples are appended and flushed one at a time, so an interrupted campaign
  keeps every completed sample. That is the graceful-shutdown requirement,
  satisfied by durability rather than by a signal handler.
- The default report template is `templates/report.md.liquid`, embedded with
  `include_str!` so the binary stays self-contained. `langbench md --template`
  overrides it; the built-in one is always the fallback, never a required file.

## Testing

- Unit tests for discovery, label parsing, statistics and command construction.
- The kernels themselves are verified by the strict-mode checksum invariant, not
  by unit tests.

## Milestones

1. **Noise floor** on the target machine. Nothing else is trustworthy until this
   number exists. ([why](METHODOLOGY.md#where-it-runs))
2. The C/gcc, C/clang, Rust/LLVM triangle on Mandelbrot, `strict`, x86-64.
3. Everything else.
