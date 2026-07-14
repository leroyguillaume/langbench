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

## The ISA target

Two modes, built from the same source via one build arg. The axis is **which
machine the code is for** — not floating-point semantics, and not "optimization on
or off". Every mode is `-O3`, and every mode is strict IEEE 754.

| Mode       | Flags                                 | Meaning |
| ---------- | ------------------------------------- | ------- |
| `baseline` | `-march=x86-64-v3` / `-march=armv8.2-a` | A pinned ISA, identical for every backend on the architecture. The binary does not depend on the CPU that built it. |
| `native`   | `-march=native`                       | Whatever this CPU offers, resolved by the toolchain against the machine it is on. |

The two are the two answers a toolchain can give to *"which machine is this code
for?"* — and **which answers a backend is capable of giving is the subject of this
project.**

An ahead-of-time compiler must choose. It emits machine code before it knows where
that code will run, so it either targets a floor every CPU of the architecture
clears, or it targets the CPU in front of it and gives up portability. Both are
real builds of the same source, so both are built, and the gap between them is
**what portability costs**.

A JIT does not choose. It generates code at run time, on the machine it is running
on, and it uses that machine's instruction set whether anybody asks it to or not.
HotSpot has no `-march`; neither does V8, nor JavaScriptCore, nor PyPy's tracing
JIT. They declare `modes: [native]`, and it is not a limitation they are apologising
for — **it is what a JIT sells.** The specialisation an ahead-of-time compiler buys
by giving up portability, a JIT gets for free, because it compiles late enough to
know the answer.

That asymmetry is not a flaw in the measurement. It is the measurement.

### Two exceptions, and each is worth knowing

**Julia is a JIT that can be pinned.** It compiles at run time like the others, but
it takes `--cpu-target`, and it *validates* the name — one of the few toolchains
here that rejects a CPU it has never heard of rather than silently falling back. So
Julia declares both modes. It proves that the `baseline` column is not "the
ahead-of-time compilers": it is "the toolchains that let you choose the target",
and the two sets are not the same. A JIT is native **by default**, not by necessity.

**CPython is neither.** It compiles nothing ahead of the run, and it has no JIT —
3.13's is experimental, off by default, and not enabled in the image we pin. No machine
code is ever generated for this CPU, at any point. The code that executes the hot
loop is CPython's own `eval` loop — a C interpreter compiled by whoever packaged it,
and we checked: there is **no `-march` anywhere in its CFLAGS**. It was built for the
toolchain's floor, plain `x86-64`, because an image that has to start on every machine
there is cannot assume otherwise. So CPython declares `baseline` — the column of "did
not get the machine", which is the column a reader wants to read it against — while
the ISA it actually got sits *below* the baseline every compiled row was held to.
Which brings us to the next section.

### The mode is a request; the ISA is what came back

Every sample carries **both**, because they disagree, and every disagreement below
is real:

| Row | Mode | ISA it actually got |
| --- | ---- | ------------------- |
| `c-gcc`, `rust-rustc`, `zig`, … | `baseline` | `x86-64-v3` — the contract, honoured |
| the same, native | `native` | `native` |
| every JIT row | `native` | `native` — same word, and that is the finding |
| **`go-gc`** | `native` | **`v3`** on an AVX2 machine — Go has no `-march=native`, so the entrypoint detects the CPU and names the psABI level it clears. Ties with its own baseline, because it *is* its own baseline |
| **`native-image`, AArch64** | `baseline` | **`armv8.1-a`** — GraalVM offers no `armv8.2-a`, so it takes one level *below* the campaign's |
| **`python-cpython`** | `baseline` | **`distro`** — an interpreter built with no `-march` at all, at the toolchain's floor |

Before this column existed, every one of those divergences lived in the free text of
a manifest's `comments` field — which is to say, **nowhere a reader of the table
would ever find them.** A column whose meaning varies from row to row is not a sin.
A column whose meaning varies from row to row *in silence* is, and it is the exact
sin this project spent a schema, a generated manifest and a validator trying to
prevent everywhere else.

So the row says what it got. When the mode and the ISA agree, the pair is boring and
you may ignore it. When they disagree, the row is telling you something you would
otherwise have had to already know.

### `native` is not `fast-math`, and the distinction is load-bearing

`-march=native` decides **which instructions** the compiler may emit. `-ffast-math`
decides **what arithmetic means**. They are orthogonal, and only the first one is in
this table.

Widening a vector reorders nothing. Under strict IEEE 754 semantics, a `native`
build and a `baseline` build of the same source compute **the same bits** — the
compiler is allowed to do four multiplications at once, and it is not allowed to do
them in a different order. So the checksum holds across both modes, and `-ffast-math`
is spelled nowhere in this repository.

