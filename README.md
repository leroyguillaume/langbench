# LangBench

LangBench is a simple benchmarking tool that processes numeric data and computes a mathematical result.

## Results

Benchmarks ran on 534 MB.

| Language   |   Elapsed Time |   System Time |   User Time | CPU Usage   |
|:-----------|---------------:|--------------:|------------:|:------------|
| c          |           7.64 |          0.04 |        7.6  | 99%         |
| nodejs     |          11.97 |          0.14 |       11.82 | 100%        |

## Usage

```bash
python -m venv venv
source venv/bin/activate
pip install -r requirements.txt
./langbench.py --help
```

## Algorithm Description

The algorithm works as follows:

1. It reads pairs of integers from an input file.
2. For each pair of integers (`left[i]`, `right[i]`):
   - Computes `cos(left[i])`
   - Computes `sin(right[i])`
   - Multiplies these values together
   - Takes the absolute value of the result
   - Computes the square root of this value
3. Sums all these calculations to produce a final result

Mathematically, the algorithm computes:

```
result = ∑ sqrt(|cos(left[i]) * sin(right[i])|)
```

## Contributing

### Installation

**MacOS**:

```bash
brew install clang-format hadolint pre-commit
pre-commit install
```
