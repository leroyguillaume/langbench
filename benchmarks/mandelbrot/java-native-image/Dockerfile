# Mandelbrot, Java, GraalVM native-image (AOT).
#
# The same `Mandelbrot.java` as every other Java row, byte for byte -- and no JVM at
# run time. native-image runs Graal ahead of time: it performs a whole-program
# points-to analysis, compiles everything it can reach, links in a garbage collector
# and a threading runtime, and emits an ELF. `run` executes a binary, not a class.
#
# That is what makes this the most interesting row in the JVM group. It completes
# the same triangle Python already has -- one source, an interpreter (openjdk), a
# JIT (graalvm), and an AOT compiler (this) -- and it is the only JVM backend that
# can honour the project's ISA rule, because it is the only one that compiles before
# it meets the machine.
#
# Single stage: the `build` subcommand recompiles from a clean tree to be timed, so
# javac *and* native-image must survive into the runtime image.
FROM ghcr.io/graalvm/native-image-community:25@sha256:0d936f32bb8acb5bc60c41b33e05f064d7a6aaf36b726538296c54949bd4a3c0

ARG FP_MODE=strict
ARG MARCH=x86-64-v3
ARG JOBS

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads.
LABEL langbench.version="25" \
      langbench.flags="-O3 -march=${MARCH} fp=${FP_MODE} aot"

ENV FP_MODE=${FP_MODE} \
    MARCH=${MARCH} \
    BUILD_DIR=/build

WORKDIR /usr/local/src/mandelbrot
COPY Mandelbrot.java .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

# The base image's ENTRYPOINT is `native-image` itself; ours replaces it below.
RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

RUN groupadd --system --gid 1000 mandelbrot \
 && useradd --system --uid 1000 --gid mandelbrot \
      --home /usr/local/src/mandelbrot --shell /usr/sbin/nologin mandelbrot

USER mandelbrot
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
