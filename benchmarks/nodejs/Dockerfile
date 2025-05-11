# hadolint ignore=DL3006
FROM langbench-base AS base

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ARG NODE_VERSION=24
ENV NVM_DIR=/root/.nvm
ENV NODE_VERSION=$NODE_VERSION

COPY *.js /usr/local/src/langbench/

RUN \
    curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash && \
    bash $NVM_DIR/nvm.sh install $NODE_VERSION

FROM base AS singlethread

CMD ["sh", "-c", "/root/.nvm/versions/node/v24.0.1/bin/node /usr/local/src/langbench/singlethread.js ${DATA_FILE} ${DATA_SIZE}"]

FROM base AS multithreads

CMD ["sh", "-c", "/root/.nvm/versions/node/v24.0.1/bin/node /usr/local/src/langbench/multithreads.js ${DATA_FILE} ${DATA_SIZE} $(nproc)"]
