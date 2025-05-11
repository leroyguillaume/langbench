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

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th><th>Max Memory (MB)</th></tr><tr><td>c</td><td>1.61</td><td>0.11</td><td>12.17</td><td>762</td><td>535.54</td></tr><tr><td>rust</td><td>1.68</td><td>0.14</td><td>12.7</td><td>765</td><td>535.99</td></tr><tr><td>java</td><td>1.77</td><td>0.09</td><td>13.34</td><td>756</td><td>583.34</td></tr><tr><td>bunjs</td><td>1.87</td><td>0.37</td><td>12.6</td><td>692</td><td>1143.71</td></tr><tr><td>nodejs</td><td>2.8</td><td>0.45</td><td>19.21</td><td>701</td><td>1455.04</td></tr></table>

#### CPU Usage Comparison

*A value of 100% indicates equal performance between the row and column languages.*

*A value of 50% indicates that the row language performs the computation twice as fast as the column language.*

<table><tr><th></th><th>c</th><th>rust</th><th>java</th><th>bunjs</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>95.83%</td><td>90.96%</td><td>86.1%</td><td>57.5%</td></tr><tr><th>rust</th><td>104.35%</td><td>100.0%</td><td>94.92%</td><td>89.84%</td><td>60.0%</td></tr><tr><th>java</th><td>109.94%</td><td>105.36%</td><td>100.0%</td><td>94.65%</td><td>63.21%</td></tr><tr><th>bunjs</th><td>116.15%</td><td>111.31%</td><td>105.65%</td><td>100.0%</td><td>66.79%</td></tr><tr><th>nodejs</th><td>173.91%</td><td>166.67%</td><td>158.19%</td><td>149.73%</td><td>100.0%</td></tr></table>

#### Memory Usage Comparison

*A value of 100% indicates equal memory usage between the row and column languages.*

*A value of 50% indicates that the row language uses half the memory of the column language.*

<table><tr><th></th><th>c</th><th>rust</th><th>java</th><th>bunjs</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>99.92%</td><td>91.81%</td><td>46.82%</td><td>36.81%</td></tr><tr><th>rust</th><td>100.08%</td><td>100.0%</td><td>91.88%</td><td>46.86%</td><td>36.84%</td></tr><tr><th>java</th><td>108.93%</td><td>108.83%</td><td>100.0%</td><td>51.0%</td><td>40.09%</td></tr><tr><th>bunjs</th><td>213.56%</td><td>213.38%</td><td>196.06%</td><td>100.0%</td><td>78.6%</td></tr><tr><th>nodejs</th><td>271.7%</td><td>271.47%</td><td>249.43%</td><td>127.22%</td><td>100.0%</td></tr></table>

### Singlethreaded

#### Results

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th><th>Max Memory (MB)</th></tr><tr><td>c</td><td>8.28</td><td>0.01</td><td>8.27</td><td>99</td><td>535.47</td></tr><tr><td>rust</td><td>8.33</td><td>0.03</td><td>8.29</td><td>99</td><td>535.96</td></tr><tr><td>bunjs</td><td>8.59</td><td>0.15</td><td>8.44</td><td>100</td><td>578.67</td></tr><tr><td>java</td><td>8.84</td><td>0.02</td><td>8.84</td><td>100</td><td>581.8</td></tr><tr><td>nodejs</td><td>14.17</td><td>0.18</td><td>13.99</td><td>100</td><td>589.12</td></tr></table>

#### CPU Usage Comparison

*A value of 100% indicates equal performance between the row and column languages.*

*A value of 50% indicates that the row language performs the computation twice as fast as the column language.*

<table><tr><th></th><th>c</th><th>rust</th><th>bunjs</th><th>java</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>99.4%</td><td>96.39%</td><td>93.67%</td><td>58.43%</td></tr><tr><th>rust</th><td>100.6%</td><td>100.0%</td><td>96.97%</td><td>94.23%</td><td>58.79%</td></tr><tr><th>bunjs</th><td>103.74%</td><td>103.12%</td><td>100.0%</td><td>97.17%</td><td>60.62%</td></tr><tr><th>java</th><td>106.76%</td><td>106.12%</td><td>102.91%</td><td>100.0%</td><td>62.39%</td></tr><tr><th>nodejs</th><td>171.14%</td><td>170.11%</td><td>164.96%</td><td>160.29%</td><td>100.0%</td></tr></table>

#### Memory Usage Comparison

*A value of 100% indicates equal memory usage between the row and column languages.*

*A value of 50% indicates that the row language uses half the memory of the column language.*

<table><tr><th></th><th>c</th><th>rust</th><th>bunjs</th><th>java</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>99.91%</td><td>92.53%</td><td>92.04%</td><td>90.89%</td></tr><tr><th>rust</th><td>100.09%</td><td>100.0%</td><td>92.62%</td><td>92.12%</td><td>90.98%</td></tr><tr><th>bunjs</th><td>108.07%</td><td>107.97%</td><td>100.0%</td><td>99.46%</td><td>98.23%</td></tr><tr><th>java</th><td>108.65%</td><td>108.55%</td><td>100.54%</td><td>100.0%</td><td>98.76%</td></tr><tr><th>nodejs</th><td>110.02%</td><td>109.92%</td><td>101.81%</td><td>101.26%</td><td>100.0%</td></tr></table>

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
