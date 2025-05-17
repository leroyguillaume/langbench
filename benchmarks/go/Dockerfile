FROM golang:1.24-bookworm AS build

WORKDIR /usr/local/src/langbench

COPY go.mod .
COPY src src

RUN \
    go build \
        -o /usr/local/bin/langbench-mt-mergesort \
        ./src/mt_mergesort.go && \
    go build \
        -o /usr/local/bin/langbench-st-mergesort \
        ./src/st_mergesort.go

FROM debian:bookworm-slim AS base

RUN \
    apt-get update && \
    apt-get install -y --no-install-recommends \
        time=1.9-0.2 && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    mkdir -p /var/lib/langbench/result

WORKDIR /var/lib/langbench

ENTRYPOINT ["/usr/bin/time", "-f", "%e,%S,%U,%P,%M", "-o", "result/result.csv"]

FROM base AS st-mergesort

COPY --from=build /usr/local/bin/langbench-st-mergesort /usr/local/bin/langbench-st-mergesort

CMD ["sh", "-c", "langbench-st-mergesort $LANGBENCH_DATA_FILE $LANGBENCH_COUNT result/data"]

FROM base AS mt-mergesort

COPY --from=build /usr/local/bin/langbench-mt-mergesort /usr/local/bin/langbench-mt-mergesort

CMD ["sh", "-c", "langbench-mt-mergesort $LANGBENCH_DATA_FILE $LANGBENCH_COUNT $LANGBENCH_CORES result/data"]
