FROM ghcr.io/graalvm/native-image-community:24 AS build

WORKDIR /usr/local/src/langbench

COPY *.java .

RUN \
    javac ./*.java && \
    native-image StMergeSort && \
    native-image MtMergeSort

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

COPY --from=build /usr/local/src/langbench/stmergesort /usr/local/bin/langbench-st-mergesort

CMD ["sh", "-c", "langbench-st-mergesort $LANGBENCH_DATA_FILE $LANGBENCH_COUNT result/data"]

FROM base AS mt-mergesort

COPY --from=build /usr/local/src/langbench/mtmergesort /usr/local/bin/langbench-mt-mergesort

CMD ["sh", "-c", "langbench-mt-mergesort $LANGBENCH_DATA_FILE $LANGBENCH_COUNT $LANGBENCH_CORES result/data"]
