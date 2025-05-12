# LangBench

LangBench is a simple benchmarking tool that processes numeric data and computes a mathematical result.

## Benchmark

### Hardware Specifications

**Processor**: x86_64

**Cores**: 4

### Data

**Size**: 534 MB

**Number of iterations:** 10

### Multithreaded

#### Results

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th><th>Max Memory (MB)</th></tr><tr><td>java</td><td>1.96</td><td>0.07</td><td>7.32</td><td>377</td><td>590.45</td></tr><tr><td>c</td><td>3.07</td><td>0.04</td><td>12.14</td><td>396</td><td>536.13</td></tr><tr><td>rust</td><td>3.16</td><td>0.04</td><td>12.36</td><td>392</td><td>536.57</td></tr><tr><td>bunjs</td><td>3.52</td><td>0.34</td><td>12.62</td><td>368</td><td>600.09</td></tr><tr><td>nodejs</td><td>5.87</td><td>0.51</td><td>21.52</td><td>375</td><td>1443.02</td></tr><tr><td>python3.11</td><td>49.9</td><td>2.37</td><td>47.22</td><td>99</td><td>6970.39</td></tr></table>

#### CPU Usage Comparison

*A value of 100% indicates equal performance between the row and column languages.*

*A value of 50% indicates that the row language performs the computation twice as fast as the column language.*

<table><tr><th></th><th>java</th><th>c</th><th>rust</th><th>bunjs</th><th>nodejs</th><th>python3.11</th></tr><tr><th>java</th><td>100.0%</td><td>63.84%</td><td>62.03%</td><td>55.68%</td><td>33.39%</td><td>3.93%</td></tr><tr><th>c</th><td>156.63%</td><td>100.0%</td><td>97.15%</td><td>87.22%</td><td>52.3%</td><td>6.15%</td></tr><tr><th>rust</th><td>161.22%</td><td>102.93%</td><td>100.0%</td><td>89.77%</td><td>53.83%</td><td>6.33%</td></tr><tr><th>bunjs</th><td>179.59%</td><td>114.66%</td><td>111.39%</td><td>100.0%</td><td>59.97%</td><td>7.05%</td></tr><tr><th>nodejs</th><td>299.49%</td><td>191.21%</td><td>185.76%</td><td>166.76%</td><td>100.0%</td><td>11.76%</td></tr><tr><th>python3.11</th><td>2545.92%</td><td>1625.41%</td><td>1579.11%</td><td>1417.61%</td><td>850.09%</td><td>100.0%</td></tr></table>

#### Memory Usage Comparison

*A value of 100% indicates equal memory usage between the row and column languages.*

*A value of 50% indicates that the row language uses half the memory of the column language.*

<table><tr><th></th><th>java</th><th>c</th><th>rust</th><th>bunjs</th><th>nodejs</th><th>python3.11</th></tr><tr><th>java</th><td>100.0%</td><td>110.13%</td><td>110.04%</td><td>98.39%</td><td>40.92%</td><td>8.47%</td></tr><tr><th>c</th><td>90.8%</td><td>100.0%</td><td>99.92%</td><td>89.34%</td><td>37.15%</td><td>7.69%</td></tr><tr><th>rust</th><td>90.87%</td><td>100.08%</td><td>100.0%</td><td>89.41%</td><td>37.18%</td><td>7.7%</td></tr><tr><th>bunjs</th><td>101.63%</td><td>111.93%</td><td>111.84%</td><td>100.0%</td><td>41.59%</td><td>8.61%</td></tr><tr><th>nodejs</th><td>244.39%</td><td>269.15%</td><td>268.93%</td><td>240.47%</td><td>100.0%</td><td>20.7%</td></tr><tr><th>python3.11</th><td>1180.52%</td><td>1300.13%</td><td>1299.06%</td><td>1161.56%</td><td>483.04%</td><td>100.0%</td></tr></table>

### Singlethreaded

#### Results

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th><th>Max Memory (MB)</th></tr><tr><td>java</td><td>5.11</td><td>0.07</td><td>5.12</td><td>101</td><td>587.81</td></tr><tr><td>c</td><td>9.89</td><td>0.03</td><td>9.85</td><td>99</td><td>535.98</td></tr><tr><td>rust</td><td>10.1</td><td>0.03</td><td>10.06</td><td>99</td><td>536.26</td></tr><tr><td>bunjs</td><td>10.5</td><td>0.31</td><td>10.2</td><td>100</td><td>580.01</td></tr><tr><td>nodejs</td><td>16.84</td><td>0.27</td><td>16.57</td><td>100</td><td>593.15</td></tr><tr><td>python3.11</td><td>47.65</td><td>2.17</td><td>44.93</td><td>98</td><td>5901.09</td></tr></table>

#### CPU Usage Comparison

*A value of 100% indicates equal performance between the row and column languages.*

*A value of 50% indicates that the row language performs the computation twice as fast as the column language.*

<table><tr><th></th><th>java</th><th>c</th><th>rust</th><th>bunjs</th><th>nodejs</th><th>python3.11</th></tr><tr><th>java</th><td>100.0%</td><td>51.67%</td><td>50.59%</td><td>48.67%</td><td>30.34%</td><td>10.72%</td></tr><tr><th>c</th><td>193.54%</td><td>100.0%</td><td>97.92%</td><td>94.19%</td><td>58.73%</td><td>20.76%</td></tr><tr><th>rust</th><td>197.65%</td><td>102.12%</td><td>100.0%</td><td>96.19%</td><td>59.98%</td><td>21.2%</td></tr><tr><th>bunjs</th><td>205.48%</td><td>106.17%</td><td>103.96%</td><td>100.0%</td><td>62.35%</td><td>22.04%</td></tr><tr><th>nodejs</th><td>329.55%</td><td>170.27%</td><td>166.73%</td><td>160.38%</td><td>100.0%</td><td>35.34%</td></tr><tr><th>python3.11</th><td>932.49%</td><td>481.8%</td><td>471.78%</td><td>453.81%</td><td>282.96%</td><td>100.0%</td></tr></table>

#### Memory Usage Comparison

*A value of 100% indicates equal memory usage between the row and column languages.*

*A value of 50% indicates that the row language uses half the memory of the column language.*

<table><tr><th></th><th>java</th><th>c</th><th>rust</th><th>bunjs</th><th>nodejs</th><th>python3.11</th></tr><tr><th>java</th><td>100.0%</td><td>109.67%</td><td>109.61%</td><td>101.34%</td><td>99.1%</td><td>9.96%</td></tr><tr><th>c</th><td>91.18%</td><td>100.0%</td><td>99.95%</td><td>92.41%</td><td>90.36%</td><td>9.08%</td></tr><tr><th>rust</th><td>91.23%</td><td>100.05%</td><td>100.0%</td><td>92.46%</td><td>90.41%</td><td>9.09%</td></tr><tr><th>bunjs</th><td>98.67%</td><td>108.21%</td><td>108.16%</td><td>100.0%</td><td>97.78%</td><td>9.83%</td></tr><tr><th>nodejs</th><td>100.91%</td><td>110.67%</td><td>110.61%</td><td>102.27%</td><td>100.0%</td><td>10.05%</td></tr><tr><th>python3.11</th><td>1003.91%</td><td>1100.99%</td><td>1100.42%</td><td>1017.41%</td><td>994.87%</td><td>100.0%</td></tr></table>

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
