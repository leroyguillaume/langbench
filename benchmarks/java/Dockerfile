# hadolint ignore=DL3006
FROM localhost/langbench-base AS base

ENV PATH="/opt/jdk/jdk-24.0.1/bin:$PATH"

RUN \
    if [ "$(dpkg --print-architecture)" = "arm64" ]; then \
        arch="aarch64"; \
    elif [ "$(dpkg --print-architecture)" = "amd64" ]; then \
        arch="x64"; \
    else \
        echo "Unsupported architecture" 1>&2 && exit 1; \
    fi && \
    mkdir -p /opt/jdk && \
    curl -SsLfo /opt/jdk/jdk.tar.gz "https://download.java.net/java/GA/jdk24.0.1/24a58e0e276943138bf3e963e6291ac2/9/GPL/openjdk-24.0.1_linux-${arch}_bin.tar.gz" && \
    tar xzf /opt/jdk/jdk.tar.gz -C /opt/jdk && \
    rm /opt/jdk/jdk.tar.gz

COPY *.java /usr/local/src/langbench/

RUN javac /usr/local/src/langbench/*.java

FROM base AS singlethread

CMD ["sh", "-c", "java -cp /usr/local/src/langbench/ SingleThread ${DATA_FILE} ${DATA_SIZE}"]

FROM base AS multithreads

CMD ["sh", "-c", "java -cp /usr/local/src/langbench/ MultiThread ${DATA_FILE} ${DATA_SIZE} ${THREADS}"]
