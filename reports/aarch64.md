# langbench report

Generated on 2026-07-12T15:22:53.189921452+00:00 by langbench 0.1.0.

## What you are looking at

langbench compiles the *same* workload — the same maths, written once per
language — with several compilers, then runs each build many times under the same
conditions and records how long each run took. This report is the summary.

**The thing being compared is the compiler, not the language.** Two rows that use
the same source and differ only in which compiler built it are a clean experiment.
Two rows written in different languages are not: the source is different, the
standard library is different, the runtime is different. You can read "one is ten
times slower" from those; you cannot read "C is 4% faster than Rust".

Raw measurements live in `samples.ndjson` — one JSON line per run, written as the
run finishes. Every number below is recomputed from that file, so the file is the
truth and this report is just a view of it. If a number here looks odd, go read the
lines it came from.

New to this? Read the column reference at the bottom before the tables. It defines
every term, in order, and the tables are not much use without it.
[METHODOLOGY.md](../METHODOLOGY.md) explains *why* the protocol is what it is.

## Careful: this host is not a clean benchmark target

The harness inspected the machine before measuring and found this:

- Running under a hypervisor (DMI vendor is `Microsoft Corporation`). Timings measure the VM's vCPU scheduling as much as the backend, and the host may throttle or migrate the guest at any moment.

