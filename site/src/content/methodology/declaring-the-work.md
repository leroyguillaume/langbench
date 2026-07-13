---
title: Declaring the work
order: 5
summary: Why there are two manifests, why the path is not metadata, and why the harness reads nothing else.
---

Two manifests, because there are two things to declare, and they are not the same
thing. A **workload** is the work. An **implementation** is a backend that does it.

A workload declares what the work is, how it is sized, what the right answer is, and
which directories implement it:

```yaml
# benchmarks/mandelbrot/workload.yaml
id: mandelbrot
params:
  - name: grid_size
    value: 2048
  - name: max_iter
    value: 1000
checksum: 1038538536
implementations:
  - python-cython
  - c-gcc
```

An implementation declares a backend, in a `bench.yaml` beside its Dockerfile:

```yaml
# benchmarks/mandelbrot/python-cython/bench.yaml
language: python
compiler: cython
interpreter: cpython
source: mandelbrot.py
modes:
  - strict
description: >-
  The same mandelbrot.py as python-cpython, byte for byte, compiled by Cython to
  a C extension module instead of interpreted.
comments: >-
  It shares a language and an interpreter with python-cpython and differs only in
  the compiler.
```

**The manifests are the only thing the harness reads.** It walks the tree for
`workload.yaml` files — that is the one search left — and after that it reads
declarations, never the tree: the workload lists the directories it is implemented
in, and each implementation names its own source file. Everything else about a
directory is inert.

## The path is not metadata

An earlier design inferred the language and the compiler from the directory name
(`benchmarks/<workload>/<language>-<compiler>/`) and read the rest back out of Docker
labels. Both are gone, for the same reason: they encode facts in places that
cannot hold them.

A path is a two-field record with no room for a third. `python-cython` is a
directory name that *cannot say* that CPython also runs the result — and that
omission is not cosmetic, because the whole value of that row is that it shares a
language **and an interpreter** with `python-cpython` and differs only in the
compiler. A naming convention had no slot for the fact that makes the experiment
clean.

So the tree is free-form. Move a directory, nest it, rename it: the campaign is
unchanged, because nothing reads it. What decides that a directory is measured is
that a workload **lists** it — not that it happens to sit somewhere. Recursing from
the workload and taking every `bench.yaml` underneath would have made the *position*
of a directory load-bearing again, which is the path being metadata under another
name.

The cost of declaring is that a manifest can be forgotten: a `bench.yaml` on disk
that no workload lists would never be built, never measured, and never missed. So
`langbench validate` walks the tree, compares it against every workload's list, and
fails on anything unclaimed. A row that is absent from a table reads exactly like a
backend nobody wrote, and that is the one thing a benchmark must never let a reader
believe.

## The answer belongs to the work

`checksum` is a property of `(workload, params)` — of the work and how it is
sized — so it lives with the workload, and not in the harness, and not in a
backend.

Without it, a campaign can only check that its backends agree *with each other*. That
is weaker than it sounds: it passes a campaign where every backend is wrong the same
way, and it makes no claim at all across two campaigns, because each one's reference
is simply whichever backend happened to run first. Declared, the answer outlives any
run, and a backend that disagrees with it is quarantined on the spot — not slow,
wrong.

Override a param and the declared reference stops applying, because it is the answer
to different work. The campaign says so, records no reference in its header, and falls
back to the weaker check. That is why the numbers this repository publishes come from
the declared params, and only from those.

## Identity is what a backend *is*

An implementation is `(workload, language, compiler, interpreter)`. There is no name,
because a name is a second thing to keep in sync with the first. Two manifests
declaring the same tuple are one implementation declared twice, and the campaign
refuses to run — they would build the same image tag and collapse into a single
row, and which of the two descriptions was published would be a coin toss.

Either half of the backend may be absent, and the absence is a fact worth
publishing: gcc compiles and nothing interprets; CPython interprets and nothing
compiles ahead of the run; Cython does both. The table prints all three columns,
`n/a` included.

## Labels are provenance, never input

Docker labels stay on the images — `langbench.version`, `langbench.flags` — but
the harness does not read them. They describe the artifact for whoever runs
`docker inspect` on it. Anything the harness *acts* on lives in the manifest,
because two sources of truth are one source of truth and one source of drift.

There is a hard reason as well as an aesthetic one: `modes` decides **which
images to build**, so it has to be known before an image exists to inspect. A
build-time label cannot answer a question asked at schedule time.

## Modes

`modes: all` — the normal case for a compiled backend — or an explicit list. An
interpreter declares `strict` alone: CPython has one floating-point semantics,
with no `-ffp-contract` to turn off and no `-ffast-math` to turn on, so `fma` and
`fast` would be the *same image under another tag*. Building them would put three
rows in the table whose only difference is noise, and someone would eventually
read that noise as an effect of the FP mode.

A mode that is requested but not declared is skipped with a warning — a row
missing from a table with no explanation is worse than a redundant one. A mode
that is *misspelled* fails the campaign: under labels we fell back to building
everything, on the grounds that a redundant campaign beats a wrong one, but a
manifest is a deliberate statement and building three images where the author
asked for one is not "redundant" — it is a table carrying rows nobody meant to
publish.

## One Dockerfile per implementation

One Dockerfile per implementation, no templating: templating Dockerfiles would
badly reinvent the thing Dockerfiles already are, and per-implementation variance
(cargo-chef, `CGO_ENABLED=0`, `uv sync`, `native-image`) lives exactly where
templates are worst. The manifest describes a backend; it does not generate one.

The FP mode, the `-march` baseline and the toolchain version remain **build
args**, not directories — they do not change the Dockerfile's structure. Four
Dockerfiles, not twenty-four. This is Docker's own parameterization, not a codegen
layer of our own invention.

Every base image is pinned **by digest** (`FROM gcc@sha256:…`), never by tag. A
benchmark that silently changes when upstream pushes is not a benchmark.

Every image exposes the same entrypoint and prints exactly one JSON record per
invocation; that contract is specified in the repository's README.
