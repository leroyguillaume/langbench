# Mandelbrot, C, Clang.
#
# The same `mandelbrot.c` as `c-gcc`, byte for byte, through the other C compiler.
# That is the whole point of the row: one source, two code generators, and the
# difference in the table is the difference between them.
#
# Single stage on purpose: the `build` subcommand recompiles from a clean tree to
# be timed, so the toolchain must survive into the runtime image.
#
# Debian rather than an LLVM image: there is no official Clang image to pin a
# digest against, and `gcc:15-trixie` -- the reference row's base -- is Debian
# trixie underneath. Same distribution, same libc, same linker: the two C rows
# differ in the compiler and in nothing else, which is what a controlled
# comparison means.
FROM debian:trixie-slim@sha256:28de0877c2189802884ccd20f15ee41c203573bd87bb6b883f5f46362d24c5c2

# Floating-point semantics, ISA baseline and job count arrive as build args, so
# one Dockerfile covers all three modes. `MARCH` is never `native`.
ARG FP_MODE=strict
ARG MARCH=x86-64-v3
ARG JOBS=1

# Pinned to a major version, not to `clang`: the unversioned metapackage follows
# whatever LLVM Debian promotes next, and a compiler that changes under the table
# is a benchmark that changes under the table.
ARG CLANG_VERSION=19

# binutils for `strip`, `size` and `objdump`: the build phase reports the shipped
# binary's sizes, and `disasm` reads its instructions. libc6-dev for the headers
# and the C runtime; clang does not ship them.
# hadolint ignore=DL3008
RUN apt-get update \
 && apt-get install --no-install-recommends --yes \
      "clang-${CLANG_VERSION}" binutils libc6-dev \
 && rm -rf /var/lib/apt/lists/*

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads: a label the harness
# consumed would be a second source of truth, and two sources of truth drift.
LABEL langbench.version="${CLANG_VERSION}" \
      langbench.flags="-O3 -march=${MARCH} fp=${FP_MODE}"

# The entrypoint reads these at run time to rebuild with exactly the flags the
# shipped binary was compiled with.
ENV FP_MODE=${FP_MODE} \
    MARCH=${MARCH} \
    CLANG_VERSION=${CLANG_VERSION} \
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
