# LangBench

LangBench is a simple benchmarking tool that processes numeric data and computes a mathematical result.

## Benchmark

### Hardware Specifications

**Processor**: arm

**Cores**: 8

### Data

**Size**: 534 MB

### Results

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th></tr><tr><td>c-pthread</td><td>1.59</td><td>0.11</td><td>11.48</td><td>728</td></tr><tr><td>bunjs-worker</td><td>1.9</td><td>0.44</td><td>12.08</td><td>658</td></tr><tr><td>nodejs-worker</td><td>2.97</td><td>0.65</td><td>19.45</td><td>675</td></tr><tr><td>c</td><td>8.32</td><td>0.08</td><td>8.23</td><td>99</td></tr><tr><td>bunjs</td><td>8.46</td><td>0.13</td><td>8.33</td><td>100</td></tr><tr><td>nodejs</td><td>14.44</td><td>0.43</td><td>14.01</td><td>100</td></tr></table>

### Comparison

<table><tr><th></th><th>c-pthread</th><th>bunjs-worker</th><th>nodejs-worker</th><th>c</th><th>bunjs</th><th>nodejs</th></tr><tr><th>c-pthread</th><td style=''></td><td style='color: red;'>-0.31</td><td style='color: red;'>-1.38</td><td style='color: red;'>-6.73</td><td style='color: red;'>-6.87</td><td style='color: red;'>-12.85</td></tr><tr><th>bunjs-worker</th><td style='color: green;'>0.31</td><td style=''></td><td style='color: red;'>-1.07</td><td style='color: red;'>-6.42</td><td style='color: red;'>-6.56</td><td style='color: red;'>-12.54</td></tr><tr><th>nodejs-worker</th><td style='color: green;'>1.38</td><td style='color: green;'>1.07</td><td style=''></td><td style='color: red;'>-5.35</td><td style='color: red;'>-5.49</td><td style='color: red;'>-11.47</td></tr><tr><th>c</th><td style='color: green;'>6.73</td><td style='color: green;'>6.42</td><td style='color: green;'>5.35</td><td style=''></td><td style='color: red;'>-0.14</td><td style='color: red;'>-6.12</td></tr><tr><th>bunjs</th><td style='color: green;'>6.87</td><td style='color: green;'>6.56</td><td style='color: green;'>5.49</td><td style='color: green;'>0.14</td><td style=''></td><td style='color: red;'>-5.98</td></tr><tr><th>nodejs</th><td style='color: green;'>12.85</td><td style='color: green;'>12.54</td><td style='color: green;'>11.47</td><td style='color: green;'>6.12</td><td style='color: green;'>5.98</td><td style=''></td></tr></table>

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
