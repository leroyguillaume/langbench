# LangBench

LangBench is a simple benchmarking tool that processes numeric data and computes a mathematical result.

## Benchmark

### Hardware Specifications

**Processor**: arm

**Cores**: 8

### Data

**Size**: 534 MB

### Multithreaded

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th></tr><tr><td>c</td><td>1.62</td><td>0.15</td><td>11.79</td><td>734</td></tr><tr><td>bunjs</td><td>1.98</td><td>0.49</td><td>12.01</td><td>629</td></tr><tr><td>nodejs</td><td>3.12</td><td>0.71</td><td>20.66</td><td>684</td></tr></table>

<table><tr><th></th><th>c</th><th>bunjs</th><th>nodejs</th></tr><tr><th>c</th><td style=''></td><td style='color: red;'>-0.36</td><td style='color: red;'>-1.5</td></tr><tr><th>bunjs</th><td style='color: green;'>0.36</td><td style=''></td><td style='color: red;'>-1.14</td></tr><tr><th>nodejs</th><td style='color: green;'>1.5</td><td style='color: green;'>1.14</td><td style=''></td></tr></table>

### Singlethreaded

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th></tr><tr><td>bunjs</td><td>8.49</td><td>0.14</td><td>8.35</td><td>100</td></tr><tr><td>c</td><td>9.93</td><td>0.13</td><td>9.76</td><td>99</td></tr><tr><td>nodejs</td><td>14.31</td><td>0.38</td><td>13.93</td><td>99</td></tr></table>

<table><tr><th></th><th>bunjs</th><th>c</th><th>nodejs</th></tr><tr><th>bunjs</th><td style=''></td><td style='color: red;'>-1.44</td><td style='color: red;'>-5.82</td></tr><tr><th>c</th><td style='color: green;'>1.44</td><td style=''></td><td style='color: red;'>-4.38</td></tr><tr><th>nodejs</th><td style='color: green;'>5.82</td><td style='color: green;'>4.38</td><td style=''></td></tr></table>

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
