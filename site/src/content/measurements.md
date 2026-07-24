---
title: Measurements
summary: What every number a campaign records means — and, for each one, what it does not.
---

## Language

The language the kernel is written in — and the *least* interesting third of a
row's identity. Two rows in the same language can differ by an order of magnitude,
and the reason is never the language.

Keep that in mind before quoting a row as "language X vs language Y": what a row
actually identifies is the tuple *(compiler, interpreter, compiler version, flags,
target CPU)*. The language is along for the ride.

Every implementation **describes itself**, in its own words, below the table — what
it is, and whatever caveat the person who wrote it wanted you to have. Read that
before quoting its row.

## Compiler

What turns the source into instructions ahead of the run — `gcc`, `cython`,
`gc` — and the axis this project exists to compare.

`n/a` means nothing is compiled ahead of time. That is a property of the backend,
not a hole in the data.

## Interpreter

What executes the result — `cpython`, a JVM, nothing at all.

A backend can have **both** a compiler and an interpreter, and that is not a
curiosity: `python` / `cython` / `cpython` compiles the kernel to a native
extension module that CPython then loads and calls. Its row and the pure-CPython
row share a language and an interpreter, and differ only in the compiler. That is
the clean experiment — the one place in this table where a single column changes.

`n/a` means the backend ships machine code and runs it directly.

## Mode

**Which machine the code was compiled for.** Every mode is compiled with `-O3`, and
every mode uses strict IEEE 754 arithmetic — the axis here is the instruction set,
never "optimized vs not optimized", and never floating-point semantics.

- **`baseline`** — a pinned instruction set, the same for every backend on this
  architecture: `x86-64-v3`, or `armv8.2-a`. The binary does not depend on the CPU
  that compiled it. This is what you ship when the code has to run on a fleet.
- **`native`** — whatever *this* CPU offers. The compiler inspects the machine it is
  standing on and uses everything it finds, including instructions a CPU two years
  older would not understand.

The gap between the two is **what portability costs**.

### Why some backends have only one mode

Because the two modes are the two answers to *"which machine is this code for?"*, and
**not every toolchain can give both answers.** That is not a gap in the data. It is
the most interesting thing on this page.

An **ahead-of-time compiler** — gcc, clang, rustc, zig, Go, `native-image` — has to
choose. It emits machine code long before it knows where that code will run, so it
either targets a floor every CPU clears, or it targets the CPU in front of it. Both
are real builds, so you get both rows.

A **JIT** — HotSpot, OpenJ9, V8, JavaScriptCore, PyPy — does not choose. It compiles
the hot loop *while the program is running*, on the silicon underneath, so it takes
that machine's instruction set whether anyone asks it to or not. There is no flag to
pin it, because pinning it was never the point: this is the thing a JIT sells. Those
backends have a `native` row and no `baseline` row, and denying them the machine to
make the table look symmetrical would mean publishing a JVM slower than the one you
actually run.

Two backends are worth knowing about because they break the neat version of that
story:

- **Julia** is a JIT that *can* be pinned — it takes `--cpu-target` and validates the
  name. So it has both modes. The `baseline` column is not "the ahead-of-time
  compilers"; it is "the toolchains that let you pick the target", and Julia is the
  proof those are different sets.
- **CPython** is neither. It compiles nothing ahead of the run and has no JIT during
  it, so no machine code is ever generated for this CPU at all. Its hot loop is the
  interpreter's own `eval` loop, compiled by whoever packaged it. It sits in
  `baseline` — the column of "did not get the machine" — and its **ISA** column tells
  you the rest.

## ISA

**What the row actually got**, as against what its mode asked for.

The mode is a *request*. This column is the *answer*, reported by the container that
ran the compiler. Most of the time they agree and you can ignore this column. The
cases where they disagree are the reason it exists:

| ISA | What it means |
| --- | ------------- |
| `x86-64-v3`, `armv8.2-a` | The pinned baseline, honoured exactly. |
| `native` | This CPU's full instruction set. A compiled row asked for it; a JIT row could not have refused it. |
| `v3`, `v4`, `v8.2` | **Go.** It has no `-march=native`: `GOAMD64`/`GOARM64` name psABI *levels*, and Go never asks the CPU which one it is on. So the entrypoint asks, and names the highest level the machine actually clears. On an AVX2 machine that is `v3` — the same level as the baseline, the same binary — and the two rows tie. That tie is the finding, not a hole: Go has no instruction set left to reach for. |
| `armv8.1-a` | **`native-image` on AArch64.** GraalVM offers no `armv8.2-a`, so it takes the level below rather than one above the campaign's baseline. The row is handicapped rather than flattered. |
| `distro` | **CPython.** Its interpreter carries no `-march` at all: whoever packaged it built it for the toolchain's floor — plain `x86-64`, one level *below* the baseline every compiled row was held to. The ISA of this row was chosen by the packager, not by this campaign. |
| `n/a` | The backend did not report one. An absence, never a claim. |

