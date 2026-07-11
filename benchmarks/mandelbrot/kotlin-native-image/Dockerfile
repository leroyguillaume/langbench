# Mandelbrot, Kotlin, GraalVM native-image (AOT).
#
# The same Mandelbrot.kt as every other Kotlin row -- and no JVM at run time.
# kotlinc emits bytecode, then native-image performs a whole-program analysis and
# links the result, *the Kotlin standard library included*, into a standalone ELF.
#
# That last part is why this row is not a rerun of java-native-image. Compute will
# tie -- it is the same loop -- but AOT has to swallow the language's runtime, and
# what that costs shows up in the Build and Binary columns. It is a fact you cannot
# get from any JIT row.
FROM ghcr.io/graalvm/native-image-community:25@sha256:0d936f32bb8acb5bc60c41b33e05f064d7a6aaf36b726538296c54949bd4a3c0

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
      langbench.flags="-O3 -march=${MARCH} fp=${FP_MODE} aot"

ENV FP_MODE=${FP_MODE} \
    MARCH=${MARCH} \
    KOTLIN_HOME=/usr/local/lib/kotlinc \
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
