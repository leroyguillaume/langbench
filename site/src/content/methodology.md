---
title: Methodology
summary: How langbench measures compiler and runtime backends — the protocol, the statistics, and the claims it refuses to make.
---

This page explains how the tests are actually performed. A benchmark whose methodology is
not published is worth nothing, so it is a deliverable, not a footnote. If you are here to
dispute a number, this is the right place — and if you are in a hurry, start with
[What this does not tell you](#what-this-does-not-tell-you).

## What is under test

**Compiler and runtime backends, not languages.**

The primary question is: *given the same source, how do different backends compare?*
gcc versus clang on identical C. rustc-LLVM versus rustc-cranelift on identical Rust.
CPython versus PyPy. OpenJDK versus GraalVM `native-image`.

The unit of comparison is therefore not a language but a tuple:

> (compiler, version, flags, target architecture)

There are two axes here, and they are two tables that are never merged:

1. **Same source, different backend.** The real experiment. gcc versus clang on
   identical C; rustc-LLVM versus rustc-cranelift on identical Rust. Clean, and
   the reason this project exists.
2. **Same workload, different language.** Confounded by construction: different
   source, different runtime, different standard library. Valid for orders of
   magnitude ("Python is roughly 80× slower than Rust"), never for percentages.

Cross-language comparison is a secondary, much weaker result, and this project treats it
as one.

## The work

A **workload** is the work itself: what it is, how it is sized, and what the right
answer is. A workload is *not* an algorithm. Mandelbrot is one; a JSON parser, an HTTP
server, a cold start are others, and nothing in the harness assumes the work is a
computation over a grid.

What a *given* workload puts under the light — and, just as importantly, what it says
nothing about — is declared in its own `workload.yaml` and shown on its page. It is not
repeated here. This section is the contract that holds whatever the work is: the rules an
implementation obeys so that the number it produces is about the backend and not about
the way the benchmark was written.

### Zero third-party dependencies

**No third-party dependencies. None.** Rust uses `std::thread` and an `AtomicUsize`
chunk counter, not `rayon`. Otherwise the timed build compiles eight thousand lines of
rayon while gcc compiles fifty lines of C, and the build-time column means nothing. It
also removes every question about pre-building dependencies. Each implementation is a
single source file.

### Work is handed out dynamically

Where a workload's units of work cost different amounts, the load is **imbalanced**, and
imbalanced on purpose: in Mandelbrot the interior pixels run to the iteration ceiling
while the exterior ones exit after a few iterations. Chunking must therefore be dynamic
— at least `4 × threads` chunks handed out on demand. A static contiguous split
measures the split, not the backend.

### Anti-cheating contract

- **The params come from `argv`**, and so does the thread count. Never compile-time
  constants — a backend may otherwise constant-fold the entire computation away. This
  is why a workload declares its `params` as an ordered list: the order is the order
  the kernels receive them on the command line.
- **The checksum is printed.** Never discard it, or dead-code elimination deletes the
  loop and the benchmark measures nothing at infinite speed.
- **The thread count is an explicit argument.** Implementations must never call
  `available_parallelism`, `os.cpu_count()`, `runtime.NumCPU()` or equivalent.
  Those functions disagree about cgroup quotas across runtimes: Rust reads the
  cgroup v2 quota, CPython does not, Go only learned to in 1.25. Auto-detection
  would measure "does this runtime read `/sys/fs/cgroup`", not parallel speed.

  (The *harness* auto-detects a default for `--cpu`. That is correct: it then
  passes the value explicitly. The prohibition applies to the kernels.)

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

### The languages that fuse behind your back

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

## Flags, and the architecture baseline

- **Never `-march=native`.** The CPU model varies between runs; the architecture baseline
  would vary with it. Pin an explicit baseline per architecture (e.g. `x86-64-v3`) as a
  build arg and record it in the results.
- `x86-64-v3` and any AArch64 baseline are **not equivalent** and we never claim
  they are. NEON is 128-bit wide — two `f64` lanes. AVX2 is 256-bit — four. A
  factor of two on a vectorized kernel comes straight out of the architecture and has
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

### The JVM cannot honour this rule, and says so

HotSpot has no `-march`. C2 compiles for **whatever CPU it finds at run time** —
which is exactly the `native` targeting forbidden everywhere else in this
methodology, and the JVM rows get it whether we like it or not. There is no flag that
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

### The architecture rule

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

### A toolchain that does not exist is not a slow toolchain

Some backends cannot be built on some architectures at all. Kotlin/Native ships
host compilers for `linux-x86_64`, macOS and Windows — and none for
`linux-aarch64`. There is no flag that fixes this.

The two ways around it are both forbidden here. **Emulation** (QEMU / `binfmt`) is
banned outright: a benchmark run under emulation measures the emulator.
**Cross-building** would let the image build, but the Build column would then
report a compile that happened on another architecture than the run, which is a
number about nothing.

So the manifest declares it — `architectures: [x86_64]`, defaulting to `all` — and a
campaign on the other machine **skips the row loudly at discovery**, before
spending a `docker build` on finding out. The row is absent from that campaign's
table, and the log says exactly why. A missing row with a reason is a result; a
missing row without one is a bug.

## How a run is measured

**`docker build` prepares. `docker run` measures.** That is the core rule.

Almost all measurement originates inside the container. The CLI contributes the one
number nothing inside the container can produce: the **external wall-clock** — nothing
in there is alive to time its own creation. Every image exposes the same entrypoint and
prints exactly one JSON record per invocation; that contract is specified in the
repository's [README](../../README.md#the-container-contract).

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
  Belt and braces with `cargo build --offline` and `GOPROXY=off`.
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
- **Nothing is stripped during the timed build.** `strip` is link-time work, and this
  is a number we are timing: the stripped size is measured afterwards, on the binary
  the image ships.

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

### The build column reports the internal clock, the run column the external one

Both phases record both clocks — the sample carries `wall_ns` and `elapsed_ns`
either way — but the table headlines a different one for each, and the asymmetry
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

## Sampling, and what may be concluded

The harness runs the loop itself. `hyperfine` is excellent, and fine for
prototyping a single image, but it cannot capture per-run stdout — so it cannot
join the external wall-clock to the program's self-reported `elapsed_ns`. Min,
median and MAD are fifty lines with no dependency; the protocol is the hard part,
not the arithmetic.

- **Size the work so that a run lasts 2–5 s.** Long enough to swamp container startup
  and scheduler quanta, short enough that CI finishes. Builds take what they
  take; 5–10 repetitions suffice.
- **Interleave round-robin.** Outer loop over rounds, inner loop over
  implementations. Never the reverse. Thirty gcc runs followed by thirty clang
  runs lets the hourly log flush land on one of them and become a bias
  indistinguishable from the effect under study. Blocking converts variance into
  bias, and no number of repetitions removes bias.
- **Keep the warm-up samples, flagged.** Recorded like any other round, and left out of
  the numbers — see below.
- **Verify the checksum on every run**, not once. A run with the wrong value is
  not a slow run, it is a wrong run, and it must never enter the statistics.
- **Store raw samples, never aggregates.** One NDJSON line per run, with the
  machine metadata and the campaign's parameters in a header record. Aggregates
  are recomputed when the samples are rendered; a discarded sample is gone forever.
  This is the highest-return rule in the protocol, and the one most regretted later.

### Warm-up rounds

The first round of every implementation is run and recorded exactly like the others, and
then left out of the published numbers. **A program's first run is its worst one**, and
for reasons that have nothing to do with the backend: the image's layers are being
faulted into the page cache, a JIT has not compiled the hot loop yet, a JVM is still
loading classes. It says more about the machine getting going than about what is under
test.

**Nothing is deleted.** The rounds are in `samples.ndjson`, flagged `warmup`, and the
website has a box that folds them back into the aggregation. That is deliberate: an
exclusion you cannot inspect is an exclusion you have to take on trust, and this project
asks for none. Turning them on re-aggregates the campaign — with the harness's own code,
over the same samples — and the numbers on screen say so while it is on, because they are
then not the figures this project publishes.

What happens when you do is itself the argument for excluding them. **Run min cannot go
up**: a minimum taken over more samples can only fall or stay, and the warm-up round is
the slow one, so it stays. **Dispersion** is a *median* absolute deviation, built to
ignore a single outlier — which is precisely what a warm-up round is — so it barely
moves. The one column that always changes is the sample count. A table that hardly
flinches is the demonstration that the exclusion is hiding nothing; a row that *does*
lurch has an expensive first run, and that row's samples are worth reading.

Warm-up rounds are a **property of the campaign**, not of the reader: the count is
`--warmup-rounds`, it is recorded in the campaign header, and a campaign run with zero of
them has nothing to fold in.

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
methodology — **a wrong run never enters the statistics**. A backend that is wrong is
not slow, and giving it a row would be worse than giving it none.

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
record beside the samples, and every rendering shows it: the website grows a *What
did not finish* table beside the results. This is not bookkeeping. A benchmark that
silently omits what did not work flatters itself — and a row that is missing from a
table looks exactly like a backend nobody ever bothered to write, when in fact it is
a backend that crashed. The reader has to be able to tell those two apart, and the
only place they can learn the difference is the file.

Quarantine changes nothing about the rows that *did* finish. Each is an
independent run of an independent image; nothing about the C row is contaminated
by the Rust one having died, and the sample count printed in each row (`Runs`)
already says how many draws its minimum came from.

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

## What this does not tell you

For the avoidance of doubt, `langbench` does not, and will not, tell you:

- **Which language is fastest.** It compares backends. Cross-language numbers are
  confounded by construction and are valid only for orders of magnitude.
- **Whether ARM is faster than x86.** Absolute cross-architecture timings are meaningless
  here; only within-architecture ratios are compared across architectures.
- **Which compiler is better.** It tells you how each one handles the loop shapes of
  one workload — on Mandelbrot, which of them vectorizes a divergent-exit loop. That
  is one optimizer pass, not a compiler. Broader claims would require a suite: a
  scalar dependency chain (n-body), a pointer-chasing kernel (alias analysis), a
  branchy kernel. Each workload says what it puts under the light, and what it does
  not, on its own page.
- **That a 3% difference is real**, unless the campaign's dispersion says it can
  be. The dispersion is published next to every number for exactly this reason.

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
