# Mandelbrot, Python, PyPy.
#
# The same `mandelbrot.py` as `python-cpython` and `python-cython`, byte for byte.
# Three Python rows, one source: CPython interprets it, Cython compiles it to a C
# extension, PyPy traces and JITs it. Same language, same program, three backends
# -- which is the experiment this repository exists to run.
#
# Single stage: the `build` subcommand recompiles source to bytecode to be timed,
# so the interpreter must survive into the runtime image.
FROM pypy:3.11-slim-trixie@sha256:bacc7d9d659b8ee561b286f79e02d4682d376bec9c6b0eebbd863f6735a1d1a6

# `MARCH` and `JOBS` are accepted and unused: PyPy is not compiled here, and
# bytecode compilation of one module has nothing to parallelise. Declaring them
# keeps this image a drop-in for the harness's build-arg contract.
ARG FP_MODE=strict
ARG MARCH
ARG JOBS

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads: a label the harness
# consumed would be a second source of truth, and two sources of truth drift.
LABEL langbench.version="3.11" \
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
