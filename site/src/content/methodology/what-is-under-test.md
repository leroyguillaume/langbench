---
title: What is under test
order: 1
summary: Compiler and runtime backends, not languages — and the claims that follow.
---

**Compiler and runtime backends, not languages.**

The primary question is: *given the same source, how do different backends compare?*
gcc versus clang on identical C. rustc-LLVM versus rustc-cranelift on identical Rust.
CPython versus PyPy. OpenJDK versus GraalVM `native-image`.

The unit of comparison is therefore not a language but a tuple:

> (compiler, version, flags, target architecture)

Cross-language comparison is a secondary, much weaker result. See
[Two axes](#two-axes-two-tables-never-merged).

## Two axes, two tables, never merged

1. **Same source, different backend.** The real experiment. gcc versus clang on
   identical C; rustc-LLVM versus rustc-cranelift on identical Rust. Clean, and
   the reason this project exists.
2. **Same workload, different language.** Confounded by construction: different
   source, different runtime, different standard library. Valid for orders of
   magnitude ("Python is roughly 80× slower than Rust"), never for percentages.

## Claims we do not make

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
