---
title: What we record
order: 7
summary: Binary size, source size, raw samples — and why a backend that fails is not a campaign that fails.
---

## Binary size

Three numbers, all cheap, recorded once per implementation in the `build` record
since they are constant across repetitions. We measure the binary the image
ships — the one `run` executes — not the throwaway from the timed rebuild.

- `binary_bytes` — the file on disk, as shipped.
- `binary_stripped_bytes` — after `strip`. We do not strip during the timed
  build; that would add link-time work to a number we are timing.
- `text_bytes` — the `.text` section, from `size(1)`.

**Only `.text` is comparable across implementations.** Total file size measures
linking policy, not codegen: gcc dynamically links libc and looks tiny while the
code lives in `libc.so`; Rust statically links its stdlib; Go embeds a runtime
and its type metadata. Ranking languages by file size ranks their packaging.

`.text` is exactly the emitted code, and it is the **cost side of the
optimization trade**: inlining, unrolling and vectorization all inflate it in
exchange for speed. Plotting `.text` against runtime is the point of a compiler
benchmark, not a curiosity — it is also where `-O2` versus `-O3` shows its hand.

**But calibrate your expectations.** On a kernel this small, `.text` is around a
kilobyte and function-entry alignment padding quantises it. Measured on C/gcc,
`fma` mode emits three fewer floating-point instructions than `strict` — twelve
bytes — and `.text` does not move at all. The column earns its keep on larger
kernels, or when a backend vectorizes and another does not. For anything finer,
read the disassembly; that is what we archive it for.

**And never read `.text` as a proxy for speed.** Cython emits 50.5 KiB of machine
code against C's 1.3 KiB — thirty-nine times more — and runs forty-two times
slower. The disassembly says why in one line: Cython's hot loop is 142 `bl`
instructions into the CPython C-API and a single `fadd`, where the C kernel has
six `fadd`, five `fmul`, three `fsub` and no call at all. More code, doing less
arithmetic. `.text` is the *cost* of an optimization, never its reward.

Interpreted and JIT backends emit no artifact: the field is `null`, not zero.
`native-image` does produce one, so "compiled" is a property of the backend, not
of the language.

We archive `objdump -d` of the hot loop alongside the results. Three lines of
Dockerfile. When clang comes out 3× ahead of gcc we do not speculate about the
vectorizer — we look for the `vmulpd`.

## Source size, and what it is not

`source_bytes` is the size of the one kernel file the manifest declares. The manifest
declares it rather than the harness guessing it: the alternative is to pattern-match
the filenames sitting beside the Dockerfile, which is parsing the path under another
name, and [the path is not metadata](../declaring-the-work/#the-path-is-not-metadata).

**It is a property of the language, not of the backend, and it is honest about that.**
`c` / `gcc` and `c` / `clang` compile the same `mandelbrot.c`, so they report the same
number and the head-to-head calls it a tie. That is not a weakness of the column; it is
the column telling the truth about the one axis this project exists to measure. Every
other number here separates two compilers on identical source. This one cannot, and
says so.

**It is not a measure of quality, and it is not a measure of effort.** It is one
author's kernel, in one style, under this repository's rules: zero dependencies, one
file, threads handed in from `argv`, a checksum printed. It says how much text a
language needed to express *this* workload under *those* constraints. It does not say
a language is verbose, and it emphatically does not say how much work it was to write.

That last distinction is why the column is **bytes and not tokens**. Counting tokens
would answer a question nobody here asked — the tokens in a finished file are not the
tokens it cost to produce one, which is prompt plus reasoning plus every attempt that
failed the checksum — and it would make the number depend on some vendor's tokenizer,
so that a figure in this repository could change because somebody else shipped a model.
Bytes are stable, vendor-neutral, and will mean the same thing in ten years.

## Sampling

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
- **Keep the warmup samples, flagged.** The first run of an image faults its
  layers into page cache. Mark them in the data rather than deleting them; the
  day something looks wrong, you will want to see them.
- **Verify the checksum on every run**, not once. A run with the wrong value is
  not a slow run, it is a wrong run, and it must never enter the statistics.
- **Store raw samples, never aggregates.** One NDJSON line per run, with the
  machine metadata and the campaign's parameters in a header record. Aggregates
  are recomputed when the samples are rendered; a discarded sample is gone forever.
  This is the highest-return rule in the protocol, and the one most regretted later.
- **Every sample carries its backend's manifest**: language, compiler,
  interpreter, description, comments, copied onto each line. It is deliberate
  repetition. A sample has to say what produced it *without a second file to join
  against*: the manifest will be edited, the directory will be renamed, the
  backend will be deleted — and the samples must still describe the campaign that
  actually ran. A foreign key into a file that changes underneath is not a
  record, it is a dangling pointer.

## A backend that fails is not a campaign that fails

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
