# Mandelbrot, Java, javac + HotSpot.
#
# Single stage on purpose: the `build` subcommand recompiles from a clean tree to
# be timed, so the toolchain must survive into the runtime image. A JDK is both
# halves anyway -- javac compiles, java runs -- which is why this backend declares
# a compiler *and* an interpreter, like python-cython does.
#
# No Maven, no Gradle, no jar: the kernel is one file with no dependency, so
# `javac` on it is the whole build. That also keeps the Build column honest -- what
# is timed is the compiler, not a build tool's bookkeeping.
FROM eclipse-temurin:25-jdk-noble@sha256:3eb81ed94d8c1a34422f19f8188548bdf02cae69c91d0328afdbb7abed90f617

# Floating-point semantics, ISA baseline and job count arrive as build args, so
# one Dockerfile covers every mode the backend distinguishes -- which, here, is
# `strict` alone. See `bench.yaml`.
ARG FP_MODE=strict
ARG MARCH=x86-64-v3
ARG JOBS

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads: a label the harness
# consumed would be a second source of truth, and two sources of truth drift.
LABEL langbench.version="25" \
      langbench.flags="march=${MARCH} fp=${FP_MODE}"

ENV FP_MODE=${FP_MODE} \
    MARCH=${MARCH} \
    CLASSES=/usr/local/lib/mandelbrot \
    BUILD_DIR=/build

WORKDIR /usr/local/src/mandelbrot
COPY Mandelbrot.java .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

# The Ubuntu base already ships a uid/gid 1000 user, named `ubuntu`. Reusing it
# beats creating a second one at the same uid -- which is, in fact, an error.
USER ubuntu
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
