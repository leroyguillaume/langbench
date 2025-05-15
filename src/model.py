import multiprocessing
import platform
from pydantic import BaseModel


class Benchmark(BaseModel):
    name: str
    description: str


class BenchmarkMetrics(BaseModel):
    cpu_usage: float = 0
    max_memory: float = 0
    time: float = 0
    system_time: float = 0
    user_time: float = 0

    def avg(self, iterations: int) -> "BenchmarkMetrics":
        return BenchmarkMetrics(
            cpu_usage=self.cpu_usage / iterations,
            max_memory=self.max_memory / iterations,
            time=self.time / iterations,
            system_time=self.system_time / iterations,
            user_time=self.user_time / iterations,
        )


class LanguageResult(BaseModel):
    language: str
    metrics: BenchmarkMetrics


class BenchmarkResult(BaseModel):
    arch: str
    benchmark: Benchmark
    cores: int
    count: int
    iterations: int
    results: list[LanguageResult] = []

    def sort(self):
        self.results.sort(key=lambda result: result.metrics.time)

    def upsert(self, result: LanguageResult):
        for existing_result in self.results:
            if existing_result.language == result.language:
                existing_result.metrics = result.metrics
                return
        self.results.append(result)

    def __iter__(self):
        return iter(self.results)


class BenchmarkResults(BaseModel):
    results: list[BenchmarkResult] = []

    def get_or_create(
        self, benchmark_name: str, count: int, iterations: int
    ) -> BenchmarkResult:
        for result in self.results:
            if (
                result.benchmark.name == benchmark_name
                and result.count == count
                and result.iterations == iterations
            ):
                return result
        result = BenchmarkResult(
            arch=platform.machine(),
            benchmark=next(b for b in BENCHMARKS if b.name == benchmark_name),
            cores=multiprocessing.cpu_count(),
            count=count,
            iterations=iterations,
        )
        self.results.append(result)
        return result

    def sort(self):
        self.results.sort(
            key=lambda result: f"{result.benchmark.name}-{result.arch}-{result.cores}-{result.count}-{result.iterations}"
        )

    def __iter__(self):
        return iter(self.results)


BENCHMARKS: list[Benchmark] = [
    Benchmark(
        name="st-mergesort",
        description="Merge Sort algorithm implementation running on a single thread",
    ),
    Benchmark(
        name="mt-mergesort",
        description="Merge Sort algorithm implementation running on multiple threads",
    ),
]