In plain terms: something else on this machine can steal CPU from a run and make it
look slower than it is. Small differences here are noise, not results. Compare
orders of magnitude ("this one is 20x slower"), never percentages ("this one is 3%
faster").

## Campaign

The knobs this run was launched with. Change any of them and the numbers below
change with them — including the reference checksum.

| Parameter | Value | What it is |
| --- | --- | --- |
| Threads (`--cpu`) | 4 | How many worker threads each kernel was told to use. The harness passes this explicitly; kernels never guess. |
| Grid | 2048 x 2048 | Size of the image computed, in pixels. This is the problem size. |
| Max iterations | 1000 | Work ceiling per pixel. Higher means more maths per pixel. |
| ISA baseline (`-march`) | armv8.2-a | The CPU instruction set every compiler was allowed to target. Pinned, so no compiler gets a private head start. |
| FP modes | strict, fma, fast | The floating-point rule sets used. See **Mode** below. |
| Run rounds | 10 (+ 1 warmup) | How many times each build was measured. Warmup runs are recorded but excluded. |
| Build rounds | 3 (+ 1 warmup) | Same, for the timed recompiles. |

## Machine

Where it ran. Timings only mean something *relative to this machine*.

| Property | Value |
| --- | --- |
| Hostname | runnervmrvz09 |
| Architecture | aarch64 |
| OS | Linux |
| Kernel | 6.17.0-1018-azure |
| Virtualization | DMI vendor is `Microsoft Corporation` |
| Harness containerized | false |
| CPU model | n/a |
| CPU vendor | 0x41 |
| Logical CPUs | 4 |
| Physical cores | n/a |
| available_parallelism | 4 |
| SMT active | false |
| NUMA nodes | 1 |
| architecture extensions | asimd, sve, sve2, bf16 |
| Scaling governor | n/a |
| Frequency min | n/a |
| Frequency max | n/a |
| Frequency at start | n/a |
| Turbo disabled | n/a |
| Isolated CPUs | n/a |
| Memory | 15.6 GiB |
| Load average at start | 0.77, 0.23, 0.08 |
| cgroup version | 2 |
| Docker version | 28.0.4 |
| Docker storage driver | overlay2 |

## Results

### mandelbrot

Reference checksum in `strict` mode: `1038538536`.

The checksum is a single 64-bit integer summarising the whole computed image. Every
`strict` row below produced exactly this number, bit for bit, whatever the compiler
or the language. That is the correctness gate, and it is enforced by exclusion: a
build that disagreed is not in this table at all. It was taken out of the campaign
where it broke, it published no timing, and the campaign recorded what it produced
instead of quietly averaging it in with the rest.

Rows are sorted **fastest first**, on **Run min**. Mind two things before reading the
order as a leaderboard: the sort mixes the FP modes together, and two rows in
different modes are different experiments (see **Mode**); and a gap smaller than
**Dispersion** is not a gap at all, it is noise that happened to land in that order.

| Language | Compiler | Interpreter | Mode | Runs | Run min | Dispersion | Compute min | Startup | CPU time | Cores | Memory | Build min | Build disp. | Source | Binary | `.text` | Δ strict |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [cpp](#mandelbrot-cpp-clang) | clang | n/a | fast | 10 | 819.3 ms | 0.47% | 582.7 ms | 236.5 ms | 2.34 s | 4.0 / 4 | 3.6 MiB | 1415.5 ms | 2.41% | 5.6 KiB | 73.1 KiB | 2.7 KiB | -2476 |
| [c](#mandelbrot-c-clang) | clang | n/a | fast | 10 | 826.2 ms | 0.70% | 581.9 ms | 243.8 ms | 2.34 s | 4.0 / 4 | 3.7 MiB | 79.5 ms | 0.43% | 5.4 KiB | 70.8 KiB | 1.5 KiB | -2476 |
| [cpp](#mandelbrot-cpp-gcc) | gcc | n/a | fast | 10 | 828.4 ms | 0.76% | 592.4 ms | 234.9 ms | 2.38 s | 4.0 / 4 | 3.7 MiB | 1129.1 ms | 0.39% | 5.6 KiB | 73.3 KiB | 2.7 KiB | +1756 |
| [c](#mandelbrot-c-gcc) | gcc | n/a | fast | 10 | 836.2 ms | 0.81% | 593.7 ms | 240.9 ms | 2.38 s | 4.0 / 4 | 3.7 MiB | 75.8 ms | 0.50% | 5.4 KiB | 70.7 KiB | 1.3 KiB | +1756 |
| [c](#mandelbrot-c-clang) | clang | n/a | fma | 10 | 839.3 ms | 0.54% | 603.4 ms | 235.9 ms | 2.42 s | 4.0 / 4 | 3.4 MiB | 82.6 ms | 50.50% | 5.4 KiB | 69.4 KiB | 1.4 KiB | -3574 |
| [cpp](#mandelbrot-cpp-clang) | clang | n/a | fma | 10 | 844.5 ms | 0.44% | 604.4 ms | 239.4 ms | 2.43 s | 4.0 / 4 | 3.7 MiB | 1438.7 ms | 3.44% | 5.6 KiB | 71.7 KiB | 2.7 KiB | -3574 |
| [cpp](#mandelbrot-cpp-gcc) | gcc | n/a | fma | 10 | 853.2 ms | 1.09% | 613.8 ms | 238.0 ms | 2.47 s | 4.0 / 4 | 3.6 MiB | 1137.0 ms | 0.31% | 5.6 KiB | 72.0 KiB | 2.7 KiB | -3574 |
| [c](#mandelbrot-c-gcc) | gcc | n/a | fma | 10 | 860.1 ms | 0.35% | 613.4 ms | 238.9 ms | 2.47 s | 4.0 / 4 | 3.7 MiB | 74.3 ms | 0.07% | 5.4 KiB | 69.3 KiB | 1.3 KiB | -3574 |
| [go](#mandelbrot-go-gc) | gc | n/a | strict | 10 | 900.3 ms | 0.68% | 657.3 ms | 241.9 ms | 2.64 s | 4.0 / 4 | 3.7 MiB | 3309.8 ms | 0.60% | 5.7 KiB | 2251.8 KiB | 581.8 KiB | 0 |
| [zig](#mandelbrot-zig-zig) | zig | n/a | strict | 10 | 903.8 ms | 0.50% | 671.5 ms | 232.0 ms | 2.69 s | 4.0 / 4 | 3.6 MiB | 1975.2 ms | 0.87% | 5.7 KiB | 1040.0 KiB | 21.8 KiB | 0 |
| [c](#mandelbrot-c-gcc) | gcc | n/a | strict | 10 | 907.8 ms | 0.28% | 673.9 ms | 232.6 ms | 2.71 s | 4.0 / 4 | 3.7 MiB | 74.5 ms | 0.52% | 5.4 KiB | 69.3 KiB | 1.3 KiB | 0 |
| [rust](#mandelbrot-rust-rustc) | rustc | n/a | strict | 10 | 910.9 ms | 0.89% | 671.7 ms | 238.3 ms | 2.69 s | 4.0 / 4 | 3.7 MiB | 446.9 ms | 0.03% | 5.2 KiB | 3890.6 KiB | 243.9 KiB | 0 |
| [cpp](#mandelbrot-cpp-clang) | clang | n/a | strict | 10 | 911.9 ms | 1.12% | 670.5 ms | 239.1 ms | 2.69 s | 4.0 / 4 | 3.7 MiB | 1430.2 ms | 0.73% | 5.6 KiB | 71.7 KiB | 2.7 KiB | 0 |
| [cpp](#mandelbrot-cpp-gcc) | gcc | n/a | strict | 10 | 915.0 ms | 0.29% | 673.5 ms | 239.6 ms | 2.71 s | 4.0 / 4 | 3.9 MiB | 1134.2 ms | 0.16% | 5.6 KiB | 72.0 KiB | 2.7 KiB | 0 |
| [c](#mandelbrot-c-clang) | clang | n/a | strict | 10 | 917.8 ms | 0.26% | 670.4 ms | 246.0 ms | 2.69 s | 4.0 / 4 | 3.6 MiB | 80.9 ms | 0.05% | 5.4 KiB | 69.4 KiB | 1.4 KiB | 0 |
| [scala](#mandelbrot-scala-scala-native) | scala-native | n/a | strict | 10 | 926.5 ms | 0.17% | 681.2 ms | 244.3 ms | 2.73 s | 4.0 / 4 | 6.6 MiB | 13835.9 ms | 10.40% | 5.9 KiB | 2501.7 KiB | 575.6 KiB | 0 |
| [java](#mandelbrot-java-javac-openjdk) | javac | openjdk | strict | 10 | 967.0 ms | 0.97% | 691.6 ms | 271.0 ms | 2.81 s | 4.0 / 4 | 12.3 MiB | 498.6 ms | 0.82% | 5.8 KiB | n/a | n/a | 0 |
| [ts](#mandelbrot-ts-bun) | n/a | bun | strict | 10 | 968.0 ms | 0.68% | 706.2 ms | 260.2 ms | 2.86 s | 4.0 / 4 | 33.0 MiB | 4.7 ms | 4.51% | 6.1 KiB | n/a | n/a | 0 |
| [js](#mandelbrot-js-nodejs) | n/a | nodejs | strict | 10 | 969.3 ms | 1.10% | 700.2 ms | 267.1 ms | 2.83 s | 4.0 / 4 | 50.6 MiB | 18.8 ms | 2.89% | 5.8 KiB | n/a | n/a | 0 |
| [js](#mandelbrot-js-bun) | n/a | bun | strict | 10 | 977.6 ms | 0.59% | 705.0 ms | 269.8 ms | 2.86 s | 4.0 / 4 | 32.8 MiB | 4.5 ms | 1.20% | 5.8 KiB | n/a | n/a | 0 |
| [kotlin](#mandelbrot-kotlin-kotlinc-openjdk) | kotlinc | openjdk | strict | 10 | 998.2 ms | 0.68% | 694.5 ms | 302.6 ms | 2.85 s | 4.1 / 4 | 26.6 MiB | 3736.2 ms | 0.42% | 4.8 KiB | n/a | n/a | 0 |
| [kotlin](#mandelbrot-kotlin-native-image) | native-image | n/a | strict | 10 | 1008.4 ms | 0.60% | 761.4 ms | 246.1 ms | 3.06 s | 4.0 / 4 | 3.6 MiB | 48446.0 ms | 0.59% | 4.8 KiB | 13276.1 KiB | 4879.0 KiB | 0 |
| [java](#mandelbrot-java-native-image) | native-image | n/a | strict | 10 | 1009.0 ms | 0.38% | 761.8 ms | 247.2 ms | 3.06 s | 4.0 / 4 | 3.7 MiB | 44592.9 ms | 0.70% | 5.8 KiB | 12696.4 KiB | 4846.2 KiB | 0 |
| [scala](#mandelbrot-scala-native-image) | native-image | n/a | strict | 10 | 1014.1 ms | 0.93% | 767.7 ms | 246.0 ms | 3.08 s | 4.0 / 4 | 3.7 MiB | 53352.3 ms | 0.61% | 5.9 KiB | 15332.2 KiB | 5252.7 KiB | 0 |
| [ts](#mandelbrot-ts-deno) | n/a | deno | strict | 10 | 1043.5 ms | 0.32% | 722.5 ms | 320.7 ms | 2.84 s | 3.9 / 4 | 60.9 MiB | 688.1 ms | 0.60% | 6.1 KiB | n/a | n/a | 0 |
| [js](#mandelbrot-js-deno) | n/a | deno | strict | 10 | 1044.9 ms | 0.52% | 723.3 ms | 321.5 ms | 2.85 s | 3.9 / 4 | 60.5 MiB | 100.6 ms | 1.17% | 5.8 KiB | n/a | n/a | 0 |
| [ts](#mandelbrot-ts-nodejs) | n/a | nodejs | strict | 10 | 1047.0 ms | 0.61% | 729.4 ms | 306.4 ms | 3.03 s | 4.1 / 4 | 115.4 MiB | 17.8 ms | 2.26% | 6.1 KiB | n/a | n/a | 0 |
| [java](#mandelbrot-java-javac-graalvm) | javac | graalvm | strict | 10 | 1080.1 ms | 0.49% | 767.4 ms | 309.4 ms | 3.19 s | 4.1 / 4 | 45.9 MiB | 509.6 ms | 0.25% | 5.8 KiB | n/a | n/a | 0 |
| [kotlin](#mandelbrot-kotlin-kotlinc-graalvm) | kotlinc | graalvm | strict | 10 | 1108.2 ms | 0.46% | 773.9 ms | 334.1 ms | 3.23 s | 4.2 / 4 | 46.4 MiB | 3742.1 ms | 0.20% | 4.8 KiB | n/a | n/a | 0 |
| [scala](#mandelbrot-scala-scalac-openjdk) | scalac | openjdk | strict | 10 | 1115.8 ms | 0.65% | 702.5 ms | 411.9 ms | 3.06 s | 4.3 / 4 | 35.0 MiB | 2683.8 ms | 1.37% | 5.9 KiB | n/a | n/a | 0 |
| [julia](#mandelbrot-julia-julia) | n/a | julia | strict | 10 | 1140.4 ms | 0.65% | 698.4 ms | 435.0 ms | 3.07 s | 4.4 / 4 | 79.3 MiB | 138.9 ms | 0.72% | 5.1 KiB | n/a | n/a | 0 |
| [scala](#mandelbrot-scala-scalac-graalvm) | scalac | graalvm | strict | 10 | 1208.5 ms | 0.44% | 776.1 ms | 429.1 ms | 3.45 s | 4.4 / 4 | 68.8 MiB | 2832.8 ms | 4.32% | 5.9 KiB | n/a | n/a | 0 |
| [python](#mandelbrot-python-pypy) | n/a | pypy | strict | 10 | 1565.5 ms | 0.52% | 1160.5 ms | 400.3 ms | 4.46 s | 3.8 / 4 | 369.8 MiB | 76.5 ms | 2.35% | 4.5 KiB | n/a | n/a | 0 |
| [java](#mandelbrot-java-javac-openj9) | javac | openj9 | strict | 10 | 1583.3 ms | 0.29% | 1242.5 ms | 329.7 ms | 5.14 s | 4.1 / 4 | 26.1 MiB | 609.9 ms | 0.70% | 5.8 KiB | n/a | n/a | 0 |
| [kotlin](#mandelbrot-kotlin-kotlinc-openj9) | kotlinc | openj9 | strict | 10 | 1619.2 ms | 0.43% | 1255.0 ms | 361.7 ms | 5.18 s | 4.1 / 4 | 28.9 MiB | 4133.0 ms | 0.16% | 4.8 KiB | n/a | n/a | 0 |
| [scala](#mandelbrot-scala-scalac-openj9) | scalac | openj9 | strict | 10 | 1664.3 ms | 0.38% | 1221.2 ms | 443.0 ms | 5.21 s | 4.2 / 4 | 33.2 MiB | 3082.4 ms | 0.98% | 5.9 KiB | n/a | n/a | 0 |
| [python](#mandelbrot-python-cpython) | n/a | cpython | strict | 10 | 30171.9 ms | 0.27% | 29788.7 ms | 379.1 ms | 115.67 s | 3.9 / 4 | 24.8 MiB | 33.5 ms | 1.39% | 4.5 KiB | n/a | n/a | 0 |
| [python](#mandelbrot-python-cython-cpython) | cython | cpython | strict | 10 | 47055.5 ms | 0.13% | 46748.0 ms | 295.9 ms | 181.67 s | 3.9 / 4 | 19.8 MiB | 2292.3 ms | 0.38% | 4.5 KiB | 147.4 KiB | 50.5 KiB | 0 |


## How to read a row in thirty seconds

1. Look at **Dispersion** first. If it is above roughly 2%, the machine was noisy
   and you may not compare rows by a few percent. Nothing else on the row can be
   more trustworthy than this column.
2. Compare **Run min** between two rows *in the same mode*. That is the headline
   number: how long the whole thing took, container startup included.
3. If two rows have similar **Run min** but very different **Startup**, you have
   found a runtime that pays a fixed tax before your code even begins — a JVM
   booting, a Python interpreter loading. That tax does not shrink when the problem
   gets bigger.
4. **Δ strict** must be `0` on every `strict` row. It always is, by construction —
   see the column reference.

## The columns

### Language

The language the kernel is written in — and the *least* interesting third of a
row's identity. Two rows in the same language can differ by an order of magnitude,
and the reason is never the language.

Keep that in mind before quoting a row as "language X vs language Y": what a row
actually identifies is the tuple *(compiler, interpreter, compiler version, flags,
target CPU)*. The language is along for the ride.

Every implementation **describes itself**, in its own words, below the table — what
it is, and whatever caveat the person who wrote it wanted you to have. Read that
before quoting its row.

### Compiler

What turns the source into instructions ahead of the run — `gcc`, `cython`,
`gc` — and the axis this project exists to compare.

`n/a` means nothing is compiled ahead of time. That is a property of the backend,
not a hole in the data.

### Interpreter

What executes the result — `cpython`, a JVM, nothing at all.

A backend can have **both** a compiler and an interpreter, and that is not a
curiosity: `python` / `cython` / `cpython` compiles the kernel to a native
extension module that CPython then loads and calls. Its row and the pure-CPython
row share a language and an interpreter, and differ only in the compiler. That is
the clean experiment — the one place in this table where a single column changes.

`n/a` means the backend ships machine code and runs it directly.

### Mode

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

### Runs

How many measured samples went into this row. Warmup rounds are written to
`samples.ndjson` and flagged there — never deleted — but they never reach these
numbers.

### Run min

The shortest wall-clock time across those samples. *Wall-clock* means the time a
stopwatch would show: the harness starts it before `docker run` and stops it after.
It therefore includes container creation, runtime startup and the computation.

**Why the minimum and not the average?** Because benchmark noise only goes one way.
A neighbouring process can steal CPU and make a run slower; nothing can make a run
faster than the machine is capable of. So the fastest sample is the one that was
least disturbed, and it is your best estimate of the machine's real capability. An
average would just be "the true value plus however much interference we happened to
collect".

### Dispersion

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

### Compute min

The shortest time the *program itself* reported for its hot loop, measured inside
the container with a monotonic clock. It excludes everything that happened before
`main` started.

### Startup

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

### CPU time

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

### Cores

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

### Memory

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

### Build min

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

### Build disp.

Same dispersion metric, over the build samples. Builds are slow, so we do fewer of
them, so this column reads `n/a (n=…)` more often.

### Source

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

### Binary

The size of the shipped executable on disk.

**Do not rank implementations with this column.** It measures linking policy, not
code quality. gcc links libc dynamically and so looks tiny — but the code is still
there, in `libc.so`, just outside the file we measured. Rust links its standard
library statically. Go embeds an entire runtime plus type metadata. These are three
different packaging decisions, not three levels of compiler skill.

### `.text`

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

### Δ strict

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
"provably identical". Sensitivity grows with `--grid-size` and `--max-iter`, because
both increase the number of pixels sitting right on a boundary.

And the reference is not a universal constant. It is a property of *(workload, grid
size, iteration ceiling, viewport)*, which is why each workload has its own, and why
the value moves when you change the campaign's parameters. The invariant is
*agreement between implementations*, never a particular number.

In `fma` and `fast` we do not gate — those modes are *allowed* to diverge, that is
their whole definition — so we report the delta instead. It is the precision you sold
for the speed you gained, sitting right next to the timing instead of buried in a
footnote. Read the two columns together, and mind the sign of the trade: an `fma` row
is typically *more* accurate than the strict reference it differs from. We do not
print the raw checksum of a relaxed row, because on its own it tells you nothing —
the distance is the whole story.

## What this report does not say

- **Which language is fastest.** It compares compilers and runtimes. Rows that
  differ in language are confounded by construction — different source, different
  runtime, different standard library — and are only meaningful for orders of
  magnitude.
- **How this machine compares to a machine with a different CPU family.** Absolute
  timings do not travel across instruction sets; only within-ISA ratios do.
- **That a small difference is real**, unless **Dispersion** says it can be. When in
  doubt, run more rounds.

## Backends

What each row of the tables above *is*, in the words of its own `bench.yaml` — the
same file the harness read to build it. Every **Language** cell links here.

A backend appears once, however many rows it has: the FP modes are three
experiments on the same thing, and what follows describes the thing.

### mandelbrot-c-clang

*mandelbrot* — language `c`, compiler `clang`, interpreter `n/a`.

The same mandelbrot.c as c-gcc, byte for byte, compiled by clang at -O3 against the same ISA baseline, on the same Debian, with the same flags. One source, two code generators: whatever separates these two rows is LLVM against GCC.

> Read this row only against c-gcc, and read the difference as codegen -- the distribution, the libc and the linker are held constant precisely so that it can be. Like gcc it links libc dynamically, so compare `.text`, never Binary.

### mandelbrot-c-gcc

*mandelbrot* — language `c`, compiler `gcc`, interpreter `n/a`.

The reference kernel. Hand-written C, compiled by gcc at -O3 against a pinned ISA baseline, with std threads and an atomic work queue. Every other row in this table is read against this one.

> gcc links libc dynamically, so the Binary column looks small: the code is still there, in libc.so, outside the file we measured. Compare `.text`, never Binary.

### mandelbrot-cpp-clang

*mandelbrot* — language `cpp`, compiler `clang`, interpreter `n/a`.

The same mandelbrot.cpp as cpp-gcc, byte for byte, compiled by clang++ at -O3 against the same ISA baseline and -- this is the part that matters -- the same libstdc++. One source, one standard library, two code generators.

> Debian's clang++ links libstdc++, not libc++, and that is deliberate: an STL swap would be a second variable, and this row is here to isolate the first. Read it against cpp-gcc, and read c-clang against c-gcc for the same reason.

### mandelbrot-cpp-gcc

*mandelbrot* — language `cpp`, compiler `gcc`, interpreter `n/a`.

The C kernel written the way a C++ programmer would write it -- std::thread, std::atomic, std::vector, a lambda for the worker -- and compiled by the same GCC at -O3 on the same base image. The arithmetic is identical; the abstractions are not.

> `cpp` and not `c++`: the identity becomes an image tag, and a Docker tag admits no `+`. Read this row against c-gcc to price the abstractions, and against cpp-clang to price the code generator.

### mandelbrot-go-gc

*mandelbrot* — language `go`, compiler `gc`, interpreter `n/a`.

The reference kernel in Go, compiled by gc against a pinned ISA baseline, with goroutines and an atomic row cursor. GOMAXPROCS is set from argv, never from the machine: Go 1.25 reads the cgroup quota by itself, and this benchmark refuses to ask that question.

> Written the natural way, this kernel returns 33209560 where the rest of the table returns 33209574 -- gc fuses the multiply-adds, and the arithmetic quietly stops being everyone else's. The float64() conversions in the source are rounding points, not casts, and they are load-bearing. Go also links a GC and a scheduler into every binary, which inflates Binary: compare `.text`.

### mandelbrot-java-javac-graalvm

*mandelbrot* — language `java`, compiler `javac`, interpreter `graalvm`.

The same Mandelbrot.java as java-javac-openjdk, byte for byte, compiled by the same javac and run on the same HotSpot -- with Graal replacing C2 as the JIT. Same bytecode, same VM, same GC: the code generator is the only variable.

> The cleanest A/B on a JIT this table can construct, and the reason the entrypoint passes -XX:+UseJVMCICompiler explicitly rather than trusting a default: a silent fall back to C2 would make this row java-javac-openjdk under another name, and publish noise as a finding. Do not expect Graal to beat C2 here by default -- on a tight numeric loop the two land close together, and this row is worth having for the comparison rather than for a winner. Like every HotSpot row, it has no true -march; the entrypoint caps vector width instead.

### mandelbrot-java-javac-openj9

*mandelbrot* — language `java`, compiler `javac`, interpreter `openj9`.

The same Mandelbrot.java as java-javac-openjdk, byte for byte, compiled by the same javac and executed by Eclipse OpenJ9 instead of HotSpot. A different JIT and a different GC underneath identical bytecode: whatever separates the two rows is the virtual machine.

> OpenJ9 is reputed to start faster than HotSpot, and on this workload it does not: measured here it starts slower. Reputations are why the harness exists -- but note this benchmark boots a JVM to run one long loop, which is the case OpenJ9's startup work is least aimed at, and the container floor is large. Take it as a measurement of this workload, not a verdict on the VM. This row also has NO ISA baseline, and the gap is published rather than papered over. OpenJ9 silently ignores unknown -XX: options -- `-XX:CompleteNonsenseFlag=42` starts it happily, where HotSpot refuses to boot -- so passing the HotSpot vector caps would have pinned nothing while claiming to. Its JIT may therefore use wider vectors than the compiled rows were allowed. No -Xquickstart, no -Xshareclasses: OpenJ9's startup knobs would flatter exactly the column this row is here to report.

### mandelbrot-java-javac-openjdk

*mandelbrot* — language `java`, compiler `javac`, interpreter `openjdk`.

The reference kernel in Java, compiled by javac and run on HotSpot with platform threads and an AtomicInteger work queue. No Maven, no Gradle, no jar: one file, one javac invocation, so the Build column times the compiler and nothing else.

> HotSpot has no -march: C2 compiles for whatever CPU it finds, which is the `native` targeting this project forbids everywhere else. The closest the JVM offers is a cap on vector width (-XX:UseAVX / -XX:UseSVE) and that is what the entrypoint pins -- an approximation, published as one. The JVM's startup is the largest in the table, and it is a result, not overhead to subtract.

### mandelbrot-java-native-image

*mandelbrot* — language `java`, compiler `native-image`, interpreter `n/a`.

The same Mandelbrot.java as java-javac-openjdk, byte for byte, compiled ahead of time by GraalVM native-image into a standalone ELF. No JVM at run time, no JIT, no class loading -- the program starts like a C binary because it is one.

> Read it against java-javac-openjdk, and read the Compute column, not Startup: an AOT binary arrives already compiled, while a JIT is still warming up inside the measured region, and that is where the gap lives. Startup barely moves, because container creation dominates that column for every fast backend alike. What AOT costs instead is the Build column -- tens of seconds of whole-program analysis. It is not free, it is prepaid. It is also the only JVM row with a true -march, though on AArch64 native-image offers no armv8.2-a, so it takes armv8.1-a: one level BELOW the campaign's baseline, never above. Binary is large because the runtime is linked in rather than installed; compare `.text`.

### mandelbrot-js-bun

*mandelbrot* — language `js`, compiler `n/a`, interpreter `bun`.

The same mandelbrot.mjs as js-nodejs and js-deno, byte for byte, executed by Bun on JavaScriptCore. Node and Deno share V8; this is the only JavaScript row with a different engine underneath, which makes it the only one where a gap is about the compiler rather than the runtime around it.

> The Build column is `bun build` -- a transpile and bundle -- where Node reports a parse and Deno a type check. Three runtimes, three different notions of "before the run": read each Build number against its own runtime, never across the rows.

### mandelbrot-js-deno

*mandelbrot* — language `js`, compiler `n/a`, interpreter `deno`.

The same mandelbrot.mjs as js-nodejs, byte for byte, executed by Deno -- which runs the same V8. Any gap between these two rows is the runtime around the engine: module resolution, the permission checks, and how quickly it can boot a worker isolate.

> The Build column here is `deno check`, a full type check, where js-nodejs reports `node --check`, a parse. Strictly more work, and not a like-for-like comparison -- read the Build column against Deno itself. DENO_DIR ships warm with the node typings, because a measured run has no network to fetch them from.

### mandelbrot-js-nodejs

*mandelbrot* — language `js`, compiler `n/a`, interpreter `nodejs`.

The kernel as written, executed by Node.js on V8. Parallelism comes from node:worker_threads -- one isolate per worker -- with the row queue in a SharedArrayBuffer and Atomics.add handing rows out, which is as close to the C kernel's atomic cursor as the language gets.

> The same mandelbrot.mjs runs under js-deno and js-bun, byte for byte, so those three rows measure runtimes rather than programs. The Build column is `node --check` -- parse and compile, no execution -- and it is not the same operation as Deno's type check or Bun's bundle: read each against its own runtime, never across the three.

### mandelbrot-julia-julia

*mandelbrot* — language `julia`, compiler `n/a`, interpreter `julia`.

The reference kernel in Julia, JIT-compiled through LLVM on first call, with Threads.@spawn tasks pulling rows off an atomic cursor. The ISA baseline is passed explicitly, because Julia's default is to compile for the host CPU -- the one thing this project never allows.

> The JIT's compile time sits inside the measured run, because that is where it happens: a script's native code is cached nowhere, so every run pays for it again. Julia is the only backend whose Build and Run columns measure overlapping work, and that is the honest shape of a JIT, not a flaw in the protocol.

### mandelbrot-kotlin-kotlinc-graalvm

*mandelbrot* — language `kotlin`, compiler `kotlinc`, interpreter `graalvm`.

The same Mandelbrot.kt as kotlin-kotlinc-openjdk, byte for byte, compiled by the same kotlinc -- with Graal replacing C2 as the JIT. One cell of the (language x JVM) grid.

> Exists to test an assumption rather than to break one: by the time a JIT sees this kernel it is the same loop whoever emitted the bytecode, so Kotlin-on-Graal should differ from Java-on-Graal by nothing at all. If it does, the two axes are not orthogonal and the whole grid needs rereading. That is worth one image.

### mandelbrot-kotlin-kotlinc-openj9

*mandelbrot* — language `kotlin`, compiler `kotlinc`, interpreter `openj9`.

The same Mandelbrot.kt as kotlin-kotlinc-openjdk, byte for byte, compiled by the same kotlinc and executed by Eclipse OpenJ9 instead of HotSpot.

> Like every OpenJ9 row, it has NO ISA baseline: OpenJ9 silently ignores unknown -XX: options, so the HotSpot vector caps would have pinned nothing while claiming to. The gap is published rather than papered over.

### mandelbrot-kotlin-kotlinc-openjdk

*mandelbrot* — language `kotlin`, compiler `kotlinc`, interpreter `openjdk`.

The kernel in Kotlin, compiled by kotlinc to JVM bytecode and run on the same HotSpot as java-javac-openjdk. Same platform threads, same AtomicInteger queue, same arithmetic -- the language in front of the JIT is the only thing that changes.

> Read the Run column against java-javac-openjdk and expect them to land close together: the two compile to bytecode that the same JIT turns into the same machine code. The Build column is where they part, and hard -- kotlinc is the slowest compiler in this table, several times javac on the same one-file kernel -- while the Startup column carries the Kotlin stdlib the JVM must load. That is the honest cost of Kotlin here: compile time and startup, not compute.

### mandelbrot-kotlin-native-image

*mandelbrot* — language `kotlin`, compiler `native-image`, interpreter `n/a`.

The same Mandelbrot.kt as every other Kotlin row, compiled ahead of time by GraalVM native-image into a standalone ELF -- with the Kotlin standard library swallowed whole and linked in. kotlinc runs first, as a step in this compiler's pipeline, exactly as javac does for java-native-image.

> This is where AOT stops being a rerun: compute ties with java-native-image, because it is the same loop, but Build and Binary do not -- native-image has to analyse and link the Kotlin runtime, and what that costs is a fact no JIT row can report. On AArch64 it takes armv8.1-a: one level BELOW the campaign baseline, never above.

### mandelbrot-python-cpython

*mandelbrot* — language `python`, compiler `n/a`, interpreter `cpython`.

The kernel as written, executed by the CPython interpreter. Parallelism comes from multiprocessing with the fork start method, because the GIL serialises CPU-bound bytecode -- threading here would measure the GIL, not the machine.

> Forking the pool is inside the timer on purpose: what a parallel runtime costs to start is part of what that runtime costs.

### mandelbrot-python-cython-cpython

*mandelbrot* — language `python`, compiler `cython`, interpreter `cpython`.

The same mandelbrot.py as python-cpython, byte for byte, compiled by Cython to a C extension module instead of interpreted. Cython is the compiler here, exactly as gcc is for the C kernel -- not a library the kernel imports.

> It is slower than the interpreter it compiles, and that is a result, not a bug. Without type annotations the generated C manipulates PyFloat objects through the C-API -- its hot loop holds 142 call instructions -- while CPython's specializing interpreter takes a fast path for float arithmetic.

### mandelbrot-python-pypy

*mandelbrot* — language `python`, compiler `n/a`, interpreter `pypy`.

The same mandelbrot.py as python-cpython, byte for byte, executed by PyPy. The source is unchanged, the multiprocessing pool is unchanged; the difference is a tracing JIT that turns the hot loop into machine code while it runs.

> The JIT's compile time is inside the measured run, because that is where it happens -- there is no warm-up phase to hide it in, and hiding it would be a different benchmark. Read this row against python-cpython (same source, no JIT) and python-cython (same source, compiled ahead of time, and slower than both).

### mandelbrot-rust-rustc

*mandelbrot* — language `rust`, compiler `rustc`, interpreter `n/a`.

The reference kernel in Rust, compiled by rustc at -O3 against a pinned ISA baseline, with std::thread scoped threads and an AtomicU32 work queue. No cargo, no rayon, no dependency: one file, one rustc invocation.

> rustc only *warns* when it does not recognise an ISA baseline, then hands back a generic binary -- so the entrypoint translates the harness's gcc-spelled -march into LLVM's spelling and fails outright on anything it does not know. It also links std statically, which inflates Binary against the C rows: compare `.text`.

### mandelbrot-scala-native-image

*mandelbrot* — language `scala`, compiler `native-image`, interpreter `n/a`.

The same Mandelbrot.scala as every other Scala row, compiled ahead of time by GraalVM native-image into a standalone ELF -- with the Scala runtime linked in. scalac runs first, as a step in this compiler's pipeline.

> Scala brings the heaviest runtime in this table -- the 3.x library plus the 2.13 one it still stands on -- and this row is what it costs to AOT-compile all of it and then ship it: watch Build and Binary, not compute. Not to be confused with scala-scala-native, which is Scala Native (LLVM), a different compiler entirely.

### mandelbrot-scala-scala-native

*mandelbrot* — language `scala`, compiler `scala-native`, interpreter `n/a`.

The same Mandelbrot.scala as scala-scalac-openjdk, byte for byte, compiled by Scala Native through LLVM into a standalone ELF. No JVM, no bytecode, no JIT -- and no edit to the kernel, because Scala Native's javalib supplies the very java.lang.Thread and AtomicInteger the JVM rows use.

> The cleanest compiler swap in the table: one source, and the choice is a JVM or LLVM. It is also the only Scala row that can honour the ISA rule exactly -- clang takes the harness's -march verbatim, where every JVM row can only approximate it. Do not confuse it with scala-native-image, which is GraalVM AOT-compiling JVM bytecode: a different compiler, a different runtime, and a confusingly similar name.

### mandelbrot-scala-scalac-graalvm

*mandelbrot* — language `scala`, compiler `scalac`, interpreter `graalvm`.

The same Mandelbrot.scala as scala-scalac-openjdk, byte for byte, compiled by the same scalac -- with Graal replacing C2 as the JIT. One cell of the (language x JVM) grid.

> Read it against java-javac-graalvm and scala-scalac-openjdk: if the language and the JIT are orthogonal, as the identical bytecode suggests they must be, this row is predictable from those two and confirms it. A row that confirms is cheaper than an assumption that is never checked.

### mandelbrot-scala-scalac-openj9

*mandelbrot* — language `scala`, compiler `scalac`, interpreter `openj9`.

The same Mandelbrot.scala as scala-scalac-openjdk, byte for byte, compiled by the same scalac and executed by Eclipse OpenJ9 instead of HotSpot.

> Like every OpenJ9 row, it has NO ISA baseline: OpenJ9 silently ignores unknown -XX: options. The gap is published rather than papered over.

### mandelbrot-scala-scalac-openjdk

*mandelbrot* — language `scala`, compiler `scalac`, interpreter `openjdk`.

The kernel in Scala 3, compiled by scalac to JVM bytecode and run on the same HotSpot as java-javac-openjdk and kotlin-kotlinc-openjdk. A plain while loop, deliberately: the algorithm is the contract, and a foldLeft over a lazy Iterator would measure the collections library instead of the backend.

> Three languages, one JIT: read the Run column against the java and kotlin rows and expect them to land close together, because all three hand C2 the same loop. Where Scala pays is the Build column -- scalac takes several times javac's time on this one file -- and the Startup column, which carries both the Scala 3 library and the 2.13 one it still stands on.

### mandelbrot-ts-bun

*mandelbrot* — language `ts`, compiler `n/a`, interpreter `bun`.

The same mandelbrot.mts as ts-nodejs and ts-deno, byte for byte, executed by Bun on JavaScriptCore. The only TypeScript row not running on V8, which makes it the only one where a gap in compute time is about the engine.

> Read it against js-bun to price the types (nothing, at run time) and against ts-nodejs and ts-deno to price the engine. The Build column is `bun build`, a transpile and bundle: not the parse Node reports, and not the type check Deno reports.

### mandelbrot-ts-deno

*mandelbrot* — language `ts`, compiler `n/a`, interpreter `deno`.

The same mandelbrot.mts as ts-nodejs, byte for byte, executed by Deno -- the runtime that has run TypeScript natively since its first release, on the same V8 Node uses. If the head start shows up anywhere, it shows up here.

> The Build column is `deno check`, a real type check, where ts-nodejs reports a parse that erases the types without reading them. That is the honest difference between the two: Deno is doing strictly more work, and only Deno's number can tell you what type-checking this kernel costs.

### mandelbrot-ts-nodejs

*mandelbrot* — language `ts`, compiler `n/a`, interpreter `nodejs`.

The kernel in TypeScript, executed by Node 24, which strips the types and runs the JavaScript underneath. Same worker_threads, same SharedArrayBuffer row queue, same arithmetic as js-nodejs -- the annotations are the only difference.

> Read this row against js-nodejs and expect the compute time to be identical: type erasure costs nothing at run time, by construction. What it can cost is startup, because the types must be stripped before the first line executes -- and every worker isolate strips them again.

### mandelbrot-zig-zig

*mandelbrot* — language `zig`, compiler `zig`, interpreter `n/a`.

The reference kernel in Zig, compiled at -OReleaseFast against a pinned ISA baseline, with std.Thread and an atomic work queue. No libc: the binary is static, and its threads are raw clone syscalls rather than pthreads.

> Pinned to Zig 0.15.2. The 0.16 standard library rewrote the I/O and process APIs this kernel calls, and a benchmark that has to be rewritten for every release is measuring the release notes. The static, libc-free binary makes Binary look large next to the dynamically linked C rows: compare `.text`.