This is why the ISA axis could replace the floating-point one without weakening
anything. The old `fma` and `fast` modes were *licensed to compute a different
number*, which meant the correctness gate covered only the `strict` rows — the two
modes most likely to expose a miscompilation were the two nobody checked. The new
axis grants no such licence. **Every sample, in every mode, is now held to the
checksum**, and a divergence quarantines the backend on the spot.

Whether relaxed floating-point is worth what it costs is a real question, and this
project no longer answers it. It could not answer it honestly anyway: `-ffast-math`,
`-Ofast`, Zig's `@setFloatMode(.optimized)` and the JVM's *nothing at all* are not
the same relaxation, so a `fast` column was comparing rows whose treatment differed
by row. That question belongs to a campaign of its own, not to a permanent column of
every table.

### The strict-mode invariant

Strict IEEE 754 arithmetic — in **every** mode — is the **correctness gate for the
entire harness**.

Mandelbrot uses only multiply, add, subtract and compare. All four are correctly
rounded under IEEE 754 — the result is specified to the bit, and both x86-64 (on
SSE2, not the old 80-bit x87) and AArch64 conform. With no FMA contraction, no
reassociation and no denormal flushing, the checksum **must be bit-identical
across every compiler, every language, every ISA target and both
architectures.**

One reference value. Every implementation across ten languages — C, C++, Rust, Zig,
Go, Julia, Python, JavaScript, TypeScript — and **every one of them agrees on it,
bit for bit**, from `gcc -O3` to a JIT-compiled Julia script to JavaScript in a Bun
worker. Any divergence is a bug — in the code, in the flags, or in our understanding
of them. Never a rounding excuse.

**And now it binds every row.** Under the old floating-point axis this claim was
about the `strict` rows alone: `fma` and `fast` were licensed to compute a different
number, so the gate that catches a miscompilation was switched off for exactly the
two modes most likely to contain one. The ISA axis licenses nothing — a wider vector
executes the same arithmetic in the same order — so the harness now checks the
checksum on **every sample it records**, `baseline` and `native` alike, warmup rounds
included. A backend that disagrees is quarantined at the sample that disagreed.

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

This is the difference between C and Go here. C says "do not contract" on the command
line (`-ffp-contract=off`), where it is visible and where a flag could have flipped
it. Go says it in the *source* — so a fused Go build is a different **program**, not
a different flag.

**Zig relaxes in the source too**, for the same structural reason:
`@setFloatMode(.optimized)` is a statement inside the program, not a compiler flag.

Both facts used to be an argument about *modes*: since neither language can relax its
arithmetic with a build arg, neither could offer an `fma` image, and both declared
`modes: [strict]`. The floating-point axis is gone and that argument with it — but the
rounding points in `mandelbrot.go` are not decoration, and deleting them would still
silently change what the program computes. They are load-bearing for the same reason
they always were, and now they are load-bearing in **both** modes.

**It is a necessary condition, not a sufficient one.** The gate sees a change only
when it flips a pixel's iteration count, so a perturbation that lands nowhere near
a boundary is invisible: shifting `X_MIN` by one ULP changes nothing at 200×200
with `max_iter=100`. Sensitivity grows with grid size and iteration ceiling, since
both increase the number of pixels sitting on a boundary. A passing checksum means
"no evidence of divergence at this resolution", not "provably identical".

**There is no mode we do not gate.** There used to be: `fma` and `fast` were scored
against the strict reference rather than held to it, and their divergence was
published as a `Δ strict` column — the speed gained and the precision sold, side by
side. Both the licence and the column are gone. `baseline` and `native` compute the
same bits, so a divergence in either is not a trade-off with a magnitude worth
printing. It is a wrong run, and it takes the backend out of the campaign.

## Flags, and the architecture baseline

- **`native` is a mode, never a default.** A native build depends on the CPU that
  built it, so it is asked for explicitly, published as its own row, and never
  allowed to stand in for the pinned one. What is forbidden is not the flag — it is a
  *baseline* that quietly varies with the machine. The `baseline` mode pins one; the
  `native` mode says out loud that it does not.

  This rule used to read *"Never `-march=native`, in any toolchain, under any
  spelling"*, and it was quietly unenforceable. A JIT compiles on the machine it runs
  on: HotSpot, V8 and PyPy were native the whole time, whatever the rule said. The ban
  never stopped the JVM from getting the machine. It only stopped **gcc** from getting
  it — and then called the result a level playing field.
- `x86-64-v3` and any AArch64 baseline are **not equivalent** and we never claim
  they are. NEON is 128-bit wide — two `f64` lanes. AVX2 is 256-bit — four. A
  factor of two on a vectorized kernel comes straight out of the architecture and has
  nothing to do with the compiler.
- Pin and document everything that trades compile time against runtime speed:
  Rust's `codegen-units`, `strip`, the linker (`ld` / `lld` / `mold`). Otherwise
  we benchmark a default rather than a decision.

