# langbench

A comprehensive benchmarking tool designed to compare performance across different programming languages. The tool focuses on implementing and measuring the performance of sorting algorithms, specifically Merge Sort, in both single-threaded and multi-threaded configurations.

Key features:
- Supports multiple programming languages including Python, Rust, C, Java, Node.js, and Bun
- Implements both single-threaded (st-mergesort) and multi-threaded (mt-mergesort) Merge Sort algorithms
- Measures key performance metrics:
  - Execution time
  - CPU usage
  - Memory consumption
  - System and user time
- Generates detailed benchmark reports with comparative analysis
- Uses Docker containers for consistent and isolated testing environments
- Supports configurable parameters:
  - Number of integers to sort
  - Number of iterations
  - Number of CPU cores for multi-threaded tests
  - Custom benchmark selection

## Results

You can find reports [here](reports/README.md).

## Benchmark Focus

This project focuses exclusively on measuring raw performance metrics (CPU and memory usage) across different programming languages. It does not attempt to evaluate:

- Development tooling or ecosystem
- Code maintainability or readability
- Language features or expressiveness
- Build system efficiency
- Package management
- Development workflow

The goal is to provide a clear, objective comparison of how different languages perform the same computational task under identical conditions. This helps developers make informed decisions about language choice when performance is the primary concern.

## Interpretation

The raw time values for a given language are not to be taken separately or as exact performance in seconds, as it includes Docker overhead and is directly linked to the hardware running the benchmark.
You can interpret the benchmark by comparing one language to another, for example, comparing its total duration `time`.

## Disclaimer

This project's codebase was primarily written by Claude 3.7 Sonnet, with approximately 90% of the code generated through AI assistance. **While the implementations aim to be fair and accurate across different languages, there is probably room for optimization and improvement from human experts in each language.**

Contributions are very welcome! If you have expertise in any of the supported languages and can suggest optimizations or improvements to the implementations, please feel free to open a Pull Request (see [CONTRIBUTING](#contributing)). This could include:

- Language-specific optimizations
- Better memory management strategies
- Improved threading implementations
- Additional benchmark algorithms
- Support for more programming languages

Let's work together to make these benchmarks as fair and comprehensive as possible!

## Implementation Fairness

The benchmark implementations are designed to be fair and consistent across languages while respecting each language's specific strengths:

- **Language-Specific Optimizations**: Each implementation uses language-specific optimizations (e.g., Rust's native types, Python's array module, C's compiler flags) but maintains the same algorithmic approach
- **Consistent I/O**: All implementations use the same binary I/O format for reading input and writing output, ensuring no language gets an unfair advantage from I/O operations
- **Memory Management**: Each implementation is optimized for its language's memory model while maintaining the same memory access patterns
- **Threading Model**: Multi-threaded implementations use each language's native threading primitives (e.g., Python's [PEP 734](https://peps.python.org/pep-0734), Rust's `stdlib`, C's `pthreads`) but follow the same parallelization strategy
- **Build Process**: All implementations are compiled with equivalent optimization levels and run in isolated Docker containers with the same resource constraints

## Install

### With `uv` (recommended)

```bash
uv venv
source .venv/bin/activate
uv pip install -e .
```

### With `pip`

```bash
python -m venv venv
source .venv/bin/activate
pip install -e .
```

## Run

```bash
langbench generate-data
langbench run
```

## Contributing

**Prerequisites:**
- [pre-commit](https://pre-commit.com/)

```bash
pre-commit install
```
