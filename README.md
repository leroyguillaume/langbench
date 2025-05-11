# LangBench

LangBench is a simple benchmarking tool that processes numeric data and computes a mathematical result.

## Benchmark

### Hardware Specifications

**Processor**: arm

**Cores**: 8

### Data

**Size**: 534 MB

**Number of iterations:** 1

### Multithreaded

#### Results

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th><th>Max Memory (MB)</th></tr><tr><td>c</td><td>1.51</td><td>0.08</td><td>11.67</td><td>775</td><td>535.54</td></tr><tr><td>rust</td><td>1.56</td><td>0.09</td><td>11.77</td><td>759</td><td>535.96</td></tr><tr><td>java</td><td>1.68</td><td>0.09</td><td>12.5</td><td>747</td><td>583.19</td></tr><tr><td>bunjs</td><td>1.71</td><td>0.18</td><td>11.83</td><td>702</td><td>613.78</td></tr><tr><td>nodejs</td><td>2.78</td><td>0.43</td><td>19.56</td><td>716</td><td>1397.96</td></tr><tr><td>python3.11</td><td>41.23</td><td>1.5</td><td>39.77</td><td>100</td><td>6968.91</td></tr></table>

#### CPU Usage Comparison

*A value of 100% indicates equal performance between the row and column languages.*

*A value of 50% indicates that the row language performs the computation twice as fast as the column language.*

<table><tr><th></th><th>c</th><th>rust</th><th>java</th><th>bunjs</th><th>nodejs</th><th>python3.11</th></tr><tr><th>c</th><td>100.0%</td><td>96.79%</td><td>89.88%</td><td>88.3%</td><td>54.32%</td><td>3.66%</td></tr><tr><th>rust</th><td>103.31%</td><td>100.0%</td><td>92.86%</td><td>91.23%</td><td>56.12%</td><td>3.78%</td></tr><tr><th>java</th><td>111.26%</td><td>107.69%</td><td>100.0%</td><td>98.25%</td><td>60.43%</td><td>4.07%</td></tr><tr><th>bunjs</th><td>113.25%</td><td>109.62%</td><td>101.79%</td><td>100.0%</td><td>61.51%</td><td>4.15%</td></tr><tr><th>nodejs</th><td>184.11%</td><td>178.21%</td><td>165.48%</td><td>162.57%</td><td>100.0%</td><td>6.74%</td></tr><tr><th>python3.11</th><td>2730.46%</td><td>2642.95%</td><td>2454.17%</td><td>2411.11%</td><td>1483.09%</td><td>100.0%</td></tr></table>

#### Memory Usage Comparison

*A value of 100% indicates equal memory usage between the row and column languages.*

*A value of 50% indicates that the row language uses half the memory of the column language.*

<table><tr><th></th><th>c</th><th>rust</th><th>java</th><th>bunjs</th><th>nodejs</th><th>python3.11</th></tr><tr><th>c</th><td>100.0%</td><td>99.92%</td><td>91.83%</td><td>87.25%</td><td>38.31%</td><td>7.68%</td></tr><tr><th>rust</th><td>100.08%</td><td>100.0%</td><td>91.9%</td><td>87.32%</td><td>38.34%</td><td>7.69%</td></tr><tr><th>java</th><td>108.9%</td><td>108.81%</td><td>100.0%</td><td>95.02%</td><td>41.72%</td><td>8.37%</td></tr><tr><th>bunjs</th><td>114.61%</td><td>114.52%</td><td>105.25%</td><td>100.0%</td><td>43.91%</td><td>8.81%</td></tr><tr><th>nodejs</th><td>261.04%</td><td>260.83%</td><td>239.71%</td><td>227.76%</td><td>100.0%</td><td>20.06%</td></tr><tr><th>python3.11</th><td>1301.29%</td><td>1300.27%</td><td>1194.96%</td><td>1135.41%</td><td>498.51%</td><td>100.0%</td></tr></table>

### Singlethreaded

#### Results

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th><th>Max Memory (MB)</th></tr><tr><td>rust</td><td>8.28</td><td>0.02</td><td>8.26</td><td>99</td><td>535.95</td></tr><tr><td>c</td><td>8.32</td><td>0.01</td><td>8.31</td><td>99</td><td>535.46</td></tr><tr><td>bunjs</td><td>8.51</td><td>0.16</td><td>8.35</td><td>100</td><td>579.01</td></tr><tr><td>java</td><td>8.81</td><td>0.03</td><td>8.81</td><td>100</td><td>581.62</td></tr><tr><td>nodejs</td><td>14.12</td><td>0.14</td><td>13.97</td><td>100</td><td>589.05</td></tr><tr><td>python3.11</td><td>38.18</td><td>1.4</td><td>36.76</td><td>99</td><td>5899.82</td></tr></table>

#### CPU Usage Comparison

*A value of 100% indicates equal performance between the row and column languages.*

*A value of 50% indicates that the row language performs the computation twice as fast as the column language.*

<table><tr><th></th><th>rust</th><th>c</th><th>bunjs</th><th>java</th><th>nodejs</th><th>python3.11</th></tr><tr><th>rust</th><td>100.0%</td><td>99.52%</td><td>97.3%</td><td>93.98%</td><td>58.64%</td><td>21.69%</td></tr><tr><th>c</th><td>100.48%</td><td>100.0%</td><td>97.77%</td><td>94.44%</td><td>58.92%</td><td>21.79%</td></tr><tr><th>bunjs</th><td>102.78%</td><td>102.28%</td><td>100.0%</td><td>96.59%</td><td>60.27%</td><td>22.29%</td></tr><tr><th>java</th><td>106.4%</td><td>105.89%</td><td>103.53%</td><td>100.0%</td><td>62.39%</td><td>23.07%</td></tr><tr><th>nodejs</th><td>170.53%</td><td>169.71%</td><td>165.92%</td><td>160.27%</td><td>100.0%</td><td>36.98%</td></tr><tr><th>python3.11</th><td>461.11%</td><td>458.89%</td><td>448.65%</td><td>433.37%</td><td>270.4%</td><td>100.0%</td></tr></table>

#### Memory Usage Comparison

*A value of 100% indicates equal memory usage between the row and column languages.*

*A value of 50% indicates that the row language uses half the memory of the column language.*

<table><tr><th></th><th>rust</th><th>c</th><th>bunjs</th><th>java</th><th>nodejs</th><th>python3.11</th></tr><tr><th>rust</th><td>100.0%</td><td>100.09%</td><td>92.56%</td><td>92.15%</td><td>90.99%</td><td>9.08%</td></tr><tr><th>c</th><td>99.91%</td><td>100.0%</td><td>92.48%</td><td>92.06%</td><td>90.9%</td><td>9.08%</td></tr><tr><th>bunjs</th><td>108.03%</td><td>108.13%</td><td>100.0%</td><td>99.55%</td><td>98.3%</td><td>9.81%</td></tr><tr><th>java</th><td>108.52%</td><td>108.62%</td><td>100.45%</td><td>100.0%</td><td>98.74%</td><td>9.86%</td></tr><tr><th>nodejs</th><td>109.91%</td><td>110.01%</td><td>101.73%</td><td>101.28%</td><td>100.0%</td><td>9.98%</td></tr><tr><th>python3.11</th><td>1100.82%</td><td>1101.82%</td><td>1018.95%</td><td>1014.38%</td><td>1001.58%</td><td>100.0%</td></tr></table>

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
