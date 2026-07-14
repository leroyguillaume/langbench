# Mandelbrot, Kotlin, kotlinc + Eclipse OpenJ9 (IBM Semeru).
#
# The same Mandelbrot.kt as every other Kotlin row, and the same kotlinc, on a
# different virtual machine: OpenJ9 rather than HotSpot. One cell of the
# (language x JVM) grid.
FROM ibm-semeru-runtimes:open-25-jdk@sha256:9ecdbb9e06828e391ed1dde149e6a834f4c1bdadcb5d95dcd3006045db72d760

# No ISA build arg: kotlinc emits bytecode, which targets no CPU, and OpenJ9's JIT
# emits the machine code during the run for whatever CPU is under it. `native` is this
# backend's only target, and there is nothing for a build arg to select.
ARG JOBS

ARG KOTLIN_VERSION=2.4.0
ARG KOTLIN_SHA256=ba1b9e6eb6ddc3275079224f2e9ea4a2b02eef7d59ce2d38404f04b22613c20a

# hadolint ignore=DL3008
RUN apt-get update \
 && apt-get install --no-install-recommends --yes ca-certificates curl unzip \
 && rm -rf /var/lib/apt/lists/*

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
      langbench.flags="isa=native jvm=openj9"

ENV KOTLIN_HOME=/usr/local/lib/kotlinc \
    CLASSES=/usr/local/lib/mandelbrot \
    BUILD_DIR=/build

WORKDIR /usr/local/src/mandelbrot
COPY Mandelbrot.kt .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

# The Ubuntu base already ships a uid/gid 1000 user, named `ubuntu`. Reusing it
# beats creating a second one at the same uid -- which is, in fact, an error.
USER ubuntu
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
