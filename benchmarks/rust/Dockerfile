FROM rust:slim-bookworm AS build

WORKDIR /usr/local/src/langbench

COPY *.rs .

RUN \
    rustc \
        -O \
        -C target-cpu=native \
        -C opt-level=3 \
        -C lto=fat \
        -C codegen-units=1 \
        -C panic=abort \
        -C strip=symbols \
        -o /usr/local/bin/langbench-st \
        singlethread.rs && \
    rustc \
        -O \
        -C target-cpu=native \
        -C opt-level=3 \
        -C lto=fat \
        -C codegen-units=1 \
        -C panic=abort \
        -C strip=symbols \
        -o /usr/local/bin/langbench-mt \
        multithreads.rs

# hadolint ignore=DL3006
FROM langbench-base AS singlethread

COPY --from=build /usr/local/bin/langbench-st /usr/local/bin/langbench-st

CMD ["sh", "-c", "langbench-st ${DATA_FILE} ${DATA_SIZE}"]

# hadolint ignore=DL3006
FROM langbench-base AS multithreads

COPY --from=build /usr/local/bin/langbench-mt /usr/local/bin/langbench-mt

CMD ["sh", "-c", "langbench-mt ${DATA_FILE} ${DATA_SIZE} ${THREADS}"]
