# Mandelbrot, Kotlin, kotlinc + HotSpot.
#
# The same JDK image as `java-javac-openjdk`, with the Kotlin compiler dropped on top: the
# two rows run on the *same* HotSpot, so whatever separates them is kotlinc and the
# Kotlin stdlib, and nothing else. That is the controlled comparison.
#
# There is no official Kotlin image to pin a digest against, so the compiler is
# fetched from GitHub and verified against a pinned SHA-256 -- the same guarantee a
# digest gives us elsewhere. The download happens at `docker build`, where the
# network lives; every measured run is `--network=none`.
#
# Single stage on purpose: the `build` subcommand recompiles from a clean tree to
# be timed, so kotlinc must survive into the runtime image.
FROM eclipse-temurin:25-jdk-noble@sha256:3eb81ed94d8c1a34422f19f8188548bdf02cae69c91d0328afdbb7abed90f617

ARG FP_MODE=strict
ARG MARCH=x86-64-v3
ARG JOBS

ARG KOTLIN_VERSION=2.4.0
ARG KOTLIN_SHA256=ba1b9e6eb6ddc3275079224f2e9ea4a2b02eef7d59ce2d38404f04b22613c20a

# curl and unzip to fetch the compiler; both are gone from the final image only in
# spirit, since this is a single stage -- but the apt lists are not.
# hadolint ignore=DL3008
RUN apt-get update \
 && apt-get install --no-install-recommends --yes ca-certificates curl unzip \
 && rm -rf /var/lib/apt/lists/*

# The archive is verified before it is unpacked, never after: an unpacked archive
# has already had its say about the filesystem.
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
LABEL langbench.version="${KOTLIN_VERSION}" \
      langbench.flags="march=${MARCH} fp=${FP_MODE}"

# The Kotlin stdlib is not a third-party dependency: it is the language's runtime,
# the way libstdc++ is C++'s. It has to be on the classpath at run time, and its
# loading cost is part of what this backend costs.
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

# The Ubuntu base already ships a uid/gid 1000 user, named `ubuntu`. Reusing it
# beats creating a second one at the same uid -- which is, in fact, an error.
USER ubuntu
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
