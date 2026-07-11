# Mandelbrot, JavaScript, Bun (JavaScriptCore).
#
# The same `mandelbrot.mjs` as `js-nodejs` and `js-deno`, byte for byte -- but a
# different engine underneath. Node and Deno both run V8; Bun runs
# JavaScriptCore. That makes this the one JavaScript row where a difference is
# allowed to be about the compiler rather than about the runtime around it.
#
# Single stage: the `build` subcommand re-runs Bun's own ahead-of-run step to be
# timed, so the runtime must survive into the image.
FROM oven/bun:1.3-debian@sha256:9dba1a1b43ce28c9d7931bfc4eb00feb63b0114720a0277a8f939ae4dfc9db6f

# `MARCH` and `JOBS` are accepted and unused: JavaScriptCore is not compiled here,
# and there is nothing to parallelise. Declaring them keeps this image a drop-in
# for the harness's build-arg contract.
ARG FP_MODE=strict
ARG MARCH
ARG JOBS

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads: a label the harness
# consumed would be a second source of truth, and two sources of truth drift.
LABEL langbench.version="1.3" \
      langbench.flags="fp=${FP_MODE}"

ENV FP_MODE=${FP_MODE} \
    BUILD_DIR=/build

WORKDIR /usr/local/src/mandelbrot
COPY mandelbrot.mjs .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

# The `bun` image already ships a uid/gid 1000 user, named `bun`. Reusing it beats
# creating a second one at the same uid.
USER bun
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
