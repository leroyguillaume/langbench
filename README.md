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

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th><th>Max Memory (MB)</th></tr><tr><td>c</td><td>0.91</td><td>0.05</td><td>10.08</td><td>1112</td><td>535.57</td></tr><tr><td>rust</td><td>0.93</td><td>0.06</td><td>10.43</td><td>1121</td><td>535.84</td></tr><tr><td>bunjs</td><td>1.01</td><td>0.13</td><td>10.32</td><td>1031</td><td>626.37</td></tr><tr><td>java</td><td>1.07</td><td>0.07</td><td>11.63</td><td>1085</td><td>583.93</td></tr><tr><td>nodejs</td><td>1.77</td><td>0.29</td><td>18.18</td><td>1043</td><td>1607.66</td></tr></table>

#### CPU Usage Comparison

*A value of 100% indicates equal performance between the row and column languages.*

*A value of 50% indicates that the row language performs the computation twice as fast as the column language.*

<table><tr><th></th><th>c</th><th>rust</th><th>bunjs</th><th>java</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>97.85%</td><td>90.1%</td><td>85.05%</td><td>51.41%</td></tr><tr><th>rust</th><td>102.2%</td><td>100.0%</td><td>92.08%</td><td>86.92%</td><td>52.54%</td></tr><tr><th>bunjs</th><td>110.99%</td><td>108.6%</td><td>100.0%</td><td>94.39%</td><td>57.06%</td></tr><tr><th>java</th><td>117.58%</td><td>115.05%</td><td>105.94%</td><td>100.0%</td><td>60.45%</td></tr><tr><th>nodejs</th><td>194.51%</td><td>190.32%</td><td>175.25%</td><td>165.42%</td><td>100.0%</td></tr></table>

#### Memory Usage Comparison

*A value of 100% indicates equal memory usage between the row and column languages.*

*A value of 50% indicates that the row language uses half the memory of the column language.*

<table><tr><th></th><th>c</th><th>rust</th><th>bunjs</th><th>java</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>99.95%</td><td>85.5%</td><td>91.72%</td><td>33.31%</td></tr><tr><th>rust</th><td>100.05%</td><td>100.0%</td><td>85.55%</td><td>91.76%</td><td>33.33%</td></tr><tr><th>bunjs</th><td>116.95%</td><td>116.89%</td><td>100.0%</td><td>107.27%</td><td>38.96%</td></tr><tr><th>java</th><td>109.03%</td><td>108.97%</td><td>93.22%</td><td>100.0%</td><td>36.32%</td></tr><tr><th>nodejs</th><td>300.18%</td><td>300.03%</td><td>256.66%</td><td>275.32%</td><td>100.0%</td></tr></table>

### Singlethreaded

#### Results

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th><th>Max Memory (MB)</th></tr><tr><td>c</td><td>7.05</td><td>0.01</td><td>7.03</td><td>99</td><td>535.46</td></tr><tr><td>rust</td><td>7.22</td><td>0.01</td><td>7.21</td><td>99</td><td>535.9</td></tr><tr><td>bunjs</td><td>7.29</td><td>0.12</td><td>7.17</td><td>100</td><td>578.61</td></tr><tr><td>java</td><td>8.0</td><td>0.03</td><td>8.0</td><td>100</td><td>582.13</td></tr><tr><td>nodejs</td><td>12.16</td><td>0.15</td><td>12.0</td><td>100</td><td>589.21</td></tr></table>

#### CPU Usage Comparison

*A value of 100% indicates equal performance between the row and column languages.*

*A value of 50% indicates that the row language performs the computation twice as fast as the column language.*

<table><tr><th></th><th>c</th><th>rust</th><th>bunjs</th><th>java</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>97.65%</td><td>96.71%</td><td>88.12%</td><td>57.98%</td></tr><tr><th>rust</th><td>102.41%</td><td>100.0%</td><td>99.04%</td><td>90.25%</td><td>59.38%</td></tr><tr><th>bunjs</th><td>103.4%</td><td>100.97%</td><td>100.0%</td><td>91.12%</td><td>59.95%</td></tr><tr><th>java</th><td>113.48%</td><td>110.8%</td><td>109.74%</td><td>100.0%</td><td>65.79%</td></tr><tr><th>nodejs</th><td>172.48%</td><td>168.42%</td><td>166.8%</td><td>152.0%</td><td>100.0%</td></tr></table>

#### Memory Usage Comparison

*A value of 100% indicates equal memory usage between the row and column languages.*

*A value of 50% indicates that the row language uses half the memory of the column language.*

<table><tr><th></th><th>c</th><th>rust</th><th>bunjs</th><th>java</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>99.92%</td><td>92.54%</td><td>91.98%</td><td>90.88%</td></tr><tr><th>rust</th><td>100.08%</td><td>100.0%</td><td>92.62%</td><td>92.06%</td><td>90.95%</td></tr><tr><th>bunjs</th><td>108.06%</td><td>107.97%</td><td>100.0%</td><td>99.4%</td><td>98.2%</td></tr><tr><th>java</th><td>108.72%</td><td>108.63%</td><td>100.61%</td><td>100.0%</td><td>98.8%</td></tr><tr><th>nodejs</th><td>110.04%</td><td>109.95%</td><td>101.83%</td><td>101.22%</td><td>100.0%</td></tr></table>

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
