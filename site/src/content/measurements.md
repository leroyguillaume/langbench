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

Which floating-point rules the compiler was allowed to play by. **Every mode is
compiled with `-O3`** — the axis here is FP semantics, never "optimized vs not
optimized".

Background, if `-ffast-math` has never bitten you yet: floating-point arithmetic is
not associative. `(a + b) + c` and `a + (b + c)` can give different results, because
each operation rounds. So a compiler that reorders your arithmetic is *changing the
answer*, slightly. The three modes are three answers to "may it?".

- **`strict`** — no. No reordering, no fusing (for gcc, literally
  `-ffp-contract=off`). Every operation rounds exactly where the source says it
  does.
- **`fma`** — fusing allowed. `a*b + c` becomes one *fused multiply-add*
  instruction, which rounds **once** instead of twice. Note the direction: this is
  usually **more** accurate than `strict`, not less. It is not a sloppy mode, it is
  a *less reproducible* one.
- **`fast`** — `-ffast-math`. Fusing plus reordering plus a pile of assumptions
  (no NaNs, no infinities, …). This is the mode where precision is genuinely traded
  for speed.

**Read `strict` first, and treat `fma` / `fast` as different experiments, not as
faster versions of the same one.**

Here is why `strict` carries the whole report. Our kernel only ever multiplies,
adds, subtracts and compares. The IEEE 754 standard specifies all four operations
down to the last bit, and both x86-64 (on SSE2 — not the ancient 80-bit x87 unit)
and AArch64 implement them faithfully. So once no compiler is allowed to fuse or
reorder, every compiler, every language and both CPU families are *obliged* to
produce the identical sequence of doubles, hence the identical iteration count for
every pixel, hence the identical checksum.

That obligation is what makes the timings comparable at all. A build that
reassociated the inner loop and a build that did not are no longer computing the
same thing, and there is no honest way to compare their run times. Tying every
compiler's hands the same way is the precondition for the whole table.

It is also why some backends only ever appear in `strict`: an interpreter has one
floating-point behaviour and no flag to change it, so an `fma` row would be the
exact same run under a different name.

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

How many cores the row actually kept busy — `CPU time / Compute min` — read against
the {{ campaign.cpu }} threads the harness handed every kernel of this campaign.

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

Note the denominator is **Compute min**, never **Run min**. The threads do not exist
during container creation and interpreter boot, but the stopwatch behind **Run min** is
already running — divide by that and a perfectly parallel backend reports far fewer
cores than it used, for no reason other than that Docker took its time.

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

`n/a` means the backend emits no artifact to measure. That is a property of the
backend, not of the language — `native-image` produces a binary from the very same
Java source that a JIT-only run does not.

## Δ strict

This run's checksum, minus the `strict` reference printed above the table.

On a `strict` row it is always `0`. That looks like a tautology, and it is not.
As the **Mode** section explains, strict semantics leave the compilers no room to
disagree — so the checksum stops being a numerical property of the computation and
becomes an **equality test between implementations**. It is the correctness gate for
the entire harness. A divergence takes that backend out of the campaign on the spot
— quarantined, no timing published, the reason recorded — and is never waved through
as "just rounding": it means two kernels are not computing the same thing. Something
was mistyped, a condition was flipped, a compiler flag leaked in. The other backends
keep measuring; the broken one has nothing left to say, and a campaign that threw
away forty good rows because one backend was wrong would be its own kind of bug.

A real example from this repo: rewriting `zr2 - zi2 + cr` as `cr + zr2 - zi2` in the
Python kernel — a reordering that looks like a harmless tidy-up — flips two pixels
out of twelve million and stops the run. That is precisely the class of bug that
unit tests do not catch, and it is why the gate exists.

It is a **necessary condition, not a sufficient one.** The gate can only notice a
perturbation big enough to push some pixel across an iteration boundary. So a
passing checksum means "no evidence of divergence at this resolution" — never
"provably identical". For Mandelbrot, sensitivity grows with the `grid_size` and
`max_iter` params, because both increase the number of pixels sitting right on a
boundary.

And the reference is not a universal constant. It is a property of *(workload, its
params)*, which is why each workload declares its own — in its `workload.yaml` — and
why overriding a param with `--param` retires it: it is the answer to the declared
work, not to the work you just asked for. The invariant is
*agreement between implementations*, never a particular number.

In `fma` and `fast` we do not gate — those modes are *allowed* to diverge, that is
their whole definition — so we report the delta instead. It is the precision you sold
for the speed you gained, sitting right next to the timing instead of buried in a
footnote. Read the two columns together, and mind the sign of the trade: an `fma` row
is typically *more* accurate than the strict reference it differs from. We do not
print the raw checksum of a relaxed row, because on its own it tells you nothing —
the distance is the whole story.
