# Mandelbrot, C++, Clang.
#
# The same `mandelbrot.cpp` as `cpp-gcc`, byte for byte, through the other C++
# compiler -- and, deliberately, against the *same* standard library: Debian's
# clang++ compiles against libstdc++, not libc++. So this row and cpp-gcc share a
# source, a distribution, a libc and an STL, and differ in the code generator
# alone. Swapping in libc++ would be a second variable and a different experiment.
#
# Single stage on purpose: the `build` subcommand recompiles from a clean tree to
# be timed, so the toolchain must survive into the runtime image.
FROM debian:trixie-slim@sha256:28de0877c2189802884ccd20f15ee41c203573bd87bb6b883f5f46362d24c5c2

# The ISA target and the job count arrive as build args, so one Dockerfile covers
# both modes: `MARCH` is either the pinned baseline of the architecture or the
# literal `native`, and that choice *is* the mode. Floating-point is not a build
# arg and never was a knob worth turning -- it is strict in every mode, and the
# entrypoint says so unconditionally.
ARG MARCH=x86-64-v3
ARG JOBS=1

# Pinned to a major version, not to `clang`: the unversioned metapackage follows
# whatever LLVM Debian promotes next, and a compiler that changes under the table
# is a benchmark that changes under the table.
ARG CLANG_VERSION=19
# The C++ standard library clang++ compiles against. Named explicitly for the
# same reason: it is half of what "C++" means here.
ARG LIBSTDCXX_VERSION=14

# binutils for `strip`, `size` and `objdump`: the build phase reports the shipped
# binary's sizes, and `disasm` reads its instructions.
# hadolint ignore=DL3008
RUN apt-get update \
 && apt-get install --no-install-recommends --yes \
      "clang-${CLANG_VERSION}" "libstdc++-${LIBSTDCXX_VERSION}-dev" binutils libc6-dev \
 && rm -rf /var/lib/apt/lists/*

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads: a label the harness
# consumed would be a second source of truth, and two sources of truth drift.
LABEL langbench.version="${CLANG_VERSION}" \
      langbench.flags="-O3 -std=c++20 -march=${MARCH} -ffp-contract=off libstdc++-${LIBSTDCXX_VERSION}"

# The entrypoint reads these at run time to rebuild with exactly the flags the
# shipped binary was compiled with.
ENV MARCH=${MARCH} \
    CLANG_VERSION=${CLANG_VERSION} \
    BUILD_DIR=/build

WORKDIR /usr/local/src/mandelbrot
COPY mandelbrot.cpp .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

RUN groupadd --system --gid 1000 mandelbrot \
 && useradd --system --uid 1000 --gid mandelbrot \
      --home /usr/local/src/mandelbrot --shell /usr/sbin/nologin mandelbrot

USER mandelbrot
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
