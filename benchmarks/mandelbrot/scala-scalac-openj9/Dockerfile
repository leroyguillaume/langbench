# Mandelbrot, Scala, scalac + Eclipse OpenJ9 (IBM Semeru).
#
# The same Mandelbrot.scala as every other Scala row, and the same scalac, on a
# different virtual machine: OpenJ9 rather than HotSpot. One cell of the
# (language x JVM) grid.
FROM ibm-semeru-runtimes:open-25-jdk@sha256:9ecdbb9e06828e391ed1dde149e6a834f4c1bdadcb5d95dcd3006045db72d760

# No ISA build arg: scalac emits bytecode, which targets no CPU, and OpenJ9's JIT
# emits the machine code during the run for whatever CPU is under it. `native` is this
# backend's only target, and there is nothing for a build arg to select.
ARG JOBS

# 3.3 is the LTS line. A benchmark that chases the newest minor release measures
# the release notes.
ARG SCALA_VERSION=3.3.8
ARG SCALA_SHA256=17a03dd9c2a790b4230a7d03b741cdedca6514096e50e7189bc5e2d299db250a

# hadolint ignore=DL3008
RUN apt-get update \
 && apt-get install --no-install-recommends --yes ca-certificates curl \
 && rm -rf /var/lib/apt/lists/*

# The archive is verified before it is unpacked, never after.
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
LABEL langbench.version="3.3.8" \
      langbench.flags="isa=native jvm=openj9"

ENV SCALA_HOME=/usr/local/lib/scala3 \
    CLASSES=/usr/local/lib/mandelbrot \
    BUILD_DIR=/build

WORKDIR /usr/local/src/mandelbrot
COPY Mandelbrot.scala .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

# The Ubuntu base already ships a uid/gid 1000 user, named `ubuntu`. Reusing it
# beats creating a second one at the same uid -- which is, in fact, an error.
USER ubuntu
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
