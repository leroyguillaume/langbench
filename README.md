# langbench

A benchmark harness comparing **compiler and runtime backends**, not languages.

`langbench` discovers benchmark implementations on disk, builds one container per
implementation, runs them under a controlled protocol, and writes raw samples —
which it then renders as a CSV or a Markdown report, on demand.

The question it answers is *given the same source, how do different backends
compare?* — gcc versus clang on identical C, rustc-LLVM versus rustc-cranelift on
identical Rust, CPython versus PyPy, OpenJDK versus GraalVM `native-image`.

> **Read [METHODOLOGY.md](METHODOLOGY.md) before trusting any number this
> produces.** It documents what is measured, what is deliberately not measured,
> and why a benchmark that skips those questions produces confident nonsense.

**The results live in two directories**, one campaign per ISA:

- **[`samples/`](samples/)** — `samples/<arch>.ndjson`, the raw samples. The only
  artifact a campaign produces that cannot be recomputed, and the source of
  everything below.
- **[`reports/`](reports/)** — `reports/<arch>.md`, each rendered from the samples
  beside it. [What is in there](reports/README.md).

They are kept apart by architecture and never merged: **an absolute timing does
not cross an ISA** ([why](METHODOLOGY.md#the-isa-rule)).

## Requirements

- Rust 1.94+ (edition 2024)
- Docker with a reachable daemon
- **Linux, for any result you intend to publish.** On macOS or Windows the
  containers run inside a VM, and the harness will say so at the top of the
  report.

## Install

```sh
cargo build --release
# the binary lands in target/release/langbench
```

## Inspect the machine first

```sh
langbench machine
```

Prints exactly what a campaign would record in its header, and every reason this
host is a poor benchmark target. Cheap, and worth doing before spending an hour
measuring on a machine that was never going to produce a trustworthy number.

```
PROPERTY                VALUE
---------------------   ----------------------------------------
Hostname                bench-01
Kernel                  6.8.0-45-generic
Virtualization          none detected
Scaling governor        performance
Turbo disabled          true
...

No warning: this host looks like a usable target.
```

## Run

```sh
langbench run
```

By default that discovers everything under `benchmarks/`, builds each
implementation in every floating-point mode its `bench.yaml` declares it
distinguishes (all three unless it says otherwise), and measures with the machine's
thread count. It writes `samples.ndjson` and nothing else; `langbench csv` and
`langbench md` turn that file into a table or a report, whenever you like.

Common flags — every one of them also reads an environment variable:

| Flag | Env | Default | Purpose |
| --- | --- | --- | --- |
| `--algo` | `ALGO` | all discovered | Restrict to some algorithms |
| `--mode` | `FP_MODE` | `strict,fma,fast` | Floating-point semantics |
| `--cpu` | `CPUS` | machine parallelism | Threads for kernels *and* compilers |
| `--output`, `-o` | `SAMPLES_OUTPUT` | `samples.ndjson` | Path of the samples file the campaign writes |
| `--benchmarks-dir` | `BENCHMARKS_DIR` | `benchmarks` | Root of the benchmark tree |
| `--grid-size` | `GRID_SIZE` | `2048` | Side of the N×N grid |
| `--max-iter` | `MAX_ITER` | `1000` | Iteration ceiling |
| `--rounds` | `ROUNDS` | `10` | Measured run rounds |
| `--build-rounds` | `BUILD_ROUNDS` | `3` | Measured build rounds |
| `--warmup-rounds` | `WARMUP_ROUNDS` | `1` | Rounds recorded but flagged |
| `--march` | `MARCH` | per-ISA baseline | ISA baseline. `native` is rejected |
| `--memory-limit-mb` | `MEMORY_LIMIT_MB` | `8192` | Memory budget of every measured container. Part of the measurement — see below |
| `--run-timeout` | `RUN_TIMEOUT` | `600` | Seconds before a container is killed |
| `--log-filter` | `LOG_FILTER` | `info` | [`tracing` filter directive][directives] |

[directives]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives

Example — only the strict mode, on four threads, for one algorithm:

```sh
langbench run --algo mandelbrot --mode strict --cpu 4 --output results/strict-4.ndjson
```

### Sizing a campaign

`--grid-size` and `--max-iter` are the same for every implementation, and they
have to be: the strict-mode checksum is a function of both, so a campaign cannot
give each backend its own grid without giving up the correctness gate.

**Size for the slowest backend.** The work scales as `grid_size² × max_iter`, and
what a campaign actually waits on is CPython, not C. At `4096` / `1000` a C run
takes about a second and a CPython run takes forty; multiply by the rounds and
the modes and you get an hour of CPython for a single campaign.

The defaults above are therefore sized for **iteration speed** — a quarter-grid,
ten rounds, one warmup — which puts a full three-mode campaign in the minutes. For
numbers you intend to publish, buy back the resolution:

```sh
langbench run --grid-size 4096 --rounds 30 --warmup-rounds 2 --build-rounds 5
```

Nothing about the smaller default is *wrong*: the estimate is a min-of-N, so more
rounds can only ever lower it, and the dispersion printed beside it tells you
whether N was large enough. A short campaign is pessimistic, never incorrect.

The harness logs one line per invocation so you can watch this happen; if nothing
has moved after `--run-timeout` seconds, the container is killed and that backend
is quarantined — a hung run is not a slow run, and the rest of the campaign is
none of its business.

Sizing for the slowest backend makes the fastest one's *wall-clock* mostly
container startup — which is why the report also carries `Compute min`, timed
inside the program and unaffected.

### The memory budget is part of the measurement

`--memory-limit-mb` is not a safety rail, and changing it changes the results — all
of them, not only the memory column.

A garbage-collected runtime sizes its default heap from what its cgroup shows it: a
JVM takes a quarter of it. Leave the budget unset and it reads the *host's* RAM, so
the peak memory we publish would describe the bench machine rather than the backend
— and moving the campaign to a box with twice the memory would "prove" that Java got
hungrier. Pinned, and pinned identically for every backend, it is a property of the
backend again.

The price is that a constrained heap is a different garbage-collection regime, so the
**timings move too**. Two campaigns run under different budgets do not compare, on any
column. Pick a value, write it down, and leave it alone.

The default of `8192` is set by the hungriest *compiler*, not by the kernels — the
kernels need almost nothing, `native-image` needs gigabytes, and the build-phase tmpfs
is charged to the same cgroup. Lower it far enough and you will not get smaller
numbers: you will get a quarantined backend and a missing row.

Swap is off. A container permitted to swap does not *fail* when it overruns its
budget — it silently gets slower, and the timing quietly absorbs a page-fault storm
that no column explains.

### There is no energy column

`langbench` does not measure energy, and the reports carry no joules. The campaigns run
on GitHub Actions runners, where the RAPL counters cannot be read at all — they are
x86-only, so AArch64 has none, and root-only on most kernels since the PLATYPUS
side-channel, so the x86 runner has none either. Every sample of every campaign came
back empty.

A column that is `n/a` on every row of every published campaign measures nothing. See
[METHODOLOGY.md](METHODOLOGY.md#why-there-is-no-energy-column).

### When a backend breaks

It does not take the campaign with it. An image that fails to build, a kernel that
segfaults, a run that hangs past `--run-timeout`, a `strict` checksum that
disagrees with the reference — each of those takes out **that one
`(backend, mode)` unit**, at the point it breaks, and the campaign carries on
measuring the others. Nothing is retried: whatever broke in round one breaks in
round nine.

The failure is not swept up, either. It is written to `samples.ndjson` as a
`failure` record, and every rendering shows it — `langbench md` grows a *What did
not finish* section, and so does the website. A benchmark that silently drops what
did not work flatters itself, and a missing row looks exactly like a backend nobody
ever wrote.

The one case that still fails the campaign is *everything* failing: a samples file
with a header and no samples renders into an empty table, and an empty table is a
lie told quietly. That exits non-zero — usually because the Docker daemon is not
running, which fails identically for every backend.

### Stopping a campaign

Ctrl-C, or `SIGTERM` from whatever is supervising it. The harness kills the
container in flight, stops, and **exits 0** — the samples it already wrote are
untouched and still render:

```sh
langbench md    # a campaign you interrupted after ten minutes is still a report
```

The run it was in the middle of is discarded rather than recorded: a killed
container has no honest timing to give, and a wrong run never enters the
statistics. So an interrupted campaign is a *shorter* campaign, never a
corrupted one — you lose resolution, and the dispersion beside each estimate
tells you how much.

Killing the container matters more than it sounds. The workload does not run in
the `langbench` process; it runs on the Docker daemon, in another process tree,
and `docker run` is only a client attached to it. A harness that simply died on
`SIGTERM` would leave the benchmark running — on this machine, that orphan was
happily holding ten cores with nobody left to watch it, which is a bias in
whatever you measure next, not merely an untidy `docker ps`.

Signal a second time and it exits immediately, at the cost of that orphan; the
log tells you so, and the container to `docker kill` is named `langbench-*`.

## Run in a container

An image is provided, mostly for CI. Running the harness in Docker means it has
to start Docker containers of its own, and there are two ways to do that. This
image uses the one that does not corrupt the measurement.

### Sibling containers, not nested ones

`langbench` orchestrates containers; it never runs the workload itself. So the
image ships a Docker **client** and talks to the **host's** daemon through its
socket. The benchmark containers it starts are therefore **siblings** of the
harness container — both are children of the host daemon — and the harness sits
entirely outside the measured path.

```
            host daemon
            ├── langbench          (the client — orchestrates, measures nothing)
            ├── mandelbrot-c-gcc   (measured)
            └── mandelbrot-cpython (measured)
```

This is often called *Docker-out-of-Docker*. The alternative — true
Docker-in-Docker, `--privileged` with a second daemon nested inside the harness
container — would put the benchmark containers **inside** langbench:

```
            host daemon
            └── langbench (--privileged)
                └── nested daemon
                    └── mandelbrot-c-gcc   (measured — through two runtimes)
```

**Do not do that here.** Every measured run would execute under a nested
storage driver and a second layer of cgroup and namespace indirection, and the
CPU time the harness reads from `cpu.stat` would come from a cgroup nested
inside another container's cgroup. That is a benchmark of your container
runtime's overhead, wearing the costume of a benchmark of gcc. The whole point
of the sibling model is that the harness contributes nothing to the numbers.

### Doing it

The benchmark tree ships **inside** the image, so a campaign needs only the
host's socket and somewhere to put the samples. The two differ in exactly one
flag — `--group-add`, the group that owns the socket — and getting it wrong
fails with `permission denied while trying to connect to the docker API`.

```sh
docker build --tag langbench .
mkdir -p results
```

**Linux.** The socket is owned by the host's `docker` group, so hand the
container that gid. Look it up rather than hardcoding `999`; it varies by
distribution:

```sh
docker run --rm \
  --hostname "$(hostname)" \
  --group-add "$(stat -c '%g' /var/run/docker.sock)" \
  --volume /var/run/docker.sock:/var/run/docker.sock \
  --volume "$PWD/results:/var/lib/langbench" \
  langbench run --mode strict
```

**macOS** (Docker Desktop, OrbStack, Colima). The daemon lives in a Linux VM,
and the socket a container actually sees there is `root:root`, mode `660`,
whatever the Mac says about the file. So the gid is `0`:

```sh
docker run --rm \
  --hostname "$(hostname)" \
  --group-add 0 \
  --volume /var/run/docker.sock:/var/run/docker.sock \
  --volume "$PWD/results:/var/lib/langbench" \
  langbench run --mode strict
```

Do not port the Linux line across by swapping `stat -c` for BSD's `stat -f '%g'`:
it *runs*, and it reports gid `1`, which grants nothing. Treat a macOS run as a
smoke test regardless — the whole workload sits in a hypervisor, and the harness
says so at the top of the report.

`/var/lib/langbench` is the working directory, and everything a campaign writes
lands directly in it — `samples.ndjson`, and later `samples.csv` and `report.md`.
That single mount is what carries the results back out.

Renderings work the same way on either platform, against the samples the campaign
left behind. They need neither the socket nor the benchmark tree — `csv` and `md`
are pure functions of the samples file, so no `--group-add` either:

```sh
docker run --rm \
  --volume "$PWD/results:/var/lib/langbench" \
  langbench md          # samples.ndjson -> report.md, both in results/
```

### Benchmarking your own tree

The bundled benchmarks are a default, not a constraint. Mount your own tree over
the image's, read-only — the harness never writes to it. That is one extra
`--volume` on the campaign command above, whichever platform's `--group-add` you
took from it:

```sh
docker run --rm \
  --hostname "$(hostname)" \
  --group-add "$(stat -c '%g' /var/run/docker.sock)" \
  --volume /var/run/docker.sock:/var/run/docker.sock \
  --volume "$PWD/results:/var/lib/langbench" \
  --volume "$PWD/benchmarks:/usr/local/share/langbench/benchmarks:ro" \
  langbench run
```

`/usr/local/share/langbench/benchmarks` is where the image keeps them, and
`BENCHMARKS_DIR` points there. It sits outside `/var/lib/langbench` deliberately:
inputs are read-only data, outputs are the mount, and a benchmark tree nested
under the output directory would vanish the moment you mounted one over it.

Four things to know:

- **Mounting the socket grants root-equivalent access to the host.** Anything
  that can talk to that socket can start a privileged container and own the
  machine. Do not do this on a host you do not own, and do not expose it to
  anything you would not trust with root.
- **Mount `/var/lib/langbench` itself.** With `--rm` and nothing mounted there,
  the samples die with the container — and the samples are the one artifact that
  cannot be recomputed.
- **Pass `--hostname`.** Otherwise the harness records the container's ID as the
  machine's hostname, and the campaign is harder to attribute later.
- **`--group-add` is needed because the image runs as a non-root user** (UID
  1000), which is not in the host's `docker` group. Never work around this by
  running the harness as root.

The harness detects that it is containerized and says so in the report, along
with any hypervisor it can find. That detection is a runtime check, not a
compile-time one — inside a Linux container a compile-time check would report
"Linux" on any host, including the macOS laptop it was never meant to trust.

## Output

A campaign writes **one** file, the one `--output` (`SAMPLES_OUTPUT`) names: a
header record with the full machine description and campaign parameters, then one
line per measured invocation, flushed as it is produced. It is the source of
truth, and the only artifact that cannot be recomputed — an interrupted campaign
keeps every sample it completed.

Everything else is a *rendering*, produced afterwards by a separate command. That
is the point: a report can only ever show what a run actually recorded, and the
same file re-renders identically on any host, months later.

```sh
langbench csv       # the samples, flat, into samples.csv
langbench md        # the samples, as a report, into report.md

langbench md results/strict-4.ndjson --output results/strict-4.md
```

Each command reads the samples the campaign wrote and writes a file of its own.
No rendering goes to stdout, so none is lost to a forgotten redirection; stdout
carries only what has no file of its own, and logs go to stderr:

| Command | Flag | Env | Default |
| --- | --- | --- | --- |
| `run` | `--output`, `-o` | `SAMPLES_OUTPUT` | `samples.ndjson` |
| `csv` | `--output`, `-o` | `CSV_OUTPUT` | `samples.csv` |
| `md` | `--output`, `-o` | `MD_OUTPUT` | `report.md` |

The samples path is the positional argument of `csv` and `md`, and it defaults to
the same `SAMPLES_OUTPUT` a `run` wrote — so a campaign and the report that
follows agree without being told twice. Missing parent directories are created.

**CSV.** The same records, flat, for a spreadsheet or a dataframe. Missing values
are **empty fields**, never `n/a`: a numeric column that sometimes holds a word
breaks every parser that reads it. The campaign's context (machine, grid size,
`-march`) has no room in a flat table, which is exactly why the NDJSON stays the
source of truth.

```sh
# Median run time per mode, straight from the samples.
langbench csv
awk -F, 'NR>1 && $6=="run" && $8=="false" { print $5, $10 }' samples.csv
```

**Markdown.** A human-facing view that leads with any reason this host is a poor
benchmark target. **One per ISA, in [`reports/`](reports/)** —
[`reports/aarch64.md`](reports/aarch64.md), and `reports/x86_64.md` once the
[`bench`](.github/workflows/bench.yaml) workflow has run — each rendered from the
campaign of the same name in [`samples/`](samples/). It renders
`templates/report.md.liquid`, embedded in the binary; `--template` swaps in your
own [Liquid][liquid] template, which receives exactly the same variables:

```sh
cp templates/report.md.liquid mine.liquid   # the built-in one, as a starting point
langbench md --template mine.liquid         # renders into report.md
```

[liquid]: https://shopify.github.io/liquid/

## The website

The third rendering. `site/` is an [Astro][astro] site — three prerendered pages,
published to GitHub Pages by
[`.github/workflows/pages.yaml`](.github/workflows/pages.yaml):

| Page | What it is |
| --- | --- |
| `/` | The campaign: tiles, charts, and the table `report.md` prints — sortable, and filtered by any field of the triple |
| `/compare/` | Two languages head to head, and whether the gap between them is a difference |
| `/methodology/` | [`METHODOLOGY.md`](METHODOLOGY.md), rendered at build time. No JavaScript: it is prose |

Static output, because the host is a file server: every route is a real `.html`,
so a deep link, a refresh and a shared URL all work without the `404.html`
fallback a client-side router needs on GitHub Pages. Everything that reads a
campaign is a React island, and Astro's `ClientRouter` swaps pages without
reloading the document — so the WebAssembly instance and the parsed campaigns
survive a navigation, and the 500 kB of samples are fetched **once**.

[astro]: https://astro.build

It is a rendering like the other two, which means three things it would have been
much easier not to do:

**The site computes no statistic of its own.** Min-of-N, the buckets, the
definition of startup — all of it is `src/analysis.rs`, the same code
`langbench md` calls, compiled to WebAssembly and called from the browser. A
second implementation of min-of-N in TypeScript would be a second definition of
what this project measures, and the two would drift the first time one of them
was "fixed". The site sorts, formats and draws; it does not do arithmetic on the
samples.

**A gap smaller than the dispersion is not a difference.** `/compare/` puts two
rows side by side and answers the question the table leaves open: *is this gap
real?* A 3% gap between two rows that each wobble by 9% is a **tie** — not equal,
*indistinguishable*, which is a statement about the campaign and not about the
backends. That verdict is `src/compare.rs`, in the harness, for the same reason
min-of-N is: what counts as a difference is a definition of what this project
measures, and it has one home
([`METHODOLOGY.md`](METHODOLOGY.md#a-difference-smaller-than-the-dispersion-is-not-a-difference)).
The pair lives in the URL — `?a=c/gcc/-/strict&b=python/-/cpython/fast` — because a
comparison nobody can link to is a comparison nobody can check.

**A row is named by its triple, never by a slug.** You pick a *language* first, and
then the toolchain that ran it, because that is the question a reader arrives with.
The harness carries a `backend` slug on the wire — its Markdown template needs a
string to hang an anchor off — and the site never shows it, never sorts on it and
never puts it in a URL: `java-native-image` reads as "java, native" and is in fact
java, compiled by `native-image`, with no interpreter at all. Three fields that say
it outright beat one name you have to decode. An absence is a published fact, so
`n/a` is rendered rather than blanked — and it is selectable as a filter, because
"the implementations that ship machine code and no runtime" is a question with an
answer.

**The site never calls `JSON.parse` on a campaign.** `checksum` is a 64-bit
integer and a JavaScript number is a double: `JSON.parse` rounds every integer
past 2^53 without a word, and the checksum is the correctness gate of the whole
harness. So `samples.ndjson` is fetched as *text*, parsed in Rust by `serde_json`,
and the checksums come back as strings. The page displays them and compares them;
it never adds them.

**One campaign per ISA, never merged.** An absolute timing does not cross an ISA
([`METHODOLOGY.md#the-isa-rule`](METHODOLOGY.md#the-isa-rule)): a millisecond on
x86-64 and a millisecond on aarch64 are not the same claim. So the site loads
every campaign and shows **one at a time**, behind an ISA selector — comparing
backends *within* an architecture, which is what a ratio is for. The ISA travels
with the link to `/compare/`, so a head-to-head never quietly changes architecture
under the reader. The architecture it keys on is the one the machine recorded in
the campaign header, never the one in the filename: a filename is a label somebody
typed.

The data it publishes is each campaign's samples, byte for byte — no export
format, no intermediate file. They live in [`samples/`](samples/), one per ISA:

```sh
langbench run --output samples/x86_64.ndjson
langbench md samples/x86_64.ndjson --output reports/x86_64.md
```

Every `samples/*.ndjson` is picked up and published. Locally:

```sh
cd site
npm install
npm run dev        # compiles the crate to WASM, copies the campaign in, serves it
npm run build      # into site/dist/
npm test           # runs the real WASM module over the real campaign
```

Every script that reads the module -- `dev`, `build`, `typecheck`, `test` --
invokes `wasm-pack` first, so a change to `src/analysis.rs` reaches the page, the
types and the tests without a second step. `site/src/wasm/` is that artefact and
is never committed: a script that skipped the build would check a fresh clone
against a module that does not exist, and a working copy against a stale one.

Requirements beyond the harness's own: Node 22+, and

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

Two environment variables move the paths, and neither changes what is published:

| Variable | Default | What it moves |
| --- | --- | --- |
| `SAMPLES_DIR` | `samples` | Where the campaigns are **read from**. Relative to the repository root, or absolute. |
| `BASE_PATH` | `/` | The URL prefix the built site is **served under** — `/langbench/` for a GitHub Pages project site. |

`SAMPLES_DIR` is how you look at a campaign you ran here and are not committing.
`samples.local/` is gitignored for exactly that — a laptop's numbers have no
business in `samples/`, which is what the bench runners publish:

```sh
langbench run --output samples.local/$(uname -m).ndjson
cd site && SAMPLES_DIR=samples.local npm run dev   # serves your campaign, not the committed one
SAMPLES_DIR=/mnt/bench/campaigns BASE_PATH=/langbench/ npm run build
```

The default stays `samples/` whatever is lying around on disk, so the published
site is built from the committed campaigns without anyone passing anything — and
pointing the page at your own numbers is a thing you can see yourself typing.

Whatever `SAMPLES_DIR` points at, the `.ndjson` files under it are copied into
`public/data/` byte for byte and indexed in `campaigns.json`; the page still reads
each campaign's ISA out of its own machine record, never out of a path.

## Continuous integration

Five workflows. Three of them are one gate — every push to `main`, every pull
request. The other two you have to ask for: one measures, the other publishes the
page.

**[`quality`](.github/workflows/quality.yaml)** — `pre-commit run --all-files`:
`cargo fmt`, `clippy`, `hadolint`, `actionlint`, `biome`, `tsc`, the generated
`bench.schema.json`, every `bench.yaml`. What is cheap to check and expensive to
get wrong. **No path filter** — it is the universal gate and runs on everything,
whatever changed.

**[`test`](.github/workflows/test.yaml)** — `cargo test` and the site's `vitest`,
one job each. The kernels are not among them: they are verified by the strict-mode
checksum invariant, which only a campaign can check. Its path filter is the union
of what the two suites *read*, which includes `benchmarks/**` — `tests/shared_kernel_source`
asserts that two backends of one language compile a byte-identical kernel, so a
kernel is one of its inputs.

**[`build`](.github/workflows/build.yaml)** — that everything still compiles:
`cargo build --release --locked`, and the site's `npm run build`, which is
`wasm-pack` → `tsc` → `vite`. That first link is the one nothing else covers — the
crate has to compile to `wasm32-unknown-unknown` **without** the half of it that
talks to Docker, and a `std::process::Command` that creeps into `analysis.rs`
breaks the website and nothing else would say so. It builds over the committed
fixture rather than `samples/`: `main` may legitimately publish no campaign, and
the question this workflow asks is whether the bundle compiles — which needs *a*
campaign, not *the* campaign. It publishes nothing. Its filter is `test`'s minus
`benchmarks/**` and `tests/**`: a kernel is source the harness ships a Dockerfile
for, never source it links.

**[`bench`](.github/workflows/bench.yaml)** — runs **one campaign per ISA**, on a
matrix of native runners (`ubuntu-24.04` and `ubuntu-24.04-arm`; **never QEMU**,
which measures the emulator). It commits every `samples/<arch>.ndjson` **and** its
`reports/<arch>.md` back to `main`. Both files, never one without the other: the
samples are the only artefact that cannot be recomputed, and committing a report
without its evidence publishes a conclusion nobody can check. Then it calls
`pages` with the SHA of that commit, so a campaign still ends with a published
page.

**`workflow_dispatch` only** — it takes `grid_size`, `max_iter` and `rounds` as
inputs. A campaign is a deliberate act, not a side effect of a push: it costs
minutes of runner time and it rewrites the two files this repository publishes,
which on every commit would mean churning them for reasons that have nothing to do
with the backends being measured.

**[`pages`](.github/workflows/pages.yaml)** — builds the site from the campaigns
committed at a given `ref` and deploys it to GitHub Pages. It measures nothing — it
is the third rendering, a pure function of `samples/` — which is exactly why it is a
separate workflow: a change to `site/` alone reaches the page without spending an
hour of runner time re-measuring backends that have not moved.

It fires three ways. **On a push to `main`** that touches anything the page is made
of: `site/`, `src/` (which *is* the site's arithmetic — `analysis.rs` and
`compare.rs` are compiled to WebAssembly and called from the browser),
`METHODOLOGY.md` and `docs/columns.md` (rendered verbatim), or the workflow itself.
**Via `workflow_call`**, from `bench`, once a campaign's samples are committed —
which is why `samples/` and `reports/` are deliberately *not* in that path filter:
`bench` publishes them and then calls this workflow itself, and a push trigger on
them would race a second, identical deploy of the same commit. And **on
`workflow_dispatch`**, to republish any `ref` by hand.

**A number this workflow produces is indicative, not publishable.** A GitHub
runner is shared, virtualised and frequency-scaled — the worst benchmark target
money can rent. It says so itself: the report and the page both lead with every
reason the host was a poor target. For a number worth trusting, run a campaign on
a real machine and commit its `samples/<arch>.ndjson`; the report and the site are
pure functions of that file and will render it unchanged.

## Adding an implementation

Drop a `bench.yaml` next to a `Dockerfile`, anywhere under `benchmarks/`:

```yaml
algo: mandelbrot
language: python
compiler: cython      # omit if nothing is compiled ahead of the run
interpreter: cpython  # omit if the binary runs on the bare CPU
source: mandelbrot.py # the one kernel file, beside this manifest
modes:
  - strict            # or `modes: all`
arch: all             # the default; omit it unless the toolchain does not exist somewhere
description: >-
  The same mandelbrot.py as python-cpython, byte for byte, compiled by Cython to
  a C extension module instead of interpreted.
comments: >-
  It is slower than the interpreter it compiles, and that is a result, not a bug.
```

**The manifest is the only thing the harness reads.** Discovery walks the tree
for `bench.yaml` files — the directory layout is yours to choose, and the
directory *name* means nothing. An implementation is identified by what it is:
`(algo, language, compiler, interpreter)`. Declare the same tuple twice and the
campaign refuses to start.

`compiler` and `interpreter` are each optional, and each absence is a published
fact rather than a gap: gcc compiles and nothing interprets, CPython interprets
and nothing compiles ahead of the run, Cython does both. That last case is why
there is no single "compiler" field — `python-cython` and `python-cpython` share
a language *and an interpreter*, and differ only in the compiler. That is the
clean experiment, and a directory name could not have expressed it.

`source` names the one kernel file, and it is **declared rather than guessed**: the
harness will not decide for itself which of the files beside the Dockerfile is the
source, because the only way to do that is to pattern-match a filename — and the path
is not metadata. Its size in bytes is published as a column. That column is a property
of the *language*, not of the backend: `c-gcc` and `c-clang` compile the same file and
report the same number, which is exactly what it should say about code written once
and compiled twice. It is not a measure of quality, and it is not a measure of effort.

`arch` is `all` unless the backend's **toolchain does not exist** for an
architecture — and that is a fact, not a preference. Kotlin/Native, for instance,
publishes no `linux-aarch64` host compiler, so `arch: [x86_64]` is simply the
truth about it; the only ways to run it on an ARM machine would be emulation
(which this project forbids) or cross-building (which would time a build that
never happened here). A campaign on the other architecture **skips the row and
says so**, instead of failing halfway through with a `docker build` error.

`modes: all` — the normal case for a compiled backend — or the list of modes the
backend actually distinguishes. An interpreter has one floating-point semantics,
so `fma` and `fast` would be the same image under another tag; declare `strict`
alone and the harness measures that one, warning about each mode it skips. A
misspelled mode fails the campaign rather than quietly building something nobody
asked for.

The image must expose an `ENTRYPOINT` taking either `build <threads>` or
`run <n> <max_iter> <threads>`, and print **exactly one JSON object on stdout**
(everything else goes to stderr):

```json
{"phase":"run","checksum":31415926535,"elapsed_ns":4102337891,"user_usec":32418004,"system_usec":118273}
```

The floating-point mode, the `-march` baseline and the job count arrive as build
args (`FP_MODE`, `MARCH`, `JOBS`), so one Dockerfile covers every mode.

Docker `LABEL`s are still welcome — `langbench.version`, `langbench.flags` — but
they are provenance for `docker inspect`, and the harness never reads them. Two
sources of truth is one source of drift.

Read [METHODOLOGY.md#container-contract](METHODOLOGY.md#container-contract) for
the full contract, including why the checksum must be an integer and why the
build directory is a tmpfs.

### Checking a manifest before you spend an hour on it

```sh
langbench validate            # every failure a campaign would hit at discovery
```

It parses every `bench.yaml` on disk and reports **all** the problems at once —
a misspelled key, an unknown FP mode, a backend that neither compiles nor
interprets, two manifests claiming the same identity — without building a single
image. Point it at a file or a directory; it defaults to the whole tree.

`bench.schema.json`, at the repo root, is the manifest's JSON Schema. It is
generated from the Rust struct the harness actually deserializes, never written
by hand:

```sh
langbench jsonschema          # rewrites bench.schema.json
```

A pre-commit hook regenerates it and fails if the checked-in copy has drifted, so
the schema your editor completes from cannot disagree with what the campaign
accepts. To get that completion, point your editor at it — for VS Code's YAML
extension:

```json
{"yaml.schemas": {"./bench.schema.json": "**/bench.yaml"}}
```

## Development

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check

pre-commit install   # fmt and clippy, plus hadolint, actionlint, biome, tsc, and
                     # the two manifest hooks: `validate` runs whenever a bench.yaml
                     # moves, and bench.schema.json is checked for drift
```

**The suites are not hooks.** `cargo test` and `vitest` are the slowest thing
either language has to offer, and a hook that is meant to be instant is the wrong
place to pay for them — run them yourself, and the
[`test`](.github/workflows/test.yaml) workflow runs them on every push anyway, in
parallel with everything else.

## What ships

Thirty implementations of Mandelbrot across thirteen languages, and **every one of
them agrees on the strict-mode checksum, bit for bit** — from `gcc -O3` to a
JIT-compiled Julia script to JavaScript running in a Bun worker.

| Language   | Backends                    | Modes  | What the group buys                                        |
| ---------- | --------------------------- | ------ | ---------------------------------------------------------- |
| C          | `gcc`, `clang`              | all    | Two code generators; same source, distro, libc and linker  |
| C++        | `gcc`, `clang`              | all    | The same, plus what `std::thread`/`std::atomic` cost over C |
| Rust       | `rustc`                     | strict | Scoped threads, no `rayon`, no `-ffast-math` to reach for  |
| Zig        | `zig`                       | strict | A static, libc-free binary; threads are raw `clone` calls  |
| Go         | `gc`                        | strict | Goroutines — and the fused-multiply-add trap below         |
| Julia      | *(JIT, no AOT compiler)*    | strict | Compiles through LLVM, but *during* the run                |
| Python     | `cpython`, `pypy`, `cython` | strict | One source: an interpreter, a JIT, and an AOT compiler     |
| JavaScript | `nodejs`, `deno`, `bun`     | strict | One `.mjs`: V8 twice, JavaScriptCore once                  |
| TypeScript | `nodejs`, `deno`, `bun`     | strict | One `.mts`: the types are erased, so what moves is startup |
| Java       | `javac` × `openjdk`, `graalvm`, `openj9`; `native-image` | strict | One source: two JITs, an interpreter, and an AOT compiler |
| Kotlin     | `kotlinc` × `openjdk`, `graalvm`, `openj9`; `native-image` | strict | The same four backends, one language over                |
| Scala      | `scalac` × `openjdk`, `graalvm`, `openj9`; `native-image`; **`scala-native`** | strict | And one that leaves the JVM entirely, via LLVM |

Backends of the same language compile a **byte-identical kernel** — a test
enforces it — so each group is a clean "same source, different backend"
experiment rather than a comparison of three different programs.

The JVM rows are the table's clearest case of a backend that both compiles *and*
interprets: `javac`/`kotlinc`/`scalac` emit bytecode ahead of the run, then a JVM
interprets it and JIT-compiles the hot loop *during* the run. The four Java rows
run **one identical `Mandelbrot.java` through four different backends** — HotSpot's
C2, GraalVM's Graal JIT, Eclipse OpenJ9's Testarossa, and GraalVM `native-image`
compiling ahead of time to a standalone ELF with no JVM at all. That is the same
interpreter/JIT/AOT triangle Python has, and it is the point of the group.

**The AOT rows land where C does.** `native-image` computes in ~13 ms against C's
~13 ms and Rust's ~14 ms, while every JIT row sits near ~30 ms — because a JIT is
still *warming up inside the region we are timing*, and an AOT binary arrives
already compiled. Scala Native, going through LLVM instead, lands there too
(~14 ms). What AOT costs instead is the **Build** column: 15–19 s of whole-program
analysis for `native-image`. It is not free, it is prepaid.

**The (language × JVM) grid is filled in, and the two axes are orthogonal.** Kotlin
and Java compute within a whisker of each other on *every* JVM (27.4 vs 27.5 ms on
OpenJDK, 27.5 vs 26.7 on Graal) — by the time a JIT sees this kernel it is the same
loop, whoever emitted the bytecode. So the grid is now evidence rather than an
assumption, which is the only reason to have built it. Where the languages *do*
separate is **Build**: kotlinc is several times javac on one file, and AOT-compiling
a language runtime costs more the heavier that runtime is (Scala's is the heaviest
here).

One caveat, in the kernel where it cannot be missed: Scala has no `break`, so its
inner loop carries an escape flag that C, Java and Kotlin do not pay for. The Scala
JVM rows compute ~30% slower for that reason alone. **That gap is the flag, not the
language** — read the Scala rows against each other, which is the axis this
benchmark actually measures.

One caveat, published rather than hidden: **HotSpot has no `-march`** and JITs for
whatever CPU it finds, so the JIT rows get a baseline the compiled rows were
denied — the entrypoints cap vector width, which is as close as a JVM gets, and
OpenJ9 cannot even do that. `native-image` is the only JVM backend with a real ISA
baseline. See
[METHODOLOGY.md](METHODOLOGY.md#the-jvm-cannot-honour-this-rule-and-says-so).

`strict` alone is not a shortcut. Where a backend declares only that mode,
relaxing the floating-point semantics would mean *editing the kernel* rather than
passing a flag (Go's `float64()` rounding points, Zig's `@setFloatMode`), or the
language offers no relaxation at all — rustc has no `-ffast-math`, and ECMAScript
forbids every JavaScript engine from contracting or reassociating. Each absence is
a published fact, not a gap.

### Two results worth the whole harness

**Cython is 1.3× slower than the CPython interpreter it compiles.** Without type
annotations the generated C manipulates `PyFloat` objects through the C-API — its
hot loop holds 142 call instructions and one `fadd`, where the C kernel has none
and fourteen — while CPython 3.13's specializing interpreter takes a fast path for
`float * float`.

**Go quietly computes something else.** Written the natural way, the Go kernel
returns `33209560` where every other language returns `33209574`: the spec permits
fusing multiply-adds across statements, and on arm64 `gc` takes the offer. It is
not slower and it is not buggy — it is computing a different thing, and nothing
but the checksum would have told us. The `float64(...)` conversions in
`mandelbrot.go` are rounding points rather than casts, and deleting one fails the
campaign. See
[METHODOLOGY.md](METHODOLOGY.md#the-languages-that-fuse-behind-your-back).

### Backends we tried, and left out

Both of these were probed, not guessed. Neither is a "todo".

**gccrs** (the GCC Rust frontend, Debian's `gccrs-14`). It refuses to compile any
Rust at all without a flag literally named
`-frust-incomplete-and-experimental-compiler-do-not-use`, and warns that "the
binaries produced might not behave accordingly". Forced past that, it does not know
`println!` — and printing the checksum is the anti-dead-code-elimination rule, not
decoration. A backend that will not vouch for the behaviour of its own output
cannot be measured against a bit-exact correctness gate. The day it compiles
`println!` and `std::thread`, it is a twenty-minute addition and the checksum will
tell us immediately whether to trust it.

**Kotlin/Native.** Two blockers. JetBrains ships no `linux-aarch64` host compiler
(that one is survivable — it is exactly what `arch: [x86_64]` is for). The real one
is that its kernel *cannot be byte-identical* to the JVM Kotlin kernel: there is no
common threading API in the Kotlin stdlib — `kotlin.concurrent.thread` is JVM-only,
`Worker` is obsolete, and the official multiplatform answer is `kotlinx.coroutines`,
a third-party dependency this project forbids. A Kotlin/Native row would therefore
compare a *different program*, which is precisely what
`tests/shared_kernel_source.rs` exists to prevent. Left out deliberately, not
forgotten.

Next: measure the noise floor of the target machine — nothing else is trustworthy
until that number exists.

## License

MIT
