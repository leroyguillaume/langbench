# Mandelbrot, Java, javac + GraalVM (Graal JIT).
#
# The same `Mandelbrot.java` as `java-javac-openjdk`, byte for byte, and the same
# javac in front of it. What changes is the *JIT*: GraalVM plugs the Graal compiler
# into HotSpot through JVMCI, in place of C2. Same bytecode, same VM, same GC --
# a different code generator turning the hot loop into machine code while it runs.
# That is as clean an A/B on a JIT as this table can construct.
#
# Single stage: the `build` subcommand recompiles from a clean tree to be timed,
# so javac must survive into the runtime image.
FROM ghcr.io/graalvm/jdk-community:25@sha256:a3bdba6a313cddc89f7e1b2649140e898c68ba356994a9c317b96b01bcb9f982

# No ISA build arg: Graal emits its machine code during the run and takes the host
# CPU as it finds it, so `native` is this backend's only target and there is nothing
# for a build arg to select.
ARG JOBS

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads.
LABEL langbench.version="25" \
      langbench.flags="isa=native jit=graal"

ENV CLASSES=/usr/local/lib/mandelbrot \
    BUILD_DIR=/build

WORKDIR /usr/local/src/mandelbrot
COPY Mandelbrot.java .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

RUN groupadd --system --gid 1000 mandelbrot \
 && useradd --system --uid 1000 --gid mandelbrot \
      --home /usr/local/src/mandelbrot --shell /usr/sbin/nologin mandelbrot

USER mandelbrot
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
