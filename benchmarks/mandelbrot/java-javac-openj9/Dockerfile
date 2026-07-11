# Mandelbrot, Java, javac + Eclipse OpenJ9 (IBM Semeru).
#
# The same `Mandelbrot.java` as `java-javac-openjdk`, byte for byte, and the same
# javac in front of it -- but not the same JVM. OpenJ9 is a different virtual
# machine entirely: a different JIT (Testarossa), a different garbage collector, a
# different object layout. HotSpot is not "the JVM"; it is *a* JVM, and this row is
# what makes that visible.
#
# Where it should show up is Startup, which OpenJ9 is built to keep small. That is
# the column this row exists to move.
#
# Single stage: the `build` subcommand recompiles from a clean tree to be timed, so
# javac must survive into the runtime image.
FROM ibm-semeru-runtimes:open-25-jdk@sha256:9ecdbb9e06828e391ed1dde149e6a834f4c1bdadcb5d95dcd3006045db72d760

ARG FP_MODE=strict
ARG MARCH=x86-64-v3
ARG JOBS

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads.
#
# `march=none` is not an oversight: OpenJ9 cannot pin an ISA baseline at all, and
# the label says so rather than repeating a MARCH this backend never honoured.
LABEL langbench.version="25" \
      langbench.flags="march=none fp=${FP_MODE} jvm=openj9"

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
