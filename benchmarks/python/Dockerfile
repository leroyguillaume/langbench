FROM python:3.13-slim-bookworm AS base

RUN \
    apt-get update && \
    apt-get install -y --no-install-recommends \
        time=1.9-0.2 && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    mkdir -p /var/lib/langbench/result

WORKDIR /usr/local/src/langbench

COPY src src
COPY pyproject.toml pyproject.toml

RUN pip install -e --no-cache-dir .

WORKDIR /var/lib/langbench

FROM base AS st-mergesort

ENTRYPOINT ["/usr/bin/time", "-f", "%e,%S,%U,%P,%M", "-o", "result/result.csv"]
CMD ["sh", "-c", "langbench-st-mergesort $LANGBENCH_DATA_FILE $LANGBENCH_COUNT result/data"]

FROM base AS mt-mergesort

ENTRYPOINT ["/usr/bin/time", "-f", "%e,%S,%U,%P,%M", "-o", "result/result.csv"]
CMD ["sh", "-c", "langbench-mt-mergesort $LANGBENCH_DATA_FILE $LANGBENCH_COUNT $LANGBENCH_CORES result/data"]
