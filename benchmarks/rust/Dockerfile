FROM rust:1.75-slim AS build

WORKDIR /usr/local/src/langbench

COPY Cargo.toml .
COPY src src

RUN cargo build --release

FROM debian:bookworm-slim AS base

RUN \
    apt-get update && \
    apt-get install -y --no-install-recommends \
        time=1.9-0.2 && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    mkdir -p /var/lib/langbench/result

WORKDIR /var/lib/langbench

FROM base AS st-mergesort

COPY --from=build /usr/local/src/langbench/target/release/st-mergesort /usr/local/bin/langbench-st-mergesort

ENTRYPOINT ["/usr/bin/time", "-f", "%e,%S,%U,%P,%M", "-o", "result/result.csv"]
CMD ["sh", "-c", "langbench-st-mergesort $LANGBENCH_DATA_FILE $LANGBENCH_COUNT result/data"]

FROM base AS mt-mergesort

COPY --from=build /usr/local/src/langbench/target/release/mt-mergesort /usr/local/bin/langbench-mt-mergesort

ENTRYPOINT ["/usr/bin/time", "-f", "%e,%S,%U,%P,%M", "-o", "result/result.csv"]
CMD ["sh", "-c", "langbench-mt-mergesort $LANGBENCH_DATA_FILE $LANGBENCH_COUNT $LANGBENCH_CORES result/data"]
