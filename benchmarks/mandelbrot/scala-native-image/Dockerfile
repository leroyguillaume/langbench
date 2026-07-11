# Mandelbrot, Scala, GraalVM native-image (AOT).
#
# The same Mandelbrot.scala as every other Scala row -- and no JVM at run time.
# scalac emits bytecode, then native-image links it, *the Scala runtime included*,
# into a standalone ELF. Scala brings the heaviest runtime in this table: the 3.x
# library plus the 2.13 one it still stands on. What it costs to AOT-compile all of
# that, and to ship it, is exactly what the Build and Binary columns are for.
#
# Not to be confused with scala-scala-native, which is Scala Native (LLVM) -- a
# different compiler entirely. This one is Graal, ahead of time, over JVM bytecode.
FROM ghcr.io/graalvm/native-image-community:25@sha256:0d936f32bb8acb5bc60c41b33e05f064d7a6aaf36b726538296c54949bd4a3c0

ARG FP_MODE=strict
ARG MARCH=x86-64-v3
ARG JOBS

# 3.3 is the LTS line. A benchmark that chases the newest minor release measures
# the release notes.
ARG SCALA_VERSION=3.3.8
ARG SCALA_SHA256=17a03dd9c2a790b4230a7d03b741cdedca6514096e50e7189bc5e2d299db250a

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
      langbench.flags="-O3 -march=${MARCH} fp=${FP_MODE} aot"

ENV FP_MODE=${FP_MODE} \
    MARCH=${MARCH} \
    SCALA_HOME=/usr/local/lib/scala3 \
    BUILD_DIR=/build

WORKDIR /usr/local/src/mandelbrot
COPY Mandelbrot.scala .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

RUN groupadd --system --gid 1000 mandelbrot \
 && useradd --system --uid 1000 --gid mandelbrot \
      --home /usr/local/src/mandelbrot --shell /usr/sbin/nologin mandelbrot

USER mandelbrot
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
