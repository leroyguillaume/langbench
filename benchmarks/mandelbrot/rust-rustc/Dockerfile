# Mandelbrot, Rust, rustc (LLVM).
#
# Single stage on purpose: the `build` subcommand recompiles from a clean tree to
# be timed, so the toolchain must survive into the runtime image.
#
# No cargo, and therefore no `target/` and no lockfile: the kernel is one file
# with no dependency, so `rustc` on it is the whole build. That also keeps the
# Build column honest -- what is timed is the compiler, not cargo's bookkeeping.
#
# Nothing to warm, either: unlike Go, Rust ships a precompiled standard library in
# the toolchain, so a cold build of our one file never recompiles std.
FROM rust:1.93-slim-trixie@sha256:c0a38f5662afdb298898da1d70b909af4bda4e0acff2dc52aea6360a9b9c6956

# The ISA target and the job count arrive as build args, so one Dockerfile covers
# both modes: `MARCH` is either the pinned baseline of the architecture or the
# literal `native`, and that choice *is* the mode. Floating-point is not a build
# arg -- it is strict in every mode, and in Rust it is strict without asking: there
# is no `-ffast-math` to pass and no flag to leave out.
ARG MARCH=x86-64-v3
ARG JOBS=1

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads: a label the harness
# consumed would be a second source of truth, and two sources of truth drift.
LABEL langbench.version="1.93" \
      langbench.flags="-C opt-level=3 -C codegen-units=1 march=${MARCH}"

# The entrypoint reads these at run time to rebuild with exactly the flags the
# shipped binary was compiled with.
ENV MARCH=${MARCH} \
    BUILD_DIR=/build

WORKDIR /usr/local/src/mandelbrot
COPY mandelbrot.rs .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

RUN groupadd --system --gid 1000 mandelbrot \
 && useradd --system --uid 1000 --gid mandelbrot \
      --home /usr/local/src/mandelbrot --shell /usr/sbin/nologin mandelbrot

USER mandelbrot
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
