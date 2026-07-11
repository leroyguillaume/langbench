# Mandelbrot, C, GCC.
#
# Single stage on purpose: the `build` subcommand recompiles from a clean tree to
# be timed, so the toolchain must survive into the runtime image. There is no
# toolchain cache to warm -- C ships no precompiled standard library the way Go
# does -- and no dependency to fetch, because the kernel has none.
FROM gcc:15-trixie@sha256:a80e634009e139c140c5a4149c2c4069e05d78c5d049576ba23f3c0b7608cd73

# Floating-point semantics, ISA baseline and job count arrive as build args, so
# one Dockerfile covers all three modes. `MARCH` is never `native`.
ARG FP_MODE=strict
ARG MARCH=x86-64-v3
ARG JOBS=1

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads: a label the harness
# consumed would be a second source of truth, and two sources of truth drift.
LABEL langbench.version="15" \
      langbench.flags="-O3 -march=${MARCH} fp=${FP_MODE}"

# The entrypoint reads these at run time to rebuild with exactly the flags the
# shipped binary was compiled with.
ENV FP_MODE=${FP_MODE} \
    MARCH=${MARCH} \
    BUILD_DIR=/build

WORKDIR /usr/local/src/mandelbrot
COPY mandelbrot.c .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

RUN groupadd --system --gid 1000 mandelbrot \
 && useradd --system --uid 1000 --gid mandelbrot \
      --home /usr/local/src/mandelbrot --shell /usr/sbin/nologin mandelbrot

USER mandelbrot
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
