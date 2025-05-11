FROM debian:stable-slim AS build

WORKDIR /usr/local/src/langbench

RUN \
    apt-get update && \
    apt-get install -y \
        --no-install-recommends \
        clang=1:14.0-55.7~deb12u1 && \
    rm -rf /var/lib/apt/lists/*

COPY *.c .

RUN \
    clang \
        -ffast-math \
        -flto \
        -funroll-loops \
        -lm \
        -mtune=native \
        -Ofast \
        -Wall \
        -Werror \
        -o /usr/local/bin/langbench-st \
        singlethread.c && \
    clang \
        -ffast-math \
        -flto \
        -funroll-loops \
        -lm \
        -mtune=native \
        -Ofast \
        -pthread \
        -Wall \
        -Werror \
        -o /usr/local/bin/langbench-mt \
        multithreads.c

# hadolint ignore=DL3006
FROM langbench-base AS singlethread

COPY --from=build /usr/local/bin/langbench-st /usr/local/bin/langbench-st

CMD ["sh", "-c", "langbench-st ${DATA_FILE} ${DATA_SIZE}"]

# hadolint ignore=DL3006
FROM langbench-base AS multithreads

COPY --from=build /usr/local/bin/langbench-mt /usr/local/bin/langbench-mt

CMD ["sh", "-c", "langbench-mt ${DATA_FILE} ${DATA_SIZE} $(nproc)"]
