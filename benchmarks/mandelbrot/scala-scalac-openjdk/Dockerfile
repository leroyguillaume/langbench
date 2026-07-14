# Mandelbrot, Scala, scalac + HotSpot.
#
# The same JDK image as `java-javac-openjdk` and `kotlin-kotlinc-openjdk`, with the Scala 3
# compiler dropped on top: all three rows run on the *same* HotSpot, so whatever
# separates them is the compiler in front of it and the runtime library behind it.
# That is the controlled comparison, and it is the reason these three sit together.
#
# There is no official Scala image to pin a digest against, so the compiler is
# fetched from GitHub and verified against a pinned SHA-256 -- the same guarantee a
# digest gives us elsewhere. The download happens at `docker build`, where the
# network lives; every measured run is `--network=none`.
#
# Single stage on purpose: the `build` subcommand recompiles from a clean tree to
# be timed, so scalac must survive into the runtime image.
FROM eclipse-temurin:25-jdk-noble@sha256:3eb81ed94d8c1a34422f19f8188548bdf02cae69c91d0328afdbb7abed90f617

# No ISA build arg: scalac emits bytecode, which targets no CPU, and C2 emits the
# machine code during the run for whatever CPU is under it. `native` is this backend's
# only target, and there is nothing for a build arg to select.
ARG JOBS

# 3.3 is the LTS line. A benchmark that chases the newest minor release measures
# the release notes.
ARG SCALA_VERSION=3.3.8
ARG SCALA_SHA256=17a03dd9c2a790b4230a7d03b741cdedca6514096e50e7189bc5e2d299db250a

# hadolint ignore=DL3008
RUN apt-get update \
 && apt-get install --no-install-recommends --yes ca-certificates curl \
 && rm -rf /var/lib/apt/lists/*

# The archive is verified before it is unpacked, never after: an unpacked archive
# has already had its say about the filesystem.
RUN set -eu; \
    curl -fsSL "https://github.com/scala/scala3/releases/download/${SCALA_VERSION}/scala3-${SCALA_VERSION}.tar.gz" \
      -o /tmp/scala3.tar.gz; \
    printf '%s  /tmp/scala3.tar.gz\n' "${SCALA_SHA256}" > /tmp/scala3.sha256; \
    sha256sum --check --strict /tmp/scala3.sha256; \
    mkdir -p /usr/local/lib/scala3; \
    tar -xzf /tmp/scala3.tar.gz -C /usr/local/lib/scala3 --strip-components=1; \
    rm /tmp/scala3.tar.gz /tmp/scala3.sha256; \
    /usr/local/lib/scala3/bin/scalac -version

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads.
LABEL langbench.version="${SCALA_VERSION}" \
      langbench.flags="isa=native jit=c2"

ENV SCALA_HOME=/usr/local/lib/scala3 \
    CLASSES=/usr/local/lib/mandelbrot \
    BUILD_DIR=/build

WORKDIR /usr/local/src/mandelbrot
COPY Mandelbrot.scala .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

# scalac wants somewhere to put its own scratch state, and $HOME here belongs to
# root. Compiling as root at image build is fine; the run phase only reads.
RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

# The Ubuntu base already ships a uid/gid 1000 user, named `ubuntu`. Reusing it
# beats creating a second one at the same uid -- which is, in fact, an error.
USER ubuntu
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
