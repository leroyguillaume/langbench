# hadolint ignore=DL3006
FROM langbench-base

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

COPY \
    langbench.js \
    worker.js \
    /usr/local/src/

RUN \
    curl -fsSL https://bun.sh/install | bash

CMD ["sh", "-c", "/root/.bun/bin/bun /usr/local/src/langbench.js ${DATA_FILE} ${DATA_SIZE}"]
