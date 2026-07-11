# Mandelbrot, Zig.
#
# There is no official Zig image to pin a digest against, so the toolchain is
# fetched from ziglang.org and verified against a pinned SHA-256 -- which is a
# stronger guarantee than a tag, and the same one a digest gives us elsewhere.
# The download happens at `docker build`, where the network lives; every measured
# run is `--network=none`.
#
# Single stage on purpose: the `build` subcommand recompiles from a clean tree to
# be timed, so the toolchain must survive into the runtime image.
FROM debian:trixie-slim@sha256:28de0877c2189802884ccd20f15ee41c203573bd87bb6b883f5f46362d24c5c2

# Floating-point semantics, ISA baseline and job count arrive as build args, so
# one Dockerfile covers every mode the backend distinguishes -- which, here, is
# `strict` alone. See `bench.yaml`.
ARG FP_MODE=strict
ARG MARCH=x86-64-v3
ARG JOBS=1

# 0.15.2 and not 0.16: Zig's standard library is still moving underneath its
# users, and 0.16 rewrote the I/O and process APIs the kernel calls. Pinning the
# previous stable buys a kernel that reads like Zig instead of like a migration.
ARG ZIG_VERSION=0.15.2
ARG ZIG_SHA256_X86_64=02aa270f183da276e5b5920b1dac44a63f1a49e55050ebde3aecc9eb82f93239
ARG ZIG_SHA256_AARCH64=958ed7d1e00d0ea76590d27666efbf7a932281b3d7ba0c6b01b0ff26498f667f
# Set by BuildKit, never by us: amd64 or arm64. The one place the harness's ISA
# rule meets the image, and it must never be guessed from the build host.
ARG TARGETARCH

# ca-certificates, curl and xz-utils to fetch the toolchain; binutils for `strip`,
# `size` and `objdump`, which the build phase and `disasm` need.
# hadolint ignore=DL3008
RUN apt-get update \
 && apt-get install --no-install-recommends --yes ca-certificates curl xz-utils binutils \
 && rm -rf /var/lib/apt/lists/*

# The tarball is verified before it is unpacked, never after: an unpacked archive
# has already had its say about the filesystem.
RUN set -eu; \
    case "${TARGETARCH}" in \
      amd64) arch=x86_64; sha=${ZIG_SHA256_X86_64} ;; \
      arm64) arch=aarch64; sha=${ZIG_SHA256_AARCH64} ;; \
      *) echo "unsupported TARGETARCH: ${TARGETARCH}" >&2; exit 1 ;; \
    esac; \
    tarball="zig-${arch}-linux-${ZIG_VERSION}.tar.xz"; \
    curl -fsSL "https://ziglang.org/download/${ZIG_VERSION}/${tarball}" -o /tmp/zig.tar.xz; \
    printf '%s  /tmp/zig.tar.xz\n' "${sha}" > /tmp/zig.sha256; \
    sha256sum --check --strict /tmp/zig.sha256; \
    mkdir -p /usr/local/lib/zig; \
    tar -xJf /tmp/zig.tar.xz -C /usr/local/lib/zig --strip-components=1; \
    rm /tmp/zig.tar.xz /tmp/zig.sha256; \
    ln -s /usr/local/lib/zig/zig /usr/local/bin/zig; \
    zig version

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads: a label the harness
# consumed would be a second source of truth, and two sources of truth drift.
LABEL langbench.version="${ZIG_VERSION}" \
      langbench.flags="-OReleaseFast mcpu=${MARCH} fp=${FP_MODE}"

# ZIG_GLOBAL_CACHE_DIR is the *toolchain* cache: compiler_rt and the pieces of the
# standard library this program pulls in. Zig ships no precompiled std, so a cold
# global cache would make the timed rebuild "Zig compiles its own runtime" rather
# than "Zig compiles our kernel" -- the Go trap, in another accent. It is warmed at
# image build below and reused by every run. The *local* cache, which holds our
# module, stays cold: the timed build points it at the tmpfs.
ENV FP_MODE=${FP_MODE} \
    MARCH=${MARCH} \
    ZIG_GLOBAL_CACHE_DIR=/usr/local/share/zig-cache \
    BUILD_DIR=/build

WORKDIR /usr/local/src/mandelbrot
COPY mandelbrot.zig .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

RUN groupadd --system --gid 1000 mandelbrot \
 && useradd --system --uid 1000 --gid mandelbrot \
      --home /usr/local/src/mandelbrot --shell /usr/sbin/nologin mandelbrot \
 && chown -R mandelbrot:mandelbrot "${ZIG_GLOBAL_CACHE_DIR}"

USER mandelbrot
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
