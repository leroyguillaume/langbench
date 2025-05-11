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

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th><th>Max Memory (MB)</th></tr><tr><td>c</td><td>1.55</td><td>0.09</td><td>11.73</td><td>763</td><td>535.62</td></tr><tr><td>java</td><td>1.71</td><td>0.08</td><td>12.39</td><td>729</td><td>583.57</td></tr><tr><td>bunjs</td><td>1.81</td><td>0.32</td><td>12.01</td><td>679</td><td>1143.47</td></tr><tr><td>nodejs</td><td>3.0</td><td>0.52</td><td>20.56</td><td>703</td><td>1513.36</td></tr></table>

#### CPU Usage Comparison

*A value of 100% indicates equal performance between the row and column languages.*

*A value of 50% indicates that the row language performs the computation twice as fast as the column language.*

<table><tr><th></th><th>c</th><th>java</th><th>bunjs</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>90.64%</td><td>85.64%</td><td>51.67%</td></tr><tr><th>java</th><td>110.32%</td><td>100.0%</td><td>94.48%</td><td>57.0%</td></tr><tr><th>bunjs</th><td>116.77%</td><td>105.85%</td><td>100.0%</td><td>60.33%</td></tr><tr><th>nodejs</th><td>193.55%</td><td>175.44%</td><td>165.75%</td><td>100.0%</td></tr></table>

#### Memory Usage Comparison

*A value of 100% indicates equal memory usage between the row and column languages.*

*A value of 50% indicates that the row language uses half the memory of the column language.*

<table><tr><th></th><th>c</th><th>java</th><th>bunjs</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>91.78%</td><td>46.84%</td><td>35.39%</td></tr><tr><th>java</th><td>108.95%</td><td>100.0%</td><td>51.04%</td><td>38.56%</td></tr><tr><th>bunjs</th><td>213.49%</td><td>195.94%</td><td>100.0%</td><td>75.56%</td></tr><tr><th>nodejs</th><td>282.54%</td><td>259.33%</td><td>132.35%</td><td>100.0%</td></tr></table>

### Singlethreaded

#### Results

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th><th>Max Memory (MB)</th></tr><tr><td>c</td><td>8.29</td><td>0.01</td><td>8.28</td><td>99</td><td>535.47</td></tr><tr><td>bunjs</td><td>8.56</td><td>0.15</td><td>8.41</td><td>100</td><td>578.59</td></tr><tr><td>java</td><td>8.82</td><td>0.02</td><td>8.81</td><td>100</td><td>581.99</td></tr><tr><td>nodejs</td><td>15.94</td><td>0.18</td><td>15.77</td><td>100</td><td>589.06</td></tr></table>

#### CPU Usage Comparison

*A value of 100% indicates equal performance between the row and column languages.*

*A value of 50% indicates that the row language performs the computation twice as fast as the column language.*

<table><tr><th></th><th>c</th><th>bunjs</th><th>java</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>96.85%</td><td>93.99%</td><td>52.01%</td></tr><tr><th>bunjs</th><td>103.26%</td><td>100.0%</td><td>97.05%</td><td>53.7%</td></tr><tr><th>java</th><td>106.39%</td><td>103.04%</td><td>100.0%</td><td>55.33%</td></tr><tr><th>nodejs</th><td>192.28%</td><td>186.21%</td><td>180.73%</td><td>100.0%</td></tr></table>

#### Memory Usage Comparison

*A value of 100% indicates equal memory usage between the row and column languages.*

*A value of 50% indicates that the row language uses half the memory of the column language.*

<table><tr><th></th><th>c</th><th>bunjs</th><th>java</th><th>nodejs</th></tr><tr><th>c</th><td>100.0%</td><td>92.55%</td><td>92.01%</td><td>90.9%</td></tr><tr><th>bunjs</th><td>108.05%</td><td>100.0%</td><td>99.42%</td><td>98.22%</td></tr><tr><th>java</th><td>108.69%</td><td>100.59%</td><td>100.0%</td><td>98.8%</td></tr><tr><th>nodejs</th><td>110.01%</td><td>101.81%</td><td>101.21%</td><td>100.0%</td></tr></table>

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
