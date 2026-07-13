---
title: Measurement protocol
order: 6
summary: docker build prepares, docker run measures — the three clocks, the cgroup, and the pinned memory budget.
---

**`docker build` prepares. `docker run` measures.** That is the core rule.

Almost all measurement originates inside the container. The CLI contributes the one
number nothing inside the container can produce: the **external wall-clock** — nothing
in there is alive to time its own creation.

## The build phase

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

## The three clocks

| Layer    | Source                    | Captures |
| -------- | ------------------------- | -------- |
| External | CLI, around `docker run`  | container create + runtime init + compute |
| Internal | program, `elapsed_ns`     | compute only |
| Floor    | a `/bin/true` image, 30×  | container overhead alone |

**External minus internal is a metric, not noise.** It is runtime startup cost,
and it is where the JVM and CPython pay their tax.

Do **not** subtract the floor from anything — that would propagate its variance
into every number. Characterize it once and publish it beside the table.

## CPU time comes from the cgroup, never from `rusage`

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

## Parallel efficiency is a median, not a minimum

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

## Memory is only comparable under a pinned budget

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

## The build column reports the internal clock, the run column the external one

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
