# Mandelbrot, Scala, Scala Native (LLVM).
#
# The same `Mandelbrot.scala` as every other Scala row, byte for byte -- and no JVM
# anywhere. Scala Native compiles Scala to its own IR, hands that to LLVM, and links
# an ELF against a reimplemented subset of the JDK. The kernel needs no adjusting to
# get there: Scala Native's javalib provides `java.lang.Thread` and
# `AtomicInteger`, so the JVM kernel simply compiles.
#
# This is the row scala-native-image is *not*. That one is Graal AOT-compiling JVM
# bytecode; this one never produces bytecode at all. Different compiler, different
# runtime, same source -- which is the experiment this repository exists to run.
#
# Single stage: the `build` subcommand recompiles from a clean tree to be timed, so
# the toolchain must survive into the runtime image.
FROM eclipse-temurin:25-jdk-noble@sha256:3eb81ed94d8c1a34422f19f8188548bdf02cae69c91d0328afdbb7abed90f617

ARG FP_MODE=strict
ARG MARCH=x86-64-v3
ARG JOBS

# Pinned, and verified against a SHA-256 rather than a tag.
ARG SCALA_CLI_VERSION=1.15.0
ARG SCALA_CLI_SHA256_X86_64=37b33939ff82a8730263b6c72d81e23db4987b4b399c8a403c8ece76604a30f6
ARG SCALA_CLI_SHA256_AARCH64=e77e2f9bfc933887815e4c4d19f58eacf7a611cd5f802021048c5c6c851440c9
# The same Scala the scalac rows use. A native backend on a different language
# version would be two variables at once.
ARG SCALA_VERSION=3.3.8
# Set by BuildKit, never by us: amd64 or arm64.
ARG TARGETARCH

# clang and the Boehm GC are Scala Native's actual backend: it emits LLVM IR, clang
# compiles it, and the binary carries a real garbage collector. libunwind is what
# its exceptions unwind through.
# hadolint ignore=DL3008
RUN apt-get update \
 && apt-get install --no-install-recommends --yes \
      ca-certificates curl clang libgc-dev libunwind-dev zlib1g-dev \
 && rm -rf /var/lib/apt/lists/*

RUN set -eu; \
    case "${TARGETARCH}" in \
      amd64) arch=x86_64; sha=${SCALA_CLI_SHA256_X86_64} ;; \
      arm64) arch=aarch64; sha=${SCALA_CLI_SHA256_AARCH64} ;; \
      *) echo "unsupported TARGETARCH: ${TARGETARCH}" >&2; exit 1 ;; \
    esac; \
    curl -fsSL "https://github.com/VirtusLab/scala-cli/releases/download/v${SCALA_CLI_VERSION}/scala-cli-${arch}-pc-linux.gz" \
      -o /tmp/scala-cli.gz; \
    printf '%s  /tmp/scala-cli.gz\n' "${sha}" > /tmp/scala-cli.sha256; \
    sha256sum --check --strict /tmp/scala-cli.sha256; \
    gunzip -c /tmp/scala-cli.gz > /usr/local/bin/scala-cli; \
    chmod 0755 /usr/local/bin/scala-cli; \
    rm /tmp/scala-cli.gz /tmp/scala-cli.sha256

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads.
LABEL langbench.version="${SCALA_VERSION}" \
      langbench.flags="release-fast -march=${MARCH} fp=${FP_MODE} llvm"

# COURSIER_CACHE is the *toolchain* cache: the Scala Native runtime jars, the
# compiler plugin, the standard library. It is warmed at image build -- where the
# network lives -- because a measured run is `--network=none` and would otherwise
# fail trying to reach Maven Central. The *project* cache stays cold: the timed
# build compiles our kernel from scratch in the tmpfs. Hot for the toolchain, cold
# for our code, exactly as for Go and Deno.
ENV FP_MODE=${FP_MODE} \
    MARCH=${MARCH} \
    SCALA_VERSION=${SCALA_VERSION} \
    COURSIER_CACHE=/usr/local/share/coursier \
    BUILD_DIR=/build

WORKDIR /usr/local/src/mandelbrot
COPY Mandelbrot.scala .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

# The Ubuntu base already ships a uid/gid 1000 user, named `ubuntu`. It has to own
# the coursier cache and its own home: scala-cli writes both, and the run phase must
# not be the first thing to discover they are read-only.
RUN chown -R ubuntu:ubuntu "${COURSIER_CACHE}" /home/ubuntu

USER ubuntu
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
