FROM debian:stable-slim AS build

WORKDIR /usr/local/src

RUN \
    apt-get update && \
    apt-get install -y \
        --no-install-recommends \
        clang=1:14.0-55.7~deb12u1 && \
    rm -rf /var/lib/apt/lists/*

COPY langbench.c .

RUN clang \
    -ffast-math \
    -flto \
    -funroll-loops \
    -lm \
    -mtune=native \
    -Ofast \
    -pthread \
    -Wall \
    -Werror \
    -o /usr/local/bin/langbench \
    langbench.c

# hadolint ignore=DL3006
FROM langbench-base

COPY --from=build /usr/local/bin/langbench /usr/local/bin/langbench

CMD ["sh", "-c", "langbench ${DATA_FILE} ${DATA_SIZE} $(nproc)"]
