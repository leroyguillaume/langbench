# Mandelbrot, TypeScript, Node.js (V8).
#
# No `tsc`, and no build step that emits JavaScript: Node 24 strips the types
# itself and runs what is left. That is why this backend declares no compiler --
# nothing is compiled ahead of the run, the annotations are simply erased -- and
# it is the whole reason the row exists. "TypeScript is slower than JavaScript" is
# a claim people make; this row and js-nodejs differ in nothing but the types, so
# the table can answer it.
#
# Single stage: the `build` subcommand re-runs Node's own ahead-of-run step to be
# timed, so the runtime must survive into the image. There is deliberately no
# package.json and no tsconfig.json: the kernel is a `.mts`, which Node, Deno and
# Bun each load as a TypeScript ES module without being told to -- which is what
# lets the three ts-* rows share one kernel, byte for byte.
FROM node:24-trixie-slim@sha256:366fdef91728b1b7fa18c84fba63b6e79ed77b7e10cc206878e9705da4d7b169

# No `MARCH`: there is no ahead-of-run code generator here to aim at an ISA. V8
# JITs the hot loop on the machine it is running on, and a build arg nothing reads
# is a knob that lies about being turnable. `JOBS` is accepted and unused -- V8 is
# not compiled here, and there is nothing to parallelise.
ARG JOBS

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads: a label the harness
# consumed would be a second source of truth, and two sources of truth drift.
# `type-stripping` is what is left worth saying about this artefact: nothing about
# the ISA is pinned here, so nothing about it is claimed here -- the run reports
# what it actually got.
LABEL langbench.version="24" \
      langbench.flags="type-stripping"

ENV BUILD_DIR=/build

WORKDIR /usr/local/src/mandelbrot
COPY mandelbrot.mts .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

# The `node` image already ships a uid/gid 1000 user, named `node` rather than
# `mandelbrot`. Reusing it beats creating a second one at the same uid.
USER node
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
