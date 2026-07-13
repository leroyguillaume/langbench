---
title: Floating-point modes
order: 3
summary: Three FP semantics from one source, and the bit-identical checksum that gates them.
---

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

## The strict-mode invariant

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
