---
title: Where it runs
order: 10
summary: The noise floor, CI versus dedicated hardware, and the two kinds of data that must never be mixed.
---

**Measure the machine before measuring the backends.**

Run the same binary thirty times and look at the median absolute deviation. This
is a twenty-minute experiment and it decides what we are allowed to claim:

- MAD under ~2% → percentage-level claims (gcc versus clang) are defensible.
- MAD around 15% → conclusions stop at factors, not percentages. Document it.
  That is already more honest than most published benchmarks.

The noise-floor run is also the harness's first integration test: the same code
path, with the machine as the subject.

## CI (GitHub Actions)

- **All implementations of an architecture run in the same job, sequentially.** A matrix
  with one job per implementation would compare Rust on one physical machine to
  Go on another, and the result would be meaningless.
- One job on `ubuntu-latest`, one on `ubuntu-24.04-arm`.
- Machine metadata is recorded on every run. When two campaigns disagree, it is
  the first thing to check.
- Hosted runners have two to four hyperthreaded vCPUs and noisy neighbours.
  Scaling curves from CI are indicative only.

## Dedicated hardware

For percentage-level claims: a bare-metal node, `kubectl cordon`-ed out of the
scheduling pool, driven over SSH with a plain `docker run` — **not as a pod**.
That removes kubelet, containerd, cadvisor, the CNI daemon and the entire
DaemonSet argument in one move. Kubernetes manages the cluster; it does not need
to manage this.

On that node:

- `performance` governor, **turbo disabled**. With turbo on, repetition 1 runs at
  5 GHz and repetition 30 at 3.4 GHz because the package heated up. That is
  drift, not noise, and no median rescues it.
- Optionally `isolcpus` / `nohz_full` / `rcu_nocbs`, which remove the cores from
  the Linux scheduler entirely.

If the benchmark must run *inside* Kubernetes, the mechanism is Guaranteed QoS
plus the static CPU Manager policy (`full-pcpus-only`), reserved CPUs for the
system, and an audit of every DaemonSet that tolerates your taint. It is more
work than cordoning a node, for a worse result.

**Verify, do not trust.** Before any campaign: check `Cpus_allowed_list` in
`/proc/<pid>/status`; confirm `nr_throttled` is zero in the cgroup's `cpu.stat`;
read `scaling_cur_freq` *during* the run; and interleave a fixed calibration
sentinel at the start, the middle and the end. If the sentinel drifts, discard
the campaign — that is the thermal throttling you did not see coming.

## Observability: the machine, not the measurement

Two kinds of data, two stores. They must not be mixed.

**Measurement data goes in a file.** Exact, complete, archivable. A campaign is
an NDJSON file you commit, diff, and reread in two years. Thirty discrete
observations, where a lost sample is a silent hole in the result.

**Environment data goes in Prometheus.** Dense, sampled, disposable.
`node-exporter` on the bench node at a one-second scrape turns the "verify, do not
trust" list into a dashboard: CPU frequency, package temperature, throttle
counters, plotted across the campaign. When round 22 comes in 8% slow, you look
at the graph instead of speculating.

Pin the exporter to the **reserved** cores. A monitor that perturbs what it
monitors is a classic of the genre.

### Never push benchmark metrics to Prometheus

Three independent reasons, each sufficient on its own:

1. **Pushing needs network**, and a network namespace cannot be added mid-run.
   Giving the container network for its whole life destroys the `--network=none`
   guarantee, trading a structural invariant for a convention.
2. **Prometheus stores `float64`.** The checksum is a sum of 64-bit integers; past
   2⁵³ it stops being exact. The bit-identical strict-mode invariant — the thing
   that catches the bugs tests do not — would be silently lost.
3. **A TSDB is lossy by design** (a missed scrape is fine, the next arrives in
   fifteen seconds) and pull-based (a container that lives four seconds is never
   scraped). Keeping all thirty repetitions would mean encoding the round number
   in a label — using a time-series database as a key-value store. That is the
   smell that says it is the wrong database.

## Why there is no energy column

Joules would be the metric that makes a backend comparison worth reading. The harness
measured them, briefly, and the column is gone. It is worth writing down why, because
the argument for adding it back is very good and the reason it fails has nothing to do
with the argument.

Energy is the one measurement a container cannot take for itself. `cpu.stat` and
`memory.peak` are cgroup files — namespaced, so the entrypoint reads its own and
reports it. RAPL is not. `/sys/class/powercap` describes a **socket**, not a cgroup,
and it is invisible from inside a container. So it has to be read on the *host*, around
the `docker run`.

And on the machines this project actually runs on, it cannot be read at all:

- **AArch64 has no counters.** RAPL is x86 (AMD drives the same `intel-rapl` powercap
  zones, misleading name and all). There is no equivalent to fall back to.
- **The x86 runner's counters are unreadable.** Since the PLATYPUS side-channel,
  distributions ship `energy_uj` root-only, and a GitHub Actions runner does not hand
  out the host's `/sys`.

The campaigns are run in CI, on those runners, and the result was not *some* missing
rows. It was `energy_uj: null` on **every sample of every campaign, on both
architectures** — 1140 nulls, and not one number. The bench machine is the CI runner;
there is no other machine, and a column the bench machine can never fill is not a
measurement. It is a promise.

A column that reads `n/a` on every row of every published campaign is worse than no
column. It invites the reader to assume a future campaign will fill it, and it keeps a
whole reading path alive — a meter, a wire format, a unit in a closed enum, a chart, a
docs section — for a number that has never once been produced. When the machine that
publishes changes, this section is the argument to re-open, not the code to un-delete.
