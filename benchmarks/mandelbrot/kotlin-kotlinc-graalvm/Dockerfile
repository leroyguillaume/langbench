# Mandelbrot, Kotlin, kotlinc + GraalVM (Graal JIT).
#
# The same Mandelbrot.kt as every other Kotlin row, and the same kotlinc. What
# changes is the JIT: Graal replaces C2 through JVMCI. Together with
# java-javac-graalvm and scala-scalac-graalvm this fills in one column of the
# (language x JVM) grid, and the interesting question it answers is whether the
# JIT and the language interact at all -- or whether, as the bytecode suggests,
# they are simply orthogonal.
FROM ghcr.io/graalvm/jdk-community:25@sha256:a3bdba6a313cddc89f7e1b2649140e898c68ba356994a9c317b96b01bcb9f982

ARG FP_MODE=strict
ARG MARCH=x86-64-v3
ARG JOBS

ARG KOTLIN_VERSION=2.4.0
ARG KOTLIN_SHA256=ba1b9e6eb6ddc3275079224f2e9ea4a2b02eef7d59ce2d38404f04b22613c20a

# Oracle Linux, so microdnf rather than apt -- and the base ships no unzip.
#
# unzip is unpinned on purpose, exactly as the apt packages elsewhere are: pinning
# a distro point release breaks the build on every security update, and the version
# that actually matters -- the compiler's -- is pinned by SHA-256 below.
# hadolint ignore=DL3041
RUN microdnf install -y unzip \
 && microdnf clean all

# The archive is verified before it is unpacked, never after.
RUN set -eu; \
    curl -fsSL "https://github.com/JetBrains/kotlin/releases/download/v${KOTLIN_VERSION}/kotlin-compiler-${KOTLIN_VERSION}.zip" \
      -o /tmp/kotlin.zip; \
    printf '%s  /tmp/kotlin.zip\n' "${KOTLIN_SHA256}" > /tmp/kotlin.sha256; \
    sha256sum --check --strict /tmp/kotlin.sha256; \
    unzip -q /tmp/kotlin.zip -d /usr/local/lib; \
    rm /tmp/kotlin.zip /tmp/kotlin.sha256; \
    /usr/local/lib/kotlinc/bin/kotlinc -version

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads.
LABEL langbench.version="2.4.0" \
      langbench.flags="march=${MARCH} fp=${FP_MODE} jit=graal"

ENV FP_MODE=${FP_MODE} \
    MARCH=${MARCH} \
    KOTLIN_HOME=/usr/local/lib/kotlinc \
    CLASSES=/usr/local/lib/mandelbrot \
    BUILD_DIR=/build

WORKDIR /usr/local/src/mandelbrot
COPY Mandelbrot.kt .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

RUN groupadd --system --gid 1000 mandelbrot \
 && useradd --system --uid 1000 --gid mandelbrot \
      --home /usr/local/src/mandelbrot --shell /usr/sbin/nologin mandelbrot

USER mandelbrot
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
