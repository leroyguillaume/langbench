FROM openjdk:24-jdk-slim-bookworm AS base

SHELL ["/bin/bash", "-eo", "pipefail", "-c"]

RUN \
    apt-get update && \
    apt-get install -y --no-install-recommends \
        apt-transport-https=2.6.1 \
        ca-certificates=20230311 \
        curl=7.88.1-10+deb12u12 \
        gnupg=2.2.40-1.1 \
        time=1.9-0.2 && \
    echo "deb https://repo.scala-sbt.org/scalasbt/debian all main" | \
        tee /etc/apt/sources.list.d/sbt.list && \
    curl -Lsf "https://keyserver.ubuntu.com/pks/lookup?op=get&search=0x2EE0EA64E40A89B84B2DF73499E82A75642AC823" | \
        gpg \
            --no-default-keyring \
            --keyring gnupg-ring:/etc/apt/trusted.gpg.d/scalasbt-release.gpg \
            --import && \
    chmod 644 /etc/apt/trusted.gpg.d/scalasbt-release.gpg && \
    apt-get update && \
    apt-get install -y --no-install-recommends \
        sbt=1.10.11 && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    mkdir -p /var/lib/langbench/result

WORKDIR /usr/local/src/langbench

COPY build.sbt .
COPY project project
COPY src src

RUN sbt assembly

WORKDIR /var/lib/langbench

ENTRYPOINT ["/usr/bin/time", "-f", "%e,%S,%U,%P,%M", "-o", "result/result.csv"]

FROM base AS st-mergesort

CMD ["sh", "-c", "java -cp /usr/local/src/langbench/target/scala-3.3.3/langbench.jar StMergeSort $LANGBENCH_DATA_FILE $LANGBENCH_COUNT result/data"]

FROM base AS mt-mergesort

CMD ["sh", "-c", "java -cp /usr/local/src/langbench/target/scala-3.3.3/langbench.jar MtMergeSort $LANGBENCH_DATA_FILE $LANGBENCH_COUNT $LANGBENCH_CORES result/data"]