Every one of those divergences used to live in a footnote inside a manifest — which
is to say, nowhere anybody reading a table would ever find it. A column whose meaning
varies from row to row is not a problem. A column whose meaning varies from row to row
*in silence* is, and this column is how it stops being silent.

### What `native` is not

It is **not** `-ffast-math`, and that distinction is what keeps every row of this
table comparable.

`-march=native` decides *which instructions* the compiler may emit. `-ffast-math`
decides *what arithmetic means* — it lets the compiler reassociate your sums, and a
compiler that reorders arithmetic is **changing the answer**, because floating-point
addition is not associative and every operation rounds.

Widening a vector reorders nothing. Four multiplications at once are still the same
four multiplications, rounded in the same places. So a `native` build and a `baseline`
build of the same source compute **the same bits** — and `-ffast-math` is spelled
nowhere in this project.

Which is why every row here, in both modes, computed the identical answer: `langbench`
verifies it on every sample and quarantines any backend that disagrees, so a row that
was wrong is not in this table — it is in the failures beneath it. See
[the strict-mode invariant](/methodology/#the-strict-mode-invariant).

## Runs

How many measured samples went into this row. Warmup rounds are written to
`samples.ndjson` and flagged there — never deleted — but they never reach these
numbers.

## Run min

The shortest wall-clock time across those samples. *Wall-clock* means the time a
stopwatch would show: the harness starts it before `docker run` and stops it after.
It therefore includes container creation, runtime startup and the computation.

**Why the minimum and not the average?** Because benchmark noise only goes one way.
A neighbouring process can steal CPU and make a run slower; nothing can make a run
faster than the machine is capable of. So the fastest sample is the one that was
least disturbed, and it is your best estimate of the machine's real capability. An
average would just be "the true value plus however much interference we happened to
collect".

## Dispersion

How much the samples of this row disagreed with each other, as a percentage.
Technically: the median absolute deviation, divided by the median. Robust by
construction — a single clobbered round does not move it.

**This is a verdict on the campaign, not an error bar on the result.** It does not
say "the true time is 8.1 ms ± 3%". It says "this machine was calm enough / too
noisy to be believed". Above roughly 2%, distrust even the minimum, and stop making
percentage-level claims.

It will not point you at an isolated spike either — being a median, it is designed
to ignore one. For spikes, read `samples.ndjson`.

Below three samples it prints `n/a (n=…)`: with two points the maths would return a
number, and that number would claim a precision the campaign never had.

## Compute min

The shortest time the *program itself* reported for its hot loop, measured inside
the container with a monotonic clock. It excludes everything that happened before
`main` started.

## Startup

Everything that happened before the real work began: container creation, runtime
initialisation, JIT warmup, interpreter boot.

It is the smallest `wall − compute` gap *within a single run*, not **Run min**
minus **Compute min**. Those two minima usually come from different rounds, so
subtracting them would describe a run that never took place — and the arithmetic
would not even close: on a noisy host the difference of the minima can exceed
every gap actually observed.

**This is a result, not overhead to subtract away.** It is where a JVM or a CPython
pays its entry fee, and it is a real cost in any program that does not run for an
hour. Comparing this column across runtimes is one of the more interesting things
this table offers.

## CPU time

`user + system` time, read from the container's own accounting file
(`/sys/fs/cgroup/cpu.stat` — the kernel's per-container bookkeeping), median over
the measured samples. It is summed over all threads, so on a parallel run it
legitimately exceeds the wall-clock: 4 threads busy for 1 second is 4 seconds of CPU
time.

Wall-clock tells you whether it was fast. CPU time tells you what it cost. A runtime
whose scheduler busy-waits (spinning in a loop while waiting instead of sleeping)
burns CPU without finishing a millisecond sooner — and the gap between these two
columns is the only place you will ever see it.

You do not have to divide this by anything: the **Cores** column beside it already
has.

## Cores

How many cores the row actually kept busy. The ratio is taken **within each
sample** — one run's CPU time over that same run's compute time — and the row
publishes the **median** of those per-sample ratios, read against the
{{ campaign.cpu }} threads the harness handed every kernel of this campaign.

This is the column that separates *this backend is slow* from *this backend cannot
use the machine*. Two rows with the same **Run min**, one at `7.8 / {{ campaign.cpu }}`
and one at `1.0 / {{ campaign.cpu }}`, are not two slow backends: one of them is a
global interpreter lock, and no amount of compiler work will ever move it. Wall-clock
alone hides that completely.

It is the one number in this table that is a **median** rather than a minimum. Every
timing here is a min-of-N because a busy machine can only ever make a run *slower*,
so the smallest sample is the best estimate of what the machine can do. Parallelism
is not one-sided like that — contention inflates the CPU clock (threads spinning) and
the compute clock alike — so no extreme recommends itself over the middle.

It can exceed {{ campaign.cpu }}, and that is a result rather than an overflow. The
numerator counts every microsecond of CPU the container burned; the denominator counts
only the span the kernel timed *itself* over. A JIT compiling on one thread while the
kernel computes on the others is spending CPU that the hot loop's clock never sees,
and a reader comparing a JVM to a static binary deserves to see it rather than have it
normalised away.

Two things the quotient is *not*. The denominator is the sample's **compute**
clock, never its **run** wall: the threads do not exist during container creation
and interpreter boot, but the run stopwatch is already running — divide by the wall
and a perfectly parallel backend reports fewer cores than it used, for no reason
other than that Docker took its time. And the ratio is taken **before** any
statistic, never between two of them: the row's median CPU time and its fastest
compute time come from different rounds, and their quotient — like a startup
computed from two minima — would describe a run that never happened.

## Memory

The peak memory of the **whole container**, from the cgroup's own high-water mark
(`memory.peak`) — minimum over the measured samples.

Not the resident set of one process, and deliberately not: it is the process tree, the
page cache it faulted in, and the tmpfs a build wrote into. The question this column
answers is what the backend needed in order to *run*, not what one of its processes
happened to be holding at the end.

The minimum is the right statistic here, and for once the argument is exact rather
than statistical: page cache and a lazy garbage collector can only ever push the
high-water mark *up*, never below what the backend genuinely had to allocate.

Every measured container ran under the **same pinned memory budget**, and that pin is
what makes the column comparable at all. A garbage-collected runtime sizes its default
heap from what its cgroup shows it — a JVM takes a quarter of it — so an unpinned
budget would have let the *bench machine's* RAM decide how much memory a JVM decided
it wanted, and this column would describe the host instead of the backend. It follows
that two campaigns run under different budgets do not compare here.

## Build min

The shortest **compile time** of a timed recompile, from a clean tmpfs (an
in-memory filesystem, so no disk cache advantage) and with no network access.

This is the compiler's own elapsed time, reported from *inside* the container —
the `docker run` wall-clock around it is discarded. Docker is here so that six
toolchains need not be installed on the host; the less it shows up in a number,
the better. Container creation costs on the order of a hundred milliseconds,
which is *several times* what gcc spends on a single-file kernel. Timing the wall
would charge every compiler the same large constant, flattering the slow ones and
compressing the very ratios this column exists to show.

There is no **Startup** analogue for this column, and that asymmetry is
deliberate: a *runtime's* startup is a property of the backend and therefore a
result, while a *container's* startup is an artefact of how we chose to isolate
the build.

Note the other subtlety: `docker build` is *preparation* and is never timed —
timing it would mostly measure Docker's layer cache. This column comes from a
`docker run` that recompiles the source inside the container and throws the
result away.

`n/a` means the backend has no build step at all.

## Build disp.

Same dispersion metric, over the build samples. Builds are slow, so we do fewer of
them, so this column reads `n/a (n=…)` more often.

## Source

The size, in bytes, of the one kernel file the backend's manifest declares.

It is a property of the **language**, not of the backend, and it is honest about that:
two rows that compile the same file report the same number. `c` / `gcc` and `c` /
`clang` are the same `mandelbrot.c`, so this column ties between them — which is
exactly what it should say about code somebody wrote once and compiled twice.

**Do not read it as a measure of quality, or of effort.** It is one author's kernel,
in one style, under this repository's rules — zero dependencies, one file, threads
handed in from `argv`. It says how much text this language needed to express *this*
workload under *those* constraints. It does not say a language is verbose, and it
certainly does not say how much work it was to write.

## Binary

The size of the shipped executable on disk.

**Do not rank implementations with this column.** It measures linking policy, not
code quality. gcc links libc dynamically and so looks tiny — but the code is still
there, in `libc.so`, just outside the file we measured. Rust links its standard
library statically. Go embeds an entire runtime plus type metadata. These are three
different packaging decisions, not three levels of compiler skill.

## `.text`

The size of the `.text` section — the machine code itself, and nothing else. This
*is* comparable across implementations.

Read it as the **price of the optimization**: inlining, loop unrolling and
vectorization all make the code bigger in exchange for making it faster. This column
is the bigger; **Run min** is the faster.

Calibrate your expectations: on a small kernel like this, `.text` is around a
kilobyte, and the linker pads function entry points to alignment boundaries. A
difference of a dozen instructions can therefore leave the number completely
unchanged. When it does not move, the answer is in the disassembly, not here.

**And never read it as a proxy for speed.** Cython emits 50.5 KiB of machine code
against C's 1.3 KiB — thirty-nine times more — and runs forty-two times slower. The
disassembly says why in one line: Cython's hot loop is 142 `bl` instructions into the
CPython C-API and a single `fadd`, where the C kernel has six `fadd`, five `fmul`,
three `fsub` and no call at all. More code, doing less arithmetic. `.text` is the
*cost* of an optimization, never its reward.

`n/a` means the backend emits no artifact to measure. That is a property of the
backend, not of the language — `native-image` produces a binary from the very same
Java source that a JIT-only run does not.
