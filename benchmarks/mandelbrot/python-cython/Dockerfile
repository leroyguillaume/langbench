# Mandelbrot, Python, Cython.
#
# The same `mandelbrot.py` as `python-cpython`, byte for byte, compiled instead
# of interpreted. That is the clean experiment this project exists for: same
# source, different backend. Cython is the compiler here, exactly as gcc is for
# the C kernel -- not a library the kernel imports.
#
# Single stage: the `build` subcommand recompiles to be timed, so Cython and a C
# toolchain must survive into the runtime image.
FROM python:3.13-slim-trixie@sha256:eb43ff125d8d58d7449dcba7d336c23bcac412f526d861db493b9994d8010280

# The ISA target, and the whole of the mode: a pinned baseline, or the literal
# `native`. It reaches gcc verbatim -- there is a real C compiler under Cython, and it
# takes the harness's spelling as it stands. There is no floating-point arg: the
# generated C is compiled `-ffp-contract=off` whatever the mode, and always will be.
ARG MARCH
# Accepted and unused: `cythonize -j` parallelises across modules, and there is
# exactly one. Declaring it keeps this image a drop-in for the harness.
ARG JOBS

ARG CYTHON_VERSION=3.2.8
ARG SETUPTOOLS_VERSION=83.0.0

# gcc and libc6-dev are unpinned on purpose: pinning Debian point releases would
# break the build on every security update, and the toolchain version that
# matters is recorded in the label below.
# hadolint ignore=DL3008
RUN apt-get update \
 && apt-get install --no-install-recommends --yes gcc libc6-dev \
 && rm -rf /var/lib/apt/lists/*

# setuptools is not a kernel dependency: `cythonize` needs it to drive the C
# compiler, now that `distutils` has left the standard library.
RUN pip install --no-cache-dir \
      "cython==${CYTHON_VERSION}" \
      "setuptools==${SETUPTOOLS_VERSION}"

# Provenance of the *image*, for `docker inspect`. What this backend IS -- its
# language, its compiler, its interpreter, the modes it distinguishes -- lives
# in `bench.yaml`, which is the only thing the harness reads. A label the harness
# consumed would be a second source of truth, and two sources of truth drift.
LABEL langbench.version="3.2.8" \
      langbench.flags="-O3 -march=${MARCH} -ffp-contract=off"

ENV MARCH=${MARCH} \
    BUILD_DIR=/build

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
