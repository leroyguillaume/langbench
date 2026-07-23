# Mandelbrot, TypeScript, Bun (JavaScriptCore).
#
# The same `mandelbrot.mts` as `ts-nodejs` and `ts-deno`, byte for byte -- but a
# different engine underneath. Node and Deno both run V8; Bun runs
# JavaScriptCore. That makes this the one TypeScript row where a difference is
# allowed to be about the compiler rather than about the runtime around it.
#
# Single stage: the `build` subcommand re-runs Bun's own ahead-of-run step to be
# timed, so the runtime must survive into the image.
FROM oven/bun:1.3-debian@sha256:9dba1a1b43ce28c9d7931bfc4eb00feb63b0114720a0277a8f939ae4dfc9db6f

# No `MARCH`: there is no ahead-of-run code generator here to aim at an ISA.
# JavaScriptCore JITs the hot loop on the machine it is running on, and a build arg
# nothing reads is a knob that lies about being turnable. `JOBS` is accepted and
# unused -- JavaScriptCore is not compiled here, and there is nothing to
# parallelise.
ARG JOBS

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads: a label the harness
# consumed would be a second source of truth, and two sources of truth drift.
# `type-stripping` is what is left worth saying about this artefact: nothing about
# the ISA is pinned here, so nothing about it is claimed here -- the run reports
# what it actually got.
LABEL langbench.version="1.3" \
      langbench.flags="type-stripping"

ENV BUILD_DIR=/build

WORKDIR /usr/local/src/mandelbrot
COPY mandelbrot.mts .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

# The `bun` image already ships a uid/gid 1000 user, named `bun`. Reusing it beats
# creating a second one at the same uid.
USER bun
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
