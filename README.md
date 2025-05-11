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

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th></tr><tr><td>c</td><td>1.52</td><td>0.06</td><td>11.81</td><td>777</td></tr><tr><td>bunjs</td><td>1.98</td><td>0.49</td><td>12.01</td><td>629</td></tr><tr><td>nodejs</td><td>3.12</td><td>0.71</td><td>20.66</td><td>684</td></tr></table>

**Comparison**

A value of 100% indicates equal performance between the row and column languages.

A value of 50% indicates that the row language performs the computation twice as fast as the column language.

<table><tr><th></th><th>c</th><th>bunjs</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>76.77%</td><td>48.72%</td></tr><tr><th>bunjs</th><td>130.26%</td><td>100.0%</td><td>63.46%</td></tr><tr><th>nodejs</th><td>205.26%</td><td>157.58%</td><td>100.0%</td></tr></table>

### Singlethreaded

**Results**

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th></tr><tr><td>c</td><td>8.3</td><td>0.01</td><td>8.29</td><td>99</td></tr><tr><td>bunjs</td><td>8.49</td><td>0.14</td><td>8.35</td><td>100</td></tr><tr><td>nodejs</td><td>14.31</td><td>0.38</td><td>13.93</td><td>99</td></tr></table>

**Comparison**

A value of 100% indicates equal performance between the row and column languages.

A value of 50% indicates that the row language performs the computation twice as fast as the column language.

<table><tr><th></th><th>c</th><th>bunjs</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>97.76%</td><td>58.0%</td></tr><tr><th>bunjs</th><td>102.29%</td><td>100.0%</td><td>59.33%</td></tr><tr><th>nodejs</th><td>172.41%</td><td>168.55%</td><td>100.0%</td></tr></table>

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
