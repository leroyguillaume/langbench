---
title: The work
order: 2
summary: What a workload must be, and the anti-cheating contract every kernel obeys.
---

A **workload** is the work itself: what it is, how it is sized, and what the right
answer is. A workload is *not* an algorithm. Mandelbrot is one; a JSON parser, an HTTP
server, a cold start are others, and nothing in the harness assumes the work is a
computation over a grid.

What a *given* workload puts under the light — and, just as importantly, what it says
nothing about — is declared in its own `workload.yaml` and shown on its page. It is not
repeated here. This page is the contract that holds whatever the work is: the rules an
implementation obeys so that the number it produces is about the backend and not about
the way the benchmark was written.

## Zero third-party dependencies

**No third-party dependencies. None.** Rust uses `std::thread` and an `AtomicUsize`
chunk counter, not `rayon`. Otherwise the timed build compiles eight thousand lines of
rayon while gcc compiles fifty lines of C, and the build-time column means nothing. It
also removes every question about pre-building dependencies. Each implementation is a
single source file.

## Work is handed out dynamically

Where a workload's units of work cost different amounts, the load is **imbalanced**, and
imbalanced on purpose: in Mandelbrot the interior pixels run to the iteration ceiling
while the exterior ones exit after a few iterations. Chunking must therefore be dynamic
— at least `4 × threads` chunks handed out on demand. A static contiguous split
measures the split, not the backend.

## Anti-cheating contract

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
