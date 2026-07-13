---
title: Flags and architectures
order: 4
summary: A pinned baseline per architecture, why every toolchain spells it differently, and why timings never cross an architecture.
---

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

## Every toolchain spells the baseline differently, and some ignore it silently

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

## A toolchain that does not exist is not a slow toolchain

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
