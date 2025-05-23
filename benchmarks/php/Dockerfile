FROM php:8.2-cli-bookworm AS base

RUN \
    apt-get update && \
    apt-get install -y --no-install-recommends \
        time=1.9-0.2 && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* && \
    docker-php-ext-install pcntl shmop && \
    mkdir -p /var/lib/langbench/result && \
    echo "memory_limit = 4G" > /usr/local/etc/php/conf.d/memory-limit.ini

WORKDIR /usr/local/src/langbench

COPY *.php .

WORKDIR /var/lib/langbench

ENTRYPOINT ["/usr/bin/time", "-f", "%e,%S,%U,%P,%M", "-o", "result/result.csv"]

FROM base AS st-mergesort

CMD ["sh", "-c", "php /usr/local/src/langbench/st_mergesort.php $LANGBENCH_DATA_FILE $LANGBENCH_COUNT result/data"]

FROM base AS mt-mergesort

CMD ["sh", "-c", "php /usr/local/src/langbench/mt_mergesort.php $LANGBENCH_DATA_FILE $LANGBENCH_COUNT $LANGBENCH_CORES result/data"]
