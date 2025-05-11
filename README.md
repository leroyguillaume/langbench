# LangBench

LangBench is a simple benchmarking tool that processes numeric data and computes a mathematical result.

## Benchmark

### Hardware Specifications

**Processor**: arm

**Cores**: 8

### Data

**Size**: 534 MB

### Multithreaded

#### Results

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th><th>Max Memory (MB)</th></tr><tr><td>c</td><td>1.53</td><td>0.1</td><td>11.72</td><td>770</td><td>535.58</td></tr><tr><td>bunjs</td><td>1.73</td><td>0.28</td><td>11.94</td><td>704</td><td>1144.46</td></tr><tr><td>nodejs</td><td>2.85</td><td>0.44</td><td>19.61</td><td>703</td><td>1443.32</td></tr></table>

#### CPU Usage Comparison

*A value of 100% indicates equal performance between the row and column languages.*

*A value of 50% indicates that the row language performs the computation twice as fast as the column language.*

<table><tr><th></th><th>c</th><th>bunjs</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>88.44%</td><td>53.68%</td></tr><tr><th>bunjs</th><td>113.07%</td><td>100.0%</td><td>60.7%</td></tr><tr><th>nodejs</th><td>186.27%</td><td>164.74%</td><td>100.0%</td></tr></table>

#### Memory Usage Comparison

*A value of 100% indicates equal memory usage between the row and column languages.*

*A value of 50% indicates that the row language uses half the memory of the column language.*

<table><tr><th></th><th>c</th><th>bunjs</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>46.8%</td><td>37.11%</td></tr><tr><th>bunjs</th><td>213.69%</td><td>100.0%</td><td>79.29%</td></tr><tr><th>nodejs</th><td>269.49%</td><td>126.11%</td><td>100.0%</td></tr></table>

### Singlethreaded

#### Results

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th><th>Max Memory (MB)</th></tr><tr><td>c</td><td>8.27</td><td>0.01</td><td>8.25</td><td>99</td><td>535.45</td></tr><tr><td>bunjs</td><td>8.58</td><td>0.18</td><td>8.4</td><td>100</td><td>578.48</td></tr><tr><td>nodejs</td><td>13.96</td><td>0.13</td><td>13.82</td><td>99</td><td>588.92</td></tr></table>

#### CPU Usage Comparison

*A value of 100% indicates equal performance between the row and column languages.*

*A value of 50% indicates that the row language performs the computation twice as fast as the column language.*

<table><tr><th></th><th>c</th><th>bunjs</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>96.39%</td><td>59.24%</td></tr><tr><th>bunjs</th><td>103.75%</td><td>100.0%</td><td>61.46%</td></tr><tr><th>nodejs</th><td>168.8%</td><td>162.7%</td><td>100.0%</td></tr></table>

#### Memory Usage Comparison

*A value of 100% indicates equal memory usage between the row and column languages.*

*A value of 50% indicates that the row language uses half the memory of the column language.*

<table><tr><th></th><th>c</th><th>bunjs</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>92.56%</td><td>90.92%</td></tr><tr><th>bunjs</th><td>108.04%</td><td>100.0%</td><td>98.23%</td></tr><tr><th>nodejs</th><td>109.99%</td><td>101.8%</td><td>100.0%</td></tr></table>

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
