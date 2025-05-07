FROM debian:stable-slim AS build

WORKDIR /usr/local/src

RUN \
    apt-get update && \
    apt-get install -y \
        --no-install-recommends \
        clang=1:14.0-55.7~deb12u1 && \
    rm -rf /var/lib/apt/lists/*

COPY random.c .

RUN clang \
    -flto \
    -mtune=native \
    -O3 \
    -Wall \
    -Werror \
    -o /usr/local/bin/random \
    random.c

FROM debian:stable-slim

ARG DATA_SIZE=20000000

ENV DATA_FILE=/etc/langbench/data
ENV DATA_SIZE=${DATA_SIZE}

RUN \
    apt-get update && \
    apt-get install -y \
        --no-install-recommends \
        time=1.9-0.2 && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /var/lib/langbench

COPY --from=build /usr/local/bin/random /usr/local/bin/random

RUN \
    mkdir -p /etc/langbench && \
    random "${DATA_FILE}" "${DATA_SIZE}"

ENTRYPOINT ["time", "-f", "%e,%S,%U,%P", "-o", "result.csv"]
