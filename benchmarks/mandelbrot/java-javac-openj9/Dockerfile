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

# No ISA build arg: Testarossa emits its machine code during the run and takes the
# host CPU as it finds it, so `native` is this backend's only target and there is
# nothing for a build arg to select.
ARG JOBS

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads.
LABEL langbench.version="25" \
      langbench.flags="isa=native jvm=openj9"

ENV CLASSES=/usr/local/lib/mandelbrot \
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
