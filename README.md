# LangBench

LangBench is a simple benchmarking tool that processes numeric data and computes a mathematical result.

## Benchmark

### Hardware Specifications

**Processor**: arm

**Cores**: 12

### Data

**Size**: 534 MB

### Multithreaded

#### Results

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th><th>Max Memory (MB)</th></tr><tr><td>c</td><td>0.87</td><td>0.05</td><td>10.08</td><td>1152</td><td>535.55</td></tr><tr><td>bunjs</td><td>1.06</td><td>0.23</td><td>10.21</td><td>978</td><td>1159.28</td></tr><tr><td>nodejs</td><td>1.72</td><td>0.34</td><td>17.42</td><td>1030</td><td>1609.23</td></tr></table>

#### CPU Usage Comparison

*A value of 100% indicates equal performance between the row and column languages.*

*A value of 50% indicates that the row language performs the computation twice as fast as the column language.*

<table><tr><th></th><th>c</th><th>bunjs</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>82.08%</td><td>50.58%</td></tr><tr><th>bunjs</th><td>121.84%</td><td>100.0%</td><td>61.63%</td></tr><tr><th>nodejs</th><td>197.7%</td><td>162.26%</td><td>100.0%</td></tr></table>

#### Memory Usage Comparison

*A value of 100% indicates equal memory usage between the row and column languages.*

*A value of 50% indicates that the row language uses half the memory of the column language.*

<table><tr><th></th><th>c</th><th>bunjs</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>46.2%</td><td>33.28%</td></tr><tr><th>bunjs</th><td>216.47%</td><td>100.0%</td><td>72.04%</td></tr><tr><th>nodejs</th><td>300.48%</td><td>138.81%</td><td>100.0%</td></tr></table>

### Singlethreaded

#### Results

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th><th>Max Memory (MB)</th></tr><tr><td>bunjs</td><td>7.22</td><td>0.11</td><td>7.12</td><td>100</td><td>578.54</td></tr><tr><td>c</td><td>7.29</td><td>0.02</td><td>7.25</td><td>99</td><td>535.47</td></tr><tr><td>nodejs</td><td>12.6</td><td>0.18</td><td>12.42</td><td>100</td><td>589.1</td></tr></table>

#### CPU Usage Comparison

*A value of 100% indicates equal performance between the row and column languages.*

*A value of 50% indicates that the row language performs the computation twice as fast as the column language.*

<table><tr><th></th><th>bunjs</th><th>c</th><th>nodejs</th></tr><tr><th>bunjs</th><td>100.0%</td><td>99.04%</td><td>57.3%</td></tr><tr><th>c</th><td>100.97%</td><td>100.0%</td><td>57.86%</td></tr><tr><th>nodejs</th><td>174.52%</td><td>172.84%</td><td>100.0%</td></tr></table>

#### Memory Usage Comparison

*A value of 100% indicates equal memory usage between the row and column languages.*

*A value of 50% indicates that the row language uses half the memory of the column language.*

<table><tr><th></th><th>bunjs</th><th>c</th><th>nodejs</th></tr><tr><th>bunjs</th><td>100.0%</td><td>108.04%</td><td>98.21%</td></tr><tr><th>c</th><td>92.56%</td><td>100.0%</td><td>90.9%</td></tr><tr><th>nodejs</th><td>101.83%</td><td>110.02%</td><td>100.0%</td></tr></table>

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
