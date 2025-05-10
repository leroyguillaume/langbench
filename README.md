# LangBench

LangBench is a simple benchmarking tool that processes numeric data and computes a mathematical result.

## Results

Benchmarks ran on 534 MB.

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th></tr><tr><td>c-pthread</td><td>1.74</td><td>675</td><td>0.25</td><td>11.53</td><tr><td>bunjs-worker</td><td>1.88</td><td>673</td><td>0.34</td><td>12.31</td><tr><td>nodejs-worker</td><td>3.28</td><td>669</td><td>0.85</td><td>21.13</td><tr><td>c</td><td>8.43</td><td>99</td><td>0.08</td><td>8.33</td><tr><td>bunjs</td><td>8.71</td><td>100</td><td>0.34</td><td>8.37</td><tr><td>nodejs</td><td>14.34</td><td>100</td><td>0.19</td><td>14.15</td></table>

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
