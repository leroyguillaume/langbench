# hadolint ignore=DL3006
FROM langbench-base AS base

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

COPY *.js /usr/local/src/langbench/

RUN curl -fsSL https://bun.sh/install | bash

FROM base AS singlethread

CMD ["sh", "-c", "/root/.bun/bin/bun /usr/local/src/langbench/singlethread.js ${DATA_FILE} ${DATA_SIZE}"]

FROM base AS multithreads

CMD ["sh", "-c", "/root/.bun/bin/bun /usr/local/src/langbench/multithreads.js ${DATA_FILE} ${DATA_SIZE} ${THREADS}"]
