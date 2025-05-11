# hadolint ignore=DL3006
FROM langbench-base AS base

RUN \
    apt-get update && \
    apt-get install -y --no-install-recommends \
        python3=3.11.2-1+b1 && \
    rm -rf /var/lib/apt/lists/*

COPY *.py /usr/local/src/langbench/

FROM base AS singlethread

CMD ["sh", "-c", "python3 /usr/local/src/langbench/singlethread.py ${DATA_FILE} ${DATA_SIZE}"]

FROM base AS multithreads

CMD ["sh", "-c", "python3 /usr/local/src/langbench/multithreads.py ${DATA_FILE} ${DATA_SIZE} ${THREADS}"]
