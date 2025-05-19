FROM mcr.microsoft.com/dotnet/sdk:9.0 AS base

WORKDIR /usr/local/src/langbench

RUN \
    apt-get update && \
    apt-get install -y --no-install-recommends \
        time=1.9-0.2 && \
    rm -rf /var/lib/apt/lists/*

COPY *.csproj .
COPY src src

RUN \
    dotnet build -c Release StMergeSort.csproj && \
    dotnet build -c Release MtMergeSort.csproj

WORKDIR /var/lib/langbench

ENTRYPOINT ["/usr/bin/time", "-f", "%e,%S,%U,%P,%M", "-o", "result/result.csv"]

FROM base AS st-mergesort

CMD ["sh", "-c", "dotnet run --project /usr/local/src/langbench/StMergeSort.csproj $LANGBENCH_DATA_FILE $LANGBENCH_COUNT result/data"]

FROM base AS mt-mergesort

CMD ["sh", "-c", "dotnet run --project /usr/local/src/langbench/MtMergeSort.csproj $LANGBENCH_DATA_FILE $LANGBENCH_COUNT $LANGBENCH_CORES result/data"]
