FROM debian:bookworm-slim AS build

RUN \
    apt-get update && \
    apt-get install -y --no-install-recommends \
        gcc=4:12.2.0-3 \
        libc6-dev=2.36-9+deb12u10 && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /usr/local/src/langbench

COPY *.c .

RUN \
    for file in *.c; do \
        gcc \
            -O3 \
            -march=native \
            -mtune=native \
            -flto \
            -ffast-math \
            -fomit-frame-pointer \
            -pthread \
            -o "/usr/local/bin/langbench-${file%.c}" \
            "${file}"; \
    done

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
