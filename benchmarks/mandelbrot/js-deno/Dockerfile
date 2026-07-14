# Mandelbrot, JavaScript, Deno (V8).
#
# The same `mandelbrot.mjs` as `js-nodejs` and `js-bun`, byte for byte. Deno runs
# it because it implements `node:worker_threads` -- the one worker API all three
# runtimes agree on -- and because a `.mjs` is an ES module everywhere.
#
# Single stage: the `build` subcommand re-runs Deno's own ahead-of-run step to be
# timed, so the runtime must survive into the image.
FROM denoland/deno:debian-2.5.5@sha256:09cda54dca77d1e5b8af6176a8b6c29ecae0183dccf18a56562ed35b9a6f68d8

# No `MARCH`: there is no ahead-of-run code generator here to aim at an ISA. V8
# JITs the hot loop on the machine it is running on, and a build arg nothing reads
# is a knob that lies about being turnable. `JOBS` is accepted and unused -- V8 is
# not compiled here, and there is nothing to parallelise.
ARG JOBS

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads: a label the harness
# consumed would be a second source of truth, and two sources of truth drift.
# Nothing about the ISA is pinned here, so nothing about it is claimed here: the
# run reports what it actually got.
LABEL langbench.version="2.5.5"

# DENO_DIR is the *toolchain* cache, and it must be warm before the campaign
# starts: `deno check` on a module that imports `node:worker_threads` resolves the
# node typings from npm, and a measured run is `--network=none`. Warming it at
# image build -- where the network lives -- is the same rule that says to warm
# Rust's precompiled std and never Go's build cache: hot for the toolchain, cold
# for our code. The timed build below checks a *copy* in the tmpfs, so the typings
# are cached while the kernel is checked from scratch.
ENV DENO_DIR=/usr/local/share/deno \
    BUILD_DIR=/build

WORKDIR /usr/local/src/mandelbrot
COPY mandelbrot.mjs .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

# The `deno` image already ships a uid/gid 1000 user, named `deno`. It has to own
# the cache: `deno check` writes its results there, and the run phase must not be
# the first thing to discover the directory is read-only.
RUN chown -R deno:deno "${DENO_DIR}"

USER deno
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
