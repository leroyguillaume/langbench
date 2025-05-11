# LangBench

LangBench is a simple benchmarking tool that processes numeric data and computes a mathematical result.

## Benchmark

### Hardware Specifications

**Processor**: arm

**Cores**: 8

### Data

**Size**: 534 MB

### Multithreaded

**Results**

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th></tr><tr><td>c</td><td>1.62</td><td>0.15</td><td>11.79</td><td>734</td></tr><tr><td>bunjs</td><td>1.98</td><td>0.49</td><td>12.01</td><td>629</td></tr><tr><td>nodejs</td><td>3.12</td><td>0.71</td><td>20.66</td><td>684</td></tr></table>

**Comparison**

100% means the row language is as fast as the column language.

50% means the row language is twice slower than the column language.

<table><tr><th></th><th>c</th><th>bunjs</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>81.82%</td><td>51.92%</td></tr><tr><th>bunjs</th><td>122.22%</td><td>100.0%</td><td>63.46%</td></tr><tr><th>nodejs</th><td>192.59%</td><td>157.58%</td><td>100.0%</td></tr></table>

### Singlethreaded

**Results**

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th></tr><tr><td>bunjs</td><td>8.49</td><td>0.14</td><td>8.35</td><td>100</td></tr><tr><td>c</td><td>9.93</td><td>0.13</td><td>9.76</td><td>99</td></tr><tr><td>nodejs</td><td>14.31</td><td>0.38</td><td>13.93</td><td>99</td></tr></table>

**Comparison**

100% means the row language is as fast as the column language.

50% means the row language is twice slower than the column language.

<table><tr><th></th><th>bunjs</th><th>c</th><th>nodejs</th></tr><tr><th>bunjs</th><td>100.0%</td><td>85.5%</td><td>59.33%</td></tr><tr><th>c</th><td>116.96%</td><td>100.0%</td><td>69.39%</td></tr><tr><th>nodejs</th><td>168.55%</td><td>144.11%</td><td>100.0%</td></tr></table>

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
brew install hadolint pre-commit
pre-commit install
```
