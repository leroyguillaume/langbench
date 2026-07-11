# langbench orchestrates containers, so the runtime image needs a Docker client
# and a socket to talk to. It never runs the workload itself: the benchmark
# containers are siblings on the host daemon, not children of this one, so this
# image sits outside the measured path.

# ---------------------------------------------------------------- docker CLI --
# The static build, not the distro package: it is one file, version-pinned, and
# glibc-independent.
FROM debian:trixie-slim@sha256:28de0877c2189802884ccd20f15ee41c203573bd87bb6b883f5f46362d24c5c2 AS docker-cli

ARG DOCKER_VERSION=29.4.0
ARG TARGETARCH

# curl and ca-certificates are unpinned on purpose: this stage is discarded, and
# pinning Debian point releases here would break the build on every security
# update without buying any reproducibility in the final image.
# hadolint ignore=DL3008
RUN apt-get update \
 && apt-get install --no-install-recommends --yes ca-certificates curl \
 && rm -rf /var/lib/apt/lists/*

RUN case "${TARGETARCH}" in \
      amd64) arch=x86_64 ;; \
      arm64) arch=aarch64 ;; \
      *) echo "unsupported architecture: ${TARGETARCH}" >&2; exit 1 ;; \
    esac \
 && curl --fail --silent --show-error --location --output /tmp/docker.tgz \
      "https://download.docker.com/linux/static/stable/${arch}/docker-${DOCKER_VERSION}.tgz" \
 && tar --extract --gzip --file /tmp/docker.tgz --directory /tmp docker/docker \
 && install -m 0755 /tmp/docker/docker /usr/local/bin/docker

# -------------------------------------------------------------------- chef ----
FROM rust:1.94-slim-trixie@sha256:cf09adf8c3ebaba10779e5c23ff7fe4df4cccdab8a91f199b0c142c53fef3e1a AS chef

RUN cargo install cargo-chef --locked
WORKDIR /usr/local/src/langbench

# ----------------------------------------------------------------- planner ----
FROM chef AS planner

COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ----------------------------------------------------------------- builder ----
FROM chef AS builder

COPY --from=planner /usr/local/src/langbench/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo build --release --locked

# ----------------------------------------------------------------- runtime ----
FROM debian:trixie-slim@sha256:28de0877c2189802884ccd20f15ee41c203573bd87bb6b883f5f46362d24c5c2 AS runtime

RUN groupadd --system --gid 1000 langbench \
 && useradd --system --uid 1000 --gid langbench \
      --home /var/lib/langbench --shell /usr/sbin/nologin langbench \
 && install --directory --owner langbench --group langbench /var/lib/langbench

COPY --from=docker-cli /usr/local/bin/docker /usr/local/bin/docker
COPY --from=builder --chown=langbench:langbench \
     /usr/local/src/langbench/target/release/langbench /usr/local/bin/langbench

# Campaign inputs and outputs. Mount the benchmark tree read-only and the result
# directory read-write; both default here so the container needs no flags.
# `OUTPUT` is the samples file itself: `run` writes it, `csv` and `md` read it.
ENV BENCHMARKS_DIR=/var/lib/langbench/benchmarks \
    OUTPUT=/var/lib/langbench/samples.ndjson

USER langbench
WORKDIR /var/lib/langbench

ENTRYPOINT ["/usr/local/bin/langbench"]
CMD ["run"]
