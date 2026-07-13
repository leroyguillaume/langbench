# Methodology

This document explains what `langbench` measures, what it refuses to measure,
and why. A benchmark whose methodology is not published is worth nothing, so
this file is a deliverable, not a footnote.

If you are here to dispute a number, this is the right page. Start with
[Claims we do not make](#claims-we-do-not-make).

---

## What is under test

**Compiler and runtime backends, not languages.**

The primary question is: *given the same source, how do different backends
compare?* gcc versus clang on identical C. rustc-LLVM versus rustc-cranelift on
identical Rust. CPython versus PyPy. OpenJDK versus GraalVM `native-image`.

The unit of comparison is therefore not a language but a tuple:

> (compiler, version, flags, target architecture)

Cross-language comparison is a secondary, much weaker result. See
[Two axes](#two-axes-two-tables-never-merged).

---

## The benchmark: Mandelbrot

For each pixel of an `N × N` grid mapped onto the complex plane, iterate
`z ← z² + c` until `|z| > 2` or `max_iter` is reached. The program prints the
sum of all iteration counts — the **checksum** — and nothing else. No image is
written: zero I/O in the measured path.

Deliberate properties:

- Embarrassingly parallel. No shared state, no locks.
- No allocation in the hot loop, no data structures. This measures codegen, not
  the quality of a standard library.
- The load is **imbalanced**: interior pixels run to `max_iter`, exterior pixels
  exit after a few iterations. Chunking must therefore be dynamic — at least
  `4 × threads` chunks handed out on demand. A static contiguous split measures
  the split, not the backend.
- **No third-party dependencies. None.** Rust uses `std::thread` and an
  `AtomicUsize` chunk counter, not `rayon`. Otherwise the timed build compiles
  eight thousand lines of rayon while gcc compiles fifty lines of C, and the
  build-time column means nothing. It also removes every question about
  pre-building dependencies. Each implementation is a single source file.

### What this benchmark actually measures

The hot loop is a **divergent-exit floating-point loop**. Vectorizing it requires
masked SIMD: process eight pixels at once, retire the lanes that have escaped.
Some autovectorizers manage it, some give up.

So between two mature optimizing backends, this benchmark largely measures
*whether the autovectorizer handles that specific loop shape*. The result is a
step function — vectorized (≈4×) or not (1×) — not a continuous quality metric.

That is a legitimate thing to measure. It is **not** "which compiler is better".
Broader claims would require a suite: a scalar dependency chain (n-body), a
pointer-chasing kernel (alias analysis), a branchy kernel. We start with
Mandelbrot alone, and we say so.

### Anti-cheating contract

- `N`, `max_iter` and the thread count come from `argv`. Never compile-time
  constants — a backend may otherwise constant-fold the entire computation away.
- The checksum is printed. Never discard it, or dead-code elimination deletes the
  loop and the benchmark measures nothing at infinite speed.
- The thread count is an **explicit argument**. Implementations must never call
  `available_parallelism`, `os.cpu_count()`, `runtime.NumCPU()` or equivalent.
  Those functions disagree about cgroup quotas across runtimes: Rust reads the
  cgroup v2 quota, CPython does not, Go only learned to in 1.25. Auto-detection
  would measure "does this runtime read `/sys/fs/cgroup`", not parallel speed.

  (The *harness* auto-detects a default for `--cpu`. That is correct: it then
  passes the value explicitly. The prohibition applies to the kernels.)

---

## Floating-point modes

Three modes, built from the same source via build args. The axis is **FP
semantics**, not "optimization on or off" — every mode is `-O3`.

| Mode     | Flags                             | Meaning |
| -------- | --------------------------------- | ------- |
| `strict` | `-ffp-contract=off`, no fast-math | Bit-reproducible IEEE 754 |
| `fma`    | FMA contraction allowed           | Bit-different but *more* accurate: one rounding instead of two |
| `fast`   | `-ffast-math`                     | Reassociation allowed: precision sold for speed |

Splitting `fma` out of `fast` matters. FMA contraction changes the result in the
last bit, but in the *right direction* — it is not cheating, it is a different
definition of the computation. Reassociation is the real relaxation. Lumping them
together throws away the most interesting column: the cost of demanding
bit-reproducibility.

### The strict-mode invariant

`strict` mode is the **correctness gate for the entire harness**.

Mandelbrot uses only multiply, add, subtract and compare. All four are correctly
rounded under IEEE 754 — the result is specified to the bit, and both x86-64 (on
SSE2, not the old 80-bit x87) and AArch64 conform. With no FMA contraction, no
reassociation and no denormal flushing, the checksum **must be bit-identical
across every compiler, every language and both architectures.**

One reference value. Seventeen implementations across ten languages — C, C++,
Rust, Zig, Go, Julia, Python, JavaScript, TypeScript — and **every one of them
agrees on it, bit for bit**, from `gcc -O3` to a JIT-compiled Julia script to
JavaScript in a Bun worker. Any divergence is a bug — in the code, in the flags,
or in our understanding of them. Never a rounding excuse.

This invariant catches the class of error that unit tests do not. Measured: C/gcc
and CPython agree exactly, and rewriting `zr2 - zi2 + cr` as `cr + zr2 - zi2` in
the Python kernel — a reassociation that looks like a harmless tidy-up — flips
two pixels out of twelve million and takes the backend out of the campaign.

#### The languages that fuse behind your back

Two backends had to be written against this invariant rather than merely checked
by it, and both are worth knowing about before adding a third.

**Go fuses, and its spec says it may.** The specification permits an
implementation to "combine multiple floating-point operations into a single fused
operation, possibly across statements". On AArch64 the `gc` compiler takes the
offer: written the obvious way, the Go kernel emits five `FMADD`/`FMSUB`
instructions and returns **33209560** where every other language returns
**33209574**. It is not slower, and it is not buggy — it is computing something
else, and nothing but the checksum would have said so. The same clause gives the
only way out: "an explicit floating-point type conversion rounds to the precision
of the target type". Every `float64(...)` in `mandelbrot.go` is therefore a
*rounding point*, not a cast, and they are load-bearing. Note that guarding the
obvious `2.0*zr*zi + ci` is not enough: the compiler also fuses `zr2 - zi2` back
into an `FMSUB` on `zr*zr`, because it can still see where `zr2` came from. Every
product that feeds an add or a subtract needs its own rounding point.

This is the difference between C and Go on this axis. C says "do not contract" on
the command line (`-ffp-contract=off`), where it is visible and where a build arg
can flip it. Go says it in the source — so a fused Go build is a *different
program*, not a different flag, and `fma` cannot be a mode over one kernel the
way it is for C. Hence `modes: [strict]`.

**Zig relaxes in the source too**, for the same structural reason:
`@setFloatMode(.optimized)` is a statement inside the program, not a compiler
flag. Same conclusion, same one-mode manifest.

**It is a necessary condition, not a sufficient one.** The gate sees a change only
when it flips a pixel's iteration count, so a perturbation that lands nowhere near
a boundary is invisible: shifting `X_MIN` by one ULP changes nothing at 200×200
with `max_iter=100`. Sensitivity grows with grid size and iteration ceiling, since
both increase the number of pixels sitting on a boundary. A passing checksum means
"no evidence of divergence at this resolution", not "provably identical".

For `fma` and `fast` we do not gate on the checksum. We report its **delta from
the strict reference** in a column beside the timing, so the reader sees the
speed gained and the precision sold in one glance.

---

## Compiler flags

- **Never `-march=native`.** The CPU model varies between runs; the architecture baseline
  would vary with it. Pin an explicit baseline per architecture (e.g. `x86-64-v3`) as a
  build arg and record it in the results.
- `x86-64-v3` and any AArch64 baseline are **not equivalent** and we never claim
  they are. NEON is 128-bit wide — two `f64` lanes. AVX2 is 256-bit — four. A
  factor of two on vectorized Mandelbrot comes straight out of the architecture and has
  nothing to do with the compiler.
- Pin and document everything that trades compile time against runtime speed:
  Rust's `codegen-units`, `strip`, the linker (`ld` / `lld` / `mold`). Otherwise
  we benchmark a default rather than a decision.

### Every toolchain spells the baseline differently, and some ignore it silently

The harness speaks gcc: it hands every backend `MARCH=x86-64-v3` or
`MARCH=armv8.2-a`. Only the C and C++ compilers take that verbatim. Each of the
others translates it in its entrypoint — `-C target-feature=+v8.2a` for rustc,
`-mcpu=generic+v8_2a` for zig, `GOARM64=v8.2` for go, `--cpu-target` for julia —
and **an unrecognised baseline must fail the build, loudly**.

That rule is not defensive pedantry. Measured:

- **rustc only warns.** `-C target-cpu=armv8.2-a` prints *"not a recognized
  processor (ignoring processor)"* and hands back a generic binary — and it says
  exactly the same thing about `-C target-cpu=nonsense-v9`. A campaign would run
  to completion and publish a row claiming an architecture baseline it was never compiled
  for. The Rust entrypoint therefore both translates the name *and* greps rustc's
  stderr for that warning, failing if it appears.
- **Go silently no-ops.** `GOAMD64=v3` on an arm64 build is not an error; it is
  ignored.
- **Julia defaults to `native`** — the one thing this project forbids outright —
  so the baseline must always be passed explicitly. To its credit it is one of the
  few toolchains here that *rejects* a name it does not know.
- **OpenJ9 ignores unknown `-XX:` options entirely.** Measured:
  `java -XX:CompleteNonsenseFlag=42 -version` starts happily, where HotSpot refuses
  to boot on the same flag. So the vector caps the HotSpot rows use would have
  pinned *nothing* there while the manifest claimed otherwise. That backend
  therefore passes no architecture flag at all and publishes the gap — an honest hole beats
  a false guarantee.

A build that quietly falls back to generic does not break the campaign. It
publishes a wrong number with a straight face, which is worse.

#### The JVM cannot honour this rule, and says so

HotSpot has no `-march`. C2 compiles for **whatever CPU it finds at run time** —
which is exactly the `native` targeting forbidden everywhere else in this
document, and the JVM rows get it whether we like it or not. There is no flag that
pins an architecture baseline the way `-march=x86-64-v3` does for gcc.

What the JVM does offer is a *cap on vector width*: `-XX:UseAVX=2` on x86-64,
`-XX:UseSVE=0` on AArch64. The Java, Kotlin and Scala entrypoints pin those, which
stops the JIT from reaching for wider vectors than the compiled rows were allowed.
It is an approximation and it is published as one, in each manifest's `comments`.
Read a JVM row against the C rows with that caveat in hand: the architecture floor is
pinned, the ceiling is not.

The alternative — dropping the JVM from the table — would be a worse answer to an
honest limitation.

**Except for one row.** GraalVM `native-image` compiles *ahead* of the run, so it
takes a real `-march` and is the only JVM backend with a genuine architecture baseline. It
comes with its own wrinkle: on AArch64 native-image offers `armv8-a` and
`armv8.1-a` and stops, with no `armv8.2-a` to match the campaign's. The rule there
is **never above the campaign's baseline** — it takes the highest level it can
express that does not exceed what every other backend was held to, which is one
below. The row is handicapped rather than flattered, and that is the safe direction
to be wrong in.

---

## Two axes, two tables, never merged

1. **Same source, different backend.** The real experiment. gcc versus clang on
   identical C; rustc-LLVM versus rustc-cranelift on identical Rust. Clean, and
   the reason this project exists.
2. **Same workload, different language.** Confounded by construction: different
   source, different runtime, different standard library. Valid for orders of
   magnitude ("Python is roughly 80× slower than Rust"), never for percentages.

---

## The architecture rule

**Absolute cross-architecture timings are never published.**

Changing architecture means changing silicon. Frequency, microarchitecture, cache
hierarchy and memory bandwidth all move together with the treatment. The
confounding variable is perfectly collinear with the one under study; no amount
of statistics recovers the effect. If a Graviton beats a Xeon here, we cannot
tell whether clang's AArch64 backend is better or whether it is simply a better
chip.

What survives is the **within-architecture ratio**:

> clang beats gcc by 12% on x86-64, but by only 3% on aarch64.

That is a statement about backend maturity per target, and it is the interesting
one.

A pleasant consequence: the "same machine" requirement applies *per architecture*. One CI
job on x86-64 and one on aarch64, on different physical machines, is fine —
because only intra-job ratios are ever used.

**Never run a benchmark under QEMU or `binfmt` emulation.** Native builds or
nothing.

---

### A toolchain that does not exist is not a slow toolchain

Some backends cannot be built on some architectures at all. Kotlin/Native ships
host compilers for `linux-x86_64`, macOS and Windows — and none for
`linux-aarch64`. There is no flag that fixes this.

The two ways around it are both forbidden here. **Emulation** (QEMU / `binfmt`) is
banned outright: a benchmark run under emulation measures the emulator.
**Cross-building** would let the image build, but the Build column would then
report a compile that happened on another architecture than the run, which is a
number about nothing.

So the manifest declares it — `arch: [x86_64]`, defaulting to `all` — and a
campaign on the other machine **skips the row loudly at discovery**, before
spending a `docker build` on finding out. The row is absent from that campaign's
table, and the log says exactly why. A missing row with a reason is a result; a
missing row without one is a bug.

---

## Repository layout

Every implementation declares itself in a `bench.yaml` beside its Dockerfile:

```yaml
workload: mandelbrot
language: python
compiler: cython
interpreter: cpython
modes:
  - strict
description: >-
  The same mandelbrot.py as python-cpython, byte for byte, compiled by Cython to
  a C extension module instead of interpreted.
comments: >-
  It is slower than the interpreter it compiles, and that is a result, not a bug.
```

**The manifest is the only thing the harness reads.** Discovery is a walk for
`bench.yaml` files: no manifest, no benchmark. Everything else about a directory
is inert.

### The path is not metadata

An earlier design inferred the language and the compiler from the directory name
(`benchmarks/<workload>/<language>-<compiler>/`) and read the rest back out of Docker
labels. Both are gone, for the same reason: they encode facts in places that
cannot hold them.

A path is a two-field record with no room for a third. `python-cython` is a
directory name that *cannot say* that CPython also runs the result — and that
omission is not cosmetic, because the whole value of that row is that it shares a
language **and an interpreter** with `python-cpython` and differs only in the
compiler. A naming convention had no slot for the fact that makes the experiment
clean.

So the tree is now free-form. Move a directory, nest it, rename it: the campaign
is unchanged, because nothing reads it.

### Identity is what a backend *is*

An implementation is `(workload, language, compiler, interpreter)`. There is no name,
because a name is a second thing to keep in sync with the first. Two manifests
declaring the same tuple are one implementation declared twice, and the campaign
refuses to run — they would build the same image tag and collapse into a single
row, and which of the two descriptions the report printed would be a coin toss.

Either half of the backend may be absent, and the absence is a fact worth
publishing: gcc compiles and nothing interprets; CPython interprets and nothing
compiles ahead of the run; Cython does both. The report prints all three columns,
`n/a` included.

### Labels are provenance, never input

Docker labels stay on the images — `langbench.version`, `langbench.flags` — but
the harness does not read them. They describe the artifact for whoever runs
`docker inspect` on it. Anything the harness *acts* on lives in the manifest,
because two sources of truth are one source of truth and one source of drift.

There is a hard reason as well as an aesthetic one: `modes` decides **which
images to build**, so it has to be known before an image exists to inspect. A
build-time label cannot answer a question asked at schedule time.

### Modes

`modes: all` — the normal case for a compiled backend — or an explicit list. An
interpreter declares `strict` alone: CPython has one floating-point semantics,
with no `-ffp-contract` to turn off and no `-ffast-math` to turn on, so `fma` and
`fast` would be the *same image under another tag*. Building them would put three
rows in the report whose only difference is noise, and someone would eventually
read that noise as an effect of the FP mode.

A mode that is requested but not declared is skipped with a warning — a row
missing from a report with no explanation is worse than a redundant one. A mode
that is *misspelled* fails the campaign: under labels we fell back to building
everything, on the grounds that a redundant campaign beats a wrong one, but a
manifest is a deliberate statement and building three images where the author
asked for one is not "redundant" — it is a table carrying rows nobody meant to
publish.

### What stays as it was

One Dockerfile per implementation, no templating: templating Dockerfiles would
badly reinvent the thing Dockerfiles already are, and per-implementation variance
(cargo-chef, `CGO_ENABLED=0`, `uv sync`, `native-image`) lives exactly where
templates are worst. The manifest describes a backend; it does not generate one.

The FP mode, the `-march` baseline and the toolchain version remain **build
args**, not directories — they do not change the Dockerfile's structure. Four
Dockerfiles, not twenty-four. This is Docker's own parameterization, not a codegen
layer of our own invention.

Every base image is pinned **by digest** (`FROM gcc@sha256:…`), never by tag. A
benchmark that silently changes when upstream pushes is not a benchmark.

---

## Container contract

Every image exposes the same `ENTRYPOINT` and takes one of two subcommands:

- `build <threads>` — recompile from a clean state, discard the artifacts.
- `run <n> <max_iter> <threads>` — execute the binary.

Each invocation prints **exactly one JSON object on stdout**, and nothing else.
Compilers and runtimes write to stderr; stdout is reserved for the record. The
harness rejects any other shape rather than measure noise.

```json
{"phase":"run","checksum":31415926535,"elapsed_ns":4102337891,"user_usec":32418004,"system_usec":118273,"peak_bytes":13160448}
{"phase":"build","elapsed_ns":812004221,"user_usec":2914000,"system_usec":204000,"binary_bytes":312840,"binary_stripped_bytes":248904,"text_bytes":41216,"peak_bytes":486539264}
```

Stdout rather than a bind-mounted file: it needs no volume, no per-invocation
temporary directory, and no reasoning about append ordering. Printing the
checksum also happens to be what stops dead-code elimination from deleting the
hot loop.

**The checksum is a JSON integer.** It is a sum of 64-bit iteration counts, and
it is the correctness gate for the whole harness. Anything that rounds it —
`float64` storage, a metrics system, a spreadsheet — destroys the invariant.

`peak_bytes` is `null` on a kernel that exposes neither `memory.peak` nor the
cgroup v1 file. `null`, never `0`: a backend that needed no memory would be a
remarkable claim, and it is not the one being made.

Almost all measurement originates inside the container. The CLI contributes the one
number nothing inside the container can produce: the **external wall-clock** — nothing
in there is alive to time its own creation.

---

## Measurement protocol

**`docker build` prepares. `docker run` measures.** That is the core rule.

### The build phase

The image ships the toolchain, the sources, **and the already-compiled binary**
(produced during `docker build`). That binary is what `run` executes. The `build`
subcommand recompiles from scratch and throws the result away — it exists only to
be timed. Compilers are deterministic, so timing one compilation and executing
another of the same source with the same flags is sound.

The build directory is a `--tmpfs` (with an explicit `size=`; a Rust `target/`
reaches hundreds of megabytes). It is therefore empty on every `docker run`,
because each container starts from the image layers with a fresh writable layer.
No cleanup step, no `--no-cache`, no network.

- **`--network=none` on every measured run.** Not "hopefully no network": a build
  that tries to fetch **fails loudly** instead of silently adding four seconds.
  Belt and braces with `cargo build --offline` and `GOPROXY=off`. This rule is
  also why the container cannot push metrics anywhere.
- **`--tmpfs` on the build directory.** Compilation writes object files;
  overlayfs latency is a noise source we can delete for free.
- **Warm the toolchain cache, never the project's.** Rust ships a precompiled
  stdlib; Go recompiles its own on a cold `GOCACHE`. Without warming we would be
  measuring "Go compiles its standard library" against "Rust does not".
- **The Go trap.** Building the binary in the Dockerfile also populates `GOCACHE`
  with *the project*, so the timed rebuild would be an instant no-op and Go would
  look infinitely fast. The final stage must run `go clean -cache && go build std`:
  hot for the stdlib, cold for our code.
- **Pass the thread count to the compiler too** (`cargo build -jN`, `make -jN`).
  Compilers are parallel.

### The three clocks

| Layer    | Source                    | Captures |
| -------- | ------------------------- | -------- |
| External | CLI, around `docker run`  | container create + runtime init + compute |
| Internal | program, `elapsed_ns`     | compute only |
| Floor    | a `/bin/true` image, 30×  | container overhead alone |

**External minus internal is a metric, not noise.** It is runtime startup cost,
and it is where the JVM and CPython pay their tax.

Do **not** subtract the floor from anything — that would propagate its variance
into every number. Characterize it once and publish it beside the table.

### CPU time comes from the cgroup, never from `rusage`

`wait4()` on the `docker` process returns the rusage of the Docker *client* — a
few milliseconds of argument parsing — because the workload runs under
`containerd-shim`, in a different process tree. You would conclude that Rust
consumes no CPU.

Read `/sys/fs/cgroup/cpu.stat` (`usage_usec`, `user_usec`, `system_usec`) from
inside the container before the entrypoint returns. With cgroup v2 and a private
cgroup namespace — Docker's default — the container sees its own. This is
language-agnostic, unlike `getrusage`.

Wall-clock says *is it fast*. Total CPU time says *at what price*. A runtime whose
scheduler busy-waits burns CPU without gaining a millisecond of wall, and that is
visible only in the gap between the two.

Build time is a **headline result**, not a footnote. The canonical
compile-versus-runtime trade-off — cranelift compiles much faster and emits
slower code — is half the story of a compiler benchmark.

### Parallel efficiency is a median, not a minimum

CPU time over compute time is how many cores a run actually kept busy, and it is
the one number that separates *this backend is slow* from *this backend cannot use
the machine*. Two rows with the same wall-clock, one at 7.8 cores and one at 1.0,
are not two slow backends: one of them is a global interpreter lock, and no amount
of compiler work will ever move it. A wall-clock alone hides that completely — which
is why the harness hands every kernel the same thread count and then reports what
each did with it.

It is derived, not measured: both operands are already on every sample, so the
column is computable on campaigns recorded before it existed.

**Per sample, never as a ratio of two minima.** The same rule as startup: the
smallest CPU time and the smallest compute time come from different rounds, and
their quotient describes a run that never happened.

**The median, and not the minimum.** This is the one place the min-of-N rule does
not apply, and it is worth being precise about why. Min-of-N is licensed by the
*one-sidedness* of contention: a busy machine can only ever make a run slower, so
the smallest sample is the closest the machine came to showing its true capability.
Parallelism has no such asymmetry. Contention inflates the CPU clock — threads
spinning, waiting on each other — and inflates the compute clock too, in whatever
proportion the scheduler happened to pick that round. Nothing recommends the extreme
of a two-sided distribution over its middle, so we take the middle, and we publish
the dispersion beside it like everywhere else.

**It can exceed the thread count, and that is a result rather than an overflow.**
The numerator counts every microsecond of CPU the container burned; the denominator
counts only the span the program timed *itself* over. A JIT compiling on one thread
while the kernel computes on eight is spending CPU that the hot loop's clock never
sees. Clamping that to the thread count, or quietly normalising it into a percentage
that cannot exceed 100%, would be hiding the very thing a reader comparing a JVM to a
static binary is entitled to see. The column reports what the cgroup said.

### Memory is only comparable under a pinned budget

Peak memory comes from the container's own cgroup — `memory.peak`, read by the
entrypoint exactly where `cpu.stat` is. It is the **whole container**: the process
tree, the page cache it faulted in, the tmpfs a build wrote into. Not the resident
set of one process, and deliberately not: the question is what the backend needed in
order to run, not what one of its processes happened to be holding at the end.

Here the minimum is not a statistical estimate but an exact bound. Page cache and a
lazy collector can only ever push a high-water mark *up*; nothing can push it below
what the backend genuinely had to allocate. Min-of-N is the memory it needed.

**The measurement is only meaningful because every measured container runs under the
same pinned `--memory`, and this is a change to the protocol rather than a safety
rail.** A garbage-collected runtime sizes its default heap from what its cgroup shows
it — a JVM takes a quarter of it — and so do Node, Bun and every other runtime with a
collector. Leave the budget unset and they read the *host's* RAM: the peak we publish
would then describe the bench machine, and moving the campaign to a box with twice the
memory would "prove" that Java got hungrier. Pinned, and pinned identically for every
backend, it is a property of the backend again.

Two consequences follow, and neither is optional:

- **Pinning the budget changes the timings.** A constrained heap is a different
  garbage-collection regime. The run column moves. Campaigns recorded under different
  budgets are not comparable to each other — not on memory, and not on time either.
- **The floor is set by the hungriest *compiler*, not by the kernels.** The kernels
  need almost nothing; `native-image` needs gigabytes, and the build-phase tmpfs is
  charged to the same cgroup. A budget that comfortably runs every kernel and quietly
  OOM-kills one toolchain does not produce a smaller number — it produces a
  quarantined backend and a missing row.

Swap is off (`--memory-swap` equals `--memory`). A container permitted to swap does
not fail when it overruns its budget: it silently gets slower, and the timing absorbs
a page-fault storm that no column explains.

### Why there is no energy column

Joules would be the metric that makes a backend comparison worth reading. The harness
measured them, briefly, and the column is gone. It is worth writing down why, because
the argument for adding it back is very good and the reason it fails has nothing to do
with the argument.

Energy is the one measurement a container cannot take for itself. `cpu.stat` and
`memory.peak` are cgroup files — namespaced, so the entrypoint reads its own and
reports it. RAPL is not. `/sys/class/powercap` describes a **socket**, not a cgroup,
and it is invisible from inside a container. So it has to be read on the *host*, around
the `docker run`.

And on the machines this project actually runs on, it cannot be read at all:

- **AArch64 has no counters.** RAPL is x86 (AMD drives the same `intel-rapl` powercap
  zones, misleading name and all). There is no equivalent to fall back to.
- **The x86 runner's counters are unreadable.** Since the PLATYPUS side-channel,
  distributions ship `energy_uj` root-only, and a GitHub Actions runner does not hand
  out the host's `/sys`.

The campaigns are run in CI, on those runners, and the result was not *some* missing
rows. It was `energy_uj: null` on **every sample of every campaign, on both
architectures** — 1140 nulls, and not one number. The bench machine is the CI runner;
there is no other machine, and a column the bench machine can never fill is not a
measurement. It is a promise.

A column that reads `n/a` on every row of every published campaign is worse than no
column. It invites the reader to assume a future campaign will fill it, and it keeps a
whole reading path alive — a meter, a wire format, a unit in a closed enum, a chart, a
docs section — for a number that has never once been produced. When the machine that
publishes changes, this section is the argument to re-open, not the code to un-delete.

### The build column reports the internal clock, the run column the external one

Both phases record both clocks — the sample carries `wall_ns` and `elapsed_ns`
either way — but the report headlines a different one for each, and the asymmetry
is deliberate.

The run row headlines the **external** clock, because what sits between the two is
the runtime's startup: a JVM booting, an interpreter loading. That is a property
of the backend, and therefore a result.

The build row headlines the **internal** clock, because what sits between the two
is Docker creating a container. That is a property of *our* isolation choice, and
therefore an artefact. Docker is in this project so that six toolchains need not be
installed on the host; the less it appears in a measurement, the better. It is not
a small constant either: container creation runs to ~110 ms on the reference host,
while gcc compiles a single-file kernel in ~30 ms. Headlining the wall-clock would
charge every compiler the same large tax, flatter the slow ones, and compress a
4.7× ratio into 1.3×. Publishing that as "build time" would be publishing Docker.

The wall-clock of a build is still written to `samples.ndjson`, like everything
else. It is simply not what the column reports.

### Binary size

Three numbers, all cheap, recorded once per implementation in the `build` record
since they are constant across repetitions. We measure the binary the image
ships — the one `run` executes — not the throwaway from the timed rebuild.

- `binary_bytes` — the file on disk, as shipped.
- `binary_stripped_bytes` — after `strip`. We do not strip during the timed
  build; that would add link-time work to a number we are timing.
- `text_bytes` — the `.text` section, from `size(1)`.

**Only `.text` is comparable across implementations.** Total file size measures
linking policy, not codegen: gcc dynamically links libc and looks tiny while the
code lives in `libc.so`; Rust statically links its stdlib; Go embeds a runtime
and its type metadata. Ranking languages by file size ranks their packaging.

`.text` is exactly the emitted code, and it is the **cost side of the
optimization trade**: inlining, unrolling and vectorization all inflate it in
exchange for speed. Plotting `.text` against runtime is the point of a compiler
benchmark, not a curiosity — it is also where `-O2` versus `-O3` shows its hand.

**But calibrate your expectations.** On a kernel this small, `.text` is around a
kilobyte and function-entry alignment padding quantises it. Measured on C/gcc,
`fma` mode emits three fewer floating-point instructions than `strict` — twelve
bytes — and `.text` does not move at all. The column earns its keep on larger
kernels, or when a backend vectorizes and another does not. For anything finer,
read the disassembly; that is what we archive it for.

**And never read `.text` as a proxy for speed.** Cython emits 50.5 KiB of machine
code against C's 1.3 KiB — thirty-nine times more — and runs forty-two times
slower. The disassembly says why in one line: Cython's hot loop is 142 `bl`
instructions into the CPython C-API and a single `fadd`, where the C kernel has
six `fadd`, five `fmul`, three `fsub` and no call at all. More code, doing less
arithmetic. `.text` is the *cost* of an optimization, never its reward.

Interpreted and JIT backends emit no artifact: the field is `null`, not zero.
`native-image` does produce one, so "compiled" is a property of the backend, not
of the language.

We archive `objdump -d` of the hot loop alongside the results. Three lines of
Dockerfile. When clang comes out 3× ahead of gcc we do not speculate about the
vectorizer — we look for the `vmulpd`.

### Source size, and what it is not

`source_bytes` is the size of the one kernel file the manifest declares. The manifest
declares it rather than the harness guessing it: the alternative is to pattern-match
the filenames sitting beside the Dockerfile, which is parsing the path under another
name, and [the path is not metadata](#the-path-is-not-metadata).

**It is a property of the language, not of the backend, and it is honest about that.**
`c` / `gcc` and `c` / `clang` compile the same `mandelbrot.c`, so they report the same
number and the head-to-head calls it a tie. That is not a weakness of the column; it is
the column telling the truth about the one axis this project exists to measure. Every
other number here separates two compilers on identical source. This one cannot, and
says so.

**It is not a measure of quality, and it is not a measure of effort.** It is one
author's kernel, in one style, under this repository's rules: zero dependencies, one
file, threads handed in from `argv`, a checksum printed. It says how much text a
language needed to express *this* workload under *those* constraints. It does not say
a language is verbose, and it emphatically does not say how much work it was to write.

That last distinction is why the column is **bytes and not tokens**. Counting tokens
would answer a question nobody here asked — the tokens in a finished file are not the
tokens it cost to produce one, which is prompt plus reasoning plus every attempt that
failed the checksum — and it would make the number depend on some vendor's tokenizer,
so that a figure in this repository could change because somebody else shipped a model.
Bytes are stable, vendor-neutral, and will mean the same thing in ten years.

### Sampling

The harness runs the loop itself. `hyperfine` is excellent, and fine for
prototyping a single image, but it cannot capture per-run stdout — so it cannot
join the external wall-clock to the program's self-reported `elapsed_ns`. Min,
median and MAD are fifty lines with no dependency; the protocol is the hard part,
not the arithmetic.

- **Size `n` so that a run lasts 2–5 s.** Long enough to swamp container startup
  and scheduler quanta, short enough that CI finishes. Builds take what they
  take; 5–10 repetitions suffice.
- **Interleave round-robin.** Outer loop over rounds, inner loop over
  implementations. Never the reverse. Thirty gcc runs followed by thirty clang
  runs lets the hourly log flush land on one of them and become a bias
  indistinguishable from the effect under study. Blocking converts variance into
  bias, and no number of repetitions removes bias.
- **Keep the warmup samples, flagged.** The first run of an image faults its
  layers into page cache. Mark them in the data rather than deleting them; the
  day something looks wrong, you will want to see them.
- **Verify the checksum on every run**, not once. A run with the wrong value is
  not a slow run, it is a wrong run, and it must never enter the statistics.
- **Store raw samples, never aggregates.** One NDJSON line per run, with the
  machine metadata and the campaign's parameters in a header record. Aggregates
  are recomputed at report time; a discarded sample is gone forever. This is the
  highest-return rule in the protocol, and the one most regretted later.
- **Every sample carries its backend's manifest**: language, compiler,
  interpreter, description, comments, copied onto each line. It is deliberate
  repetition. A sample has to say what produced it *without a second file to join
  against*: the manifest will be edited, the directory will be renamed, the
  backend will be deleted — and the samples must still describe the campaign that
  actually ran. A foreign key into a file that changes underneath is not a
  record, it is a dangling pointer.

### A backend that fails is not a campaign that fails

Sixteen backends, three modes, an hour of wall-clock. One of them segfaults in
round seven. Aborting there throws away fifteen backends' worth of correct
measurement to report a fact about the sixteenth — and on a bench machine you
booked for the evening, you find out about it in the morning.

So a failure is **quarantined, never propagated**. The unit — one
`(implementation, mode)` pair — is taken out of the campaign at the point it
breaks, and the remaining rounds carry on without it. This covers every way a
backend can fail us:

- the image does not build,
- the container crashes, or is killed for exceeding `--run-timeout`,
- it prints a record the harness cannot parse,
- its `strict` checksum disagrees with the reference.

The last one is the interesting case, because it is not a crash: the run
*succeeded*, and produced the wrong answer. It is quarantined for exactly the same
reason as the segfault, and the reason is the one rule that governs this whole
file — **a wrong run never enters the statistics**. A backend that is wrong is not
slow, and giving it a row would be worse than giving it none.

Three consequences, all deliberate:

- **The unit is quarantined, not the implementation.** A backend whose `fast`
  build is broken still has a `strict` row to publish, and that row is worth
  having.
- **It is never retried.** Whatever broke in round one breaks in round nine, and
  a campaign that re-learns it every round pays for the lesson in wall-clock.
- **The campaign still exits 0** — with one exception. If *every* unit failed there
  is no campaign, only a list of things that broke, and a samples file with a
  header and nothing under it renders into an empty table. An empty table is a lie
  told quietly, so that, and only that, is a non-zero exit.

And the failure is **published**. It goes into `samples.ndjson` as a `failure`
record beside the samples, and every rendering shows it: `report.md` grows a *What
did not finish* section, the website grows the same table. This is not
bookkeeping. A benchmark that silently omits what did not work flatters itself —
and a row that is missing from a table looks exactly like a backend nobody ever
bothered to write, when in fact it is a backend that crashed. The reader has to be
able to tell those two apart, and the only place they can learn the difference is
the file.

Quarantine changes nothing about the rows that *did* finish. Each is an
independent run of an independent image; nothing about the C row is contaminated
by the Rust one having died, and the sample count printed in each row (`Runs`)
already says how many draws its minimum came from.

### Why min-of-N, not the median

Contention noise is **one-sided**: it can only slow you down, never speed you up.
The median of a distribution pushed against a hard floor is not the true value.

So we report the **minimum** as the estimate of the machine's capability, and we
keep the dispersion beside it as a **quality signal for the campaign**. If the
spread is wide, the measurement is worthless — including its minimum. The
dispersion is not an error bar on the result; it is a verdict on the run.

### A difference smaller than the dispersion is not a difference

A table of minima invites one operation, and every reader performs it: divide two
rows. That ratio is the only cross-backend claim this project publishes — and the
campaign is entitled to it **only when the gap survives its own noise**.

Two rows whose minima differ by 3%, on a campaign whose rows each wobble by 9%,
are not a 3% result. They are the same number, measured twice, on a machine that
was busy. The minimum of one happened to fall lower than the minimum of the other,
and a second campaign on the same hardware would as happily reverse them. So the
verdict is a **tie**: not *equal* — *indistinguishable*, which is a statement about
this campaign and not about the backends.

The bar a gap has to clear is the **worse** of the two rows' dispersions: a claim
about a pair is only as defensible as its shakier half. And a row with fewer than
three samples has **no known dispersion** — the median absolute deviation of two
observations is structurally zero, and a structural zero is not a quiet machine.
It buys the pair no tolerance.

This is why the site's head-to-head is computed in `src/compare.rs` and not in the
browser that displays it. What counts as a difference is a definition of what this
project measures, and it lives beside min-of-N — one definition, one place, for the
same reason the site does not re-implement the statistics it plots.

A benchmark that reports `1.03×` where it should report *we cannot tell* has not
made a small error. It has made the only error that matters.

---

## Where it runs

**Measure the machine before measuring the backends.**

Run the same binary thirty times and look at the median absolute deviation. This
is a twenty-minute experiment and it decides what we are allowed to claim:

- MAD under ~2% → percentage-level claims (gcc versus clang) are defensible.
- MAD around 15% → conclusions stop at factors, not percentages. Document it.
  That is already more honest than most published benchmarks.

The noise-floor run is also the harness's first integration test: the same code
path, with the machine as the subject.

### CI (GitHub Actions)

- **All implementations of an architecture run in the same job, sequentially.** A matrix
  with one job per implementation would compare Rust on one physical machine to
  Go on another, and the result would be meaningless.
- One job on `ubuntu-latest`, one on `ubuntu-24.04-arm`.
- Machine metadata is recorded on every run. When two campaigns disagree, it is
  the first thing to check.
- Hosted runners have two to four hyperthreaded vCPUs and noisy neighbours.
  Scaling curves from CI are indicative only.

### Dedicated hardware

For percentage-level claims: a bare-metal node, `kubectl cordon`-ed out of the
scheduling pool, driven over SSH with a plain `docker run` — **not as a pod**.
That removes kubelet, containerd, cadvisor, the CNI daemon and the entire
DaemonSet argument in one move. Kubernetes manages the cluster; it does not need
to manage this.

On that node:

- `performance` governor, **turbo disabled**. With turbo on, repetition 1 runs at
  5 GHz and repetition 30 at 3.4 GHz because the package heated up. That is
  drift, not noise, and no median rescues it.
- Optionally `isolcpus` / `nohz_full` / `rcu_nocbs`, which remove the cores from
  the Linux scheduler entirely.

If the benchmark must run *inside* Kubernetes, the mechanism is Guaranteed QoS
plus the static CPU Manager policy (`full-pcpus-only`), reserved CPUs for the
system, and an audit of every DaemonSet that tolerates your taint. It is more
work than cordoning a node, for a worse result.

**Verify, do not trust.** Before any campaign: check `Cpus_allowed_list` in
`/proc/<pid>/status`; confirm `nr_throttled` is zero in the cgroup's `cpu.stat`;
read `scaling_cur_freq` *during* the run; and interleave a fixed calibration
sentinel at the start, the middle and the end. If the sentinel drifts, discard
the campaign — that is the thermal throttling you did not see coming.

---

## Observability: the machine, not the measurement

Two kinds of data, two stores. They must not be mixed.

**Measurement data goes in a file.** Exact, complete, archivable. A campaign is
an NDJSON file you commit, diff, and reread in two years. Thirty discrete
observations, where a lost sample is a silent hole in the result.

**Environment data goes in Prometheus.** Dense, sampled, disposable.
`node-exporter` on the bench node at a one-second scrape turns the "verify, do not
trust" list into a dashboard: CPU frequency, package temperature, throttle
counters, plotted across the campaign. When round 22 comes in 8% slow, you look
at the graph instead of speculating.

Pin the exporter to the **reserved** cores. A monitor that perturbs what it
monitors is a classic of the genre.

### Never push benchmark metrics to Prometheus

Three independent reasons, each sufficient on its own:

1. **Pushing needs network**, and a network namespace cannot be added mid-run.
   Giving the container network for its whole life destroys the `--network=none`
   guarantee, trading a structural invariant for a convention.
2. **Prometheus stores `float64`.** The checksum is a sum of 64-bit integers; past
   2⁵³ it stops being exact. The bit-identical strict-mode invariant — the thing
   that catches the bugs tests do not — would be silently lost.
3. **A TSDB is lossy by design** (a missed scrape is fine, the next arrives in
   fifteen seconds) and pull-based (a container that lives four seconds is never
   scraped). Keeping all thirty repetitions would mean encoding the round number
   in a label — using a time-series database as a key-value store. That is the
   smell that says it is the wrong database.

---

## Claims we do not make

For the avoidance of doubt, `langbench` does not, and will not, tell you:

- **Which language is fastest.** It compares backends. Cross-language numbers are
  confounded by construction and are valid only for orders of magnitude.
- **Whether ARM is faster than x86.** Absolute cross-architecture timings are meaningless
  here; only within-architecture ratios are compared across architectures.
- **Which compiler is better.** On Mandelbrot it tells you which one vectorizes a
  divergent-exit loop. That is one optimizer pass, not a compiler.
- **That a 3% difference is real**, unless the campaign's dispersion says it can
  be. The dispersion is published next to every number for exactly this reason.
