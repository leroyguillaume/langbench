# Mandelbrot, TypeScript, Deno (V8).
#
# The same `mandelbrot.mts` as `ts-nodejs` and `ts-bun`, byte for byte. TypeScript
# is Deno's native tongue -- it has run `.ts` without a build step since long
# before Node learned to strip types -- so this row is where that head start gets
# priced, against two runtimes that caught up later.
#
# Single stage: the `build` subcommand re-runs Deno's own ahead-of-run step to be
# timed, so the runtime must survive into the image.
FROM denoland/deno:debian-2.5.5@sha256:09cda54dca77d1e5b8af6176a8b6c29ecae0183dccf18a56562ed35b9a6f68d8

# `MARCH` and `JOBS` are accepted and unused: V8 is not compiled here, and there
# is nothing to parallelise. Declaring them keeps this image a drop-in for the
# harness's build-arg contract.
ARG FP_MODE=strict
ARG MARCH
ARG JOBS

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads: a label the harness
# consumed would be a second source of truth, and two sources of truth drift.
LABEL langbench.version="2.5.5" \
      langbench.flags="fp=${FP_MODE} type-stripping"

# DENO_DIR is the *toolchain* cache, and it must be warm before the campaign
# starts: `deno check` on a module that imports `node:worker_threads` resolves the
# node typings from npm, and a measured run is `--network=none`. Warming it at
# image build -- where the network lives -- is the same rule that says to warm
# Rust's precompiled std and never Go's build cache: hot for the toolchain, cold
# for our code. The timed build below checks a *copy* in the tmpfs, so the typings
# are cached while the kernel is checked from scratch.
ENV FP_MODE=${FP_MODE} \
    DENO_DIR=/usr/local/share/deno \
    BUILD_DIR=/build

WORKDIR /usr/local/src/mandelbrot
COPY mandelbrot.mts .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

# The `deno` image already ships a uid/gid 1000 user, named `deno`. It has to own
# the cache: `deno check` writes its results there, and the run phase must not be
# the first thing to discover the directory is read-only.
RUN chown -R deno:deno "${DENO_DIR}"

USER deno
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
