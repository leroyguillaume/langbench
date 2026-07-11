# Mandelbrot, Python, CPython.
#
# Single stage: the `build` subcommand recompiles source to bytecode to be timed,
# so the interpreter must survive into the runtime image. There is no dependency
# to fetch -- the kernel has none -- and no toolchain cache to warm.
#
# 3.13 and not 3.14: CPython 3.14 changes the default `multiprocessing` start
# method on Linux, and the start method is part of what this benchmark measures.
# The kernel pins `fork` explicitly, but the base image stays where the default
# and the pin agree.
FROM python:3.13-slim-trixie@sha256:eb43ff125d8d58d7449dcba7d336c23bcac412f526d861db493b9994d8010280

# `MARCH` and `JOBS` are accepted and unused: CPython is not compiled here, and
# bytecode compilation of one module has nothing to parallelise. Declaring them
# keeps this image a drop-in for the harness's build-arg contract.
ARG FP_MODE=strict
ARG MARCH
ARG JOBS

# `fp_modes` declares that only `strict` is meaningful: the interpreter has one
# floating-point semantics, so `fma` and `fast` are the same run under another
# name. The harness reads it out of this file (not from `docker inspect`: it
# decides what to build) and skips the other two modes, loudly.
LABEL langbench.language="python" \
      langbench.compiler="cpython" \
      langbench.version="3.13" \
      langbench.fp_modes="strict" \
      langbench.flags="fp=${FP_MODE}"

ENV FP_MODE=${FP_MODE} \
    BUILD_DIR=/build \
    PYTHONDONTWRITEBYTECODE=

WORKDIR /usr/local/src/mandelbrot
COPY mandelbrot.py .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

RUN groupadd --system --gid 1000 mandelbrot \
 && useradd --system --uid 1000 --gid mandelbrot \
      --home /usr/local/src/mandelbrot --shell /usr/sbin/nologin mandelbrot

USER mandelbrot
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
