# Mandelbrot, Julia.
#
# Single stage: the `build` subcommand re-runs Julia's ahead-of-run compile step to
# be timed, so the runtime must survive into the image. There is no dependency to
# fetch -- the kernel imports nothing -- and therefore no depot to populate.
#
# Julia is the awkward case for this table's vocabulary, and the interesting one.
# It compiles: the hot loop becomes native code through LLVM, exactly as the C
# kernel does. But it compiles *on first call*, during the run, so there is nothing
# compiled ahead of the run and nothing to ship but source. The manifest therefore
# declares an interpreter and no compiler -- see `bench.yaml`.
FROM julia:1.11-bookworm@sha256:7e1ff970295929bfc547b6b0b7dab339320cfd1b52394efdea4f928502fecd26

# `JOBS` is accepted and unused: there is one method to compile, and Julia's
# codegen for it is not parallel.
ARG FP_MODE=strict
ARG MARCH=x86-64-v3
ARG JOBS

# Provenance of the *image*, for `docker inspect`. What this backend IS lives in
# `bench.yaml`, which is the only thing the harness reads: a label the harness
# consumed would be a second source of truth, and two sources of truth drift.
LABEL langbench.version="1.11" \
      langbench.flags="--cpu-target=${MARCH} fp=${FP_MODE}"

# JULIA_DEPOT_PATH out of `$HOME`: the depot is where Julia writes its compilation
# cache, and a home directory it cannot write to would fail the run rather than the
# build. Nothing is fetched into it -- there are no packages -- so it stays empty
# and the network stays unused, which is what `--network=none` will enforce anyway.
ENV FP_MODE=${FP_MODE} \
    MARCH=${MARCH} \
    JULIA_DEPOT_PATH=/usr/local/share/julia-depot \
    BUILD_DIR=/build

WORKDIR /usr/local/src/mandelbrot
COPY mandelbrot.jl .
COPY entrypoint.sh /usr/local/bin/entrypoint.sh

RUN chmod 0755 /usr/local/bin/entrypoint.sh \
 && /usr/local/bin/entrypoint.sh install

RUN groupadd --system --gid 1000 mandelbrot \
 && useradd --system --uid 1000 --gid mandelbrot \
      --home /usr/local/src/mandelbrot --shell /usr/sbin/nologin mandelbrot \
 && mkdir -p "${JULIA_DEPOT_PATH}" \
 && chown -R mandelbrot:mandelbrot "${JULIA_DEPOT_PATH}"

USER mandelbrot
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
