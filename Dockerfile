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
# Only the crate's own inputs, never `COPY . .`: the benchmark tree ships in the
# runtime image, and copying it here would make editing a kernel recompile the
# harness that measures it.
FROM chef AS planner

COPY .cargo .cargo
COPY Cargo.toml Cargo.lock build.rs ./
COPY src src
COPY templates templates
RUN cargo chef prepare --recipe-path recipe.json

# ----------------------------------------------------------------- builder ----
FROM chef AS builder

COPY --from=planner /usr/local/src/langbench/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY .cargo .cargo
COPY Cargo.toml Cargo.lock build.rs ./
COPY src src
COPY templates templates
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

# The benchmark tree ships with the harness, so `docker run langbench` measures
# something out of the box. It is read-only, architecture-independent data, and
# it lives well away from the workdir on purpose: a caller mounts a directory
# over /var/lib/langbench to collect the samples, and anything nested under that
# path would be shadowed by the mount.
COPY --chown=langbench:langbench benchmarks /usr/local/share/langbench/benchmarks

# Inputs are baked in, outputs go to the workdir — mount it to keep them.
#
# `SAMPLES_OUTPUT` names the samples file on both sides — `run` writes it, `csv`
# and `md` default to reading the same one. The name matters: the harness reads
# `SAMPLES_OUTPUT`, so a plain `OUTPUT` here would be dead config.
ENV BENCHMARKS_DIR=/usr/local/share/langbench/benchmarks \
    SAMPLES_OUTPUT=/var/lib/langbench/samples.ndjson \
    CSV_OUTPUT=/var/lib/langbench/samples.csv \
    MD_OUTPUT=/var/lib/langbench/report.md

USER langbench
WORKDIR /var/lib/langbench

ENTRYPOINT ["/usr/local/bin/langbench"]
CMD ["run"]