### Every toolchain spells the target differently, and some ignore it silently

The harness speaks gcc: it hands every backend `MARCH=x86-64-v3`, `MARCH=armv8.2-a`,
or the literal `MARCH=native`. Only the C and C++ compilers take that verbatim. Each
of the others translates it in its entrypoint — `-C target-cpu=` for rustc, `-mcpu=`
for zig, `GOAMD64=` for go, `--cpu-target=` for julia — and **a target the toolchain
cannot express must fail the build, loudly, or be reported as what it settled for.**

That rule is not defensive pedantry. Measured:

- **rustc only warns.** `-C target-cpu=armv8.2-a` prints *"not a recognized
  processor (ignoring processor)"* and hands back a generic binary — and it says
  exactly the same thing about `-C target-cpu=nonsense-v9`. A campaign would run
  to completion and publish a row claiming an ISA target it was never compiled
  for. The Rust entrypoint therefore both translates the name *and* greps rustc's
  stderr for that warning, failing if it appears.
- **Go silently no-ops**, and **Go has no `native` at all.** `GOAMD64=v3` on an arm64
  build is not an error; it is ignored. And `GOAMD64` accepts only the psABI levels
  `v1`…`v4` — there is no way to say "this exact CPU". So the entrypoint does the CPU
  detection `-march=native` would have done, and names the highest level the machine
  actually clears. Naming the *ceiling* instead (`v4`) would be a claim, not a
  measurement: v4 is AVX-512, the Go runtime verifies it at startup, and on an AVX2 bench
  machine that binary refuses to run — the `native` row would delete itself. Detected, it
  answers `v3`, ties with its own baseline, and that tie is the finding: Go has no
  instruction set left to reach for here.
- **Julia defaults to `native`**, so the baseline must always be passed explicitly —
  a forgotten flag there does not fail, it publishes a native row in the pinned
  column. To its credit it is one of the few toolchains here that *rejects* a name it
  does not know, which is why it is the one JIT in this table that can honour both
  modes.
- **OpenJ9 ignores unknown `-XX:` options entirely.** Measured:
  `java -XX:CompleteNonsenseFlag=42 -version` starts happily, where HotSpot refuses
  to boot on the same flag. Any flag we passed it hoping to pin something would have
  pinned *nothing* while the manifest claimed otherwise.

A build that quietly falls back to generic does not break the campaign. It
publishes a wrong number with a straight face, which is worse.

### The JIT gets the machine for free, and that is the result

HotSpot has no `-march`. C2 compiles for **whatever CPU it finds at run time**, and
there is no flag that pins an ISA the way `-march=x86-64-v3` does for gcc. The same
is true of OpenJ9's Testarossa, of Graal-as-a-JIT, of V8 and JavaScriptCore, and of
PyPy's tracing JIT.

For a long time this page called that a limitation. It listed the *cap on vector
width* the JVM does offer — `-XX:UseAVX=2` on x86-64, `-XX:UseSVE=0` on AArch64 —
explained that the entrypoints pinned those to stop the JIT reaching for wider
vectors than the compiled rows were allowed, and admitted the result was an
approximation: **the floor is pinned, the ceiling is not.**

That was the wrong conclusion, and the caps are gone.

The JVM was never failing to honour a rule. It was doing the thing it exists to do.
A JIT compiles *late* — after the program has started, on the silicon it is standing
on — and the specialisation an ahead-of-time compiler can only buy by giving up
portability, a JIT gets for nothing. That has been the headline argument for
just-in-time compilation since the 1990s. Capping it to make the C rows look fair was
not levelling the field: it was **deleting the JVM's actual advantage so that the
table would look symmetrical**, and then publishing a JVM slower than the one anybody
runs.

So the JIT rows declare `modes: [native]` — a fact, stated once, in the same manifest
field where an interpreter states that it compiles nothing. And the compiled rows are
built **both** ways, so the reader can see the same specialisation being bought
explicitly, and see what it costs in portability.

The honest comparison was never "everyone pinned" or "everyone native". It is: *here
is what each toolchain can be asked for, and here is what it did.*

**Except for one row.** GraalVM `native-image` compiles *ahead* of the run, so it
takes a real `-march` and is the only JVM backend that can be pinned. It comes with
its own wrinkle: on AArch64 native-image offers `armv8-a` and `armv8.1-a` and stops,
with no `armv8.2-a` to match the campaign's. The rule there is **never above the
campaign's baseline** — it takes the highest level it can express that does not exceed
what every other backend was held to, which is one below. The row is handicapped
rather than flattered, that is the safe direction to be wrong in, and it reports
`armv8.1-a` as its ISA so the reader can see it.

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

Thirty backends, two modes, an hour of wall-clock. One of them segfaults in
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
