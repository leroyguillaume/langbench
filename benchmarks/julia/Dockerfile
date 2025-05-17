FROM julia:1-bookworm AS base

RUN \
    apt-get update && \
    apt-get install -y --no-install-recommends \
        time=1.9-0.2 && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    mkdir -p /var/lib/langbench/result

WORKDIR /usr/local/src/langbench

COPY *.jl .

WORKDIR /var/lib/langbench

ENTRYPOINT ["/usr/bin/time", "-f", "%e,%S,%U,%P,%M", "-o", "result/result.csv"]

FROM base AS st-mergesort

CMD ["sh", "-c", "julia /usr/local/src/langbench/st_mergesort.jl $LANGBENCH_DATA_FILE $LANGBENCH_COUNT result/data"]

FROM base AS mt-mergesort

CMD ["sh", "-c", "julia /usr/local/src/langbench/mt_mergesort.jl $LANGBENCH_DATA_FILE $LANGBENCH_COUNT $LANGBENCH_CORES result/data"]
