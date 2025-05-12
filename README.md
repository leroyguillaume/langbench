# LangBench

LangBench is a simple benchmarking tool that processes numeric data and computes a mathematical result.

## Benchmark

### Hardware Specifications

**Processor**: arm

**Cores**: 8

### Data

**Size**: 534 MB

**Number of iterations:** 3

### Multithreaded

#### Results

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th><th>Max Memory (MB)</th></tr><tr><td>c</td><td>1.67</td><td>0.12</td><td>12.63</td><td>764</td><td>535.55</td></tr><tr><td>java</td><td>1.92</td><td>0.06</td><td>7.31</td><td>383</td><td>590.38</td></tr><tr><td>bunjs</td><td>3.51</td><td>0.33</td><td>12.62</td><td>368</td><td>600.42</td></tr><tr><td>rust</td><td>3.52</td><td>0.03</td><td>12.31</td><td>350</td><td>536.56</td></tr><tr><td>nodejs</td><td>5.85</td><td>0.49</td><td>21.52</td><td>375</td><td>1443.72</td></tr><tr><td>python3.11</td><td>49.39</td><td>2.34</td><td>46.66</td><td>99</td><td>6970.55</td></tr></table>

#### CPU Usage Comparison

*A value of 100% indicates equal performance between the row and column languages.*

*A value of 50% indicates that the row language performs the computation twice as fast as the column language.*

<table><tr><th></th><th>c</th><th>java</th><th>bunjs</th><th>rust</th><th>nodejs</th><th>python3.11</th></tr><tr><th>c</th><td>100.0%</td><td>86.98%</td><td>47.58%</td><td>47.44%</td><td>28.55%</td><td>3.38%</td></tr><tr><th>java</th><td>114.97%</td><td>100.0%</td><td>54.7%</td><td>54.55%</td><td>32.82%</td><td>3.89%</td></tr><tr><th>bunjs</th><td>210.18%</td><td>182.81%</td><td>100.0%</td><td>99.72%</td><td>60.0%</td><td>7.11%</td></tr><tr><th>rust</th><td>210.78%</td><td>183.33%</td><td>100.28%</td><td>100.0%</td><td>60.17%</td><td>7.13%</td></tr><tr><th>nodejs</th><td>350.3%</td><td>304.69%</td><td>166.67%</td><td>166.19%</td><td>100.0%</td><td>11.84%</td></tr><tr><th>python3.11</th><td>2957.49%</td><td>2572.4%</td><td>1407.12%</td><td>1403.12%</td><td>844.27%</td><td>100.0%</td></tr></table>

#### Memory Usage Comparison

*A value of 100% indicates equal memory usage between the row and column languages.*

*A value of 50% indicates that the row language uses half the memory of the column language.*

<table><tr><th></th><th>c</th><th>java</th><th>bunjs</th><th>rust</th><th>nodejs</th><th>python3.11</th></tr><tr><th>c</th><td>100.0%</td><td>90.71%</td><td>89.2%</td><td>99.81%</td><td>37.1%</td><td>7.68%</td></tr><tr><th>java</th><td>110.24%</td><td>100.0%</td><td>98.33%</td><td>110.03%</td><td>40.89%</td><td>8.47%</td></tr><tr><th>bunjs</th><td>112.11%</td><td>101.7%</td><td>100.0%</td><td>111.9%</td><td>41.59%</td><td>8.61%</td></tr><tr><th>rust</th><td>100.19%</td><td>90.88%</td><td>89.36%</td><td>100.0%</td><td>37.17%</td><td>7.7%</td></tr><tr><th>nodejs</th><td>269.58%</td><td>244.54%</td><td>240.45%</td><td>269.07%</td><td>100.0%</td><td>20.71%</td></tr><tr><th>python3.11</th><td>1301.57%</td><td>1180.69%</td><td>1160.95%</td><td>1299.12%</td><td>482.82%</td><td>100.0%</td></tr></table>

### Singlethreaded

#### Results

<table><tr><th>Language</th><th>Elapsed Time (s)</th><th>System Time (s)</th><th>User Time (s)</th><th>CPU Usage (%)</th><th>Max Memory (MB)</th></tr><tr><td>java</td><td>5.1</td><td>0.06</td><td>5.12</td><td>101</td><td>587.97</td></tr><tr><td>c</td><td>8.32</td><td>0.03</td><td>8.28</td><td>99</td><td>535.53</td></tr><tr><td>rust</td><td>10.12</td><td>0.04</td><td>10.07</td><td>99</td><td>536.19</td></tr><tr><td>bunjs</td><td>10.51</td><td>0.31</td><td>10.22</td><td>100</td><td>580.2</td></tr><tr><td>nodejs</td><td>16.84</td><td>0.26</td><td>16.57</td><td>100</td><td>593.42</td></tr><tr><td>python3.11</td><td>49.24</td><td>2.24</td><td>46.42</td><td>98</td><td>5900.95</td></tr></table>

#### CPU Usage Comparison

*A value of 100% indicates equal performance between the row and column languages.*

*A value of 50% indicates that the row language performs the computation twice as fast as the column language.*

<table><tr><th></th><th>java</th><th>c</th><th>rust</th><th>bunjs</th><th>nodejs</th><th>python3.11</th></tr><tr><th>java</th><td>100.0%</td><td>61.3%</td><td>50.4%</td><td>48.53%</td><td>30.29%</td><td>10.36%</td></tr><tr><th>c</th><td>163.14%</td><td>100.0%</td><td>82.21%</td><td>79.16%</td><td>49.41%</td><td>16.9%</td></tr><tr><th>rust</th><td>198.43%</td><td>121.63%</td><td>100.0%</td><td>96.29%</td><td>60.1%</td><td>20.55%</td></tr><tr><th>bunjs</th><td>206.08%</td><td>126.32%</td><td>103.85%</td><td>100.0%</td><td>62.41%</td><td>21.34%</td></tr><tr><th>nodejs</th><td>330.2%</td><td>202.4%</td><td>166.4%</td><td>160.23%</td><td>100.0%</td><td>34.2%</td></tr><tr><th>python3.11</th><td>965.49%</td><td>591.83%</td><td>486.56%</td><td>468.51%</td><td>292.4%</td><td>100.0%</td></tr></table>

#### Memory Usage Comparison

*A value of 100% indicates equal memory usage between the row and column languages.*

*A value of 50% indicates that the row language uses half the memory of the column language.*

<table><tr><th></th><th>java</th><th>c</th><th>rust</th><th>bunjs</th><th>nodejs</th><th>python3.11</th></tr><tr><th>java</th><td>100.0%</td><td>109.79%</td><td>109.66%</td><td>101.34%</td><td>99.08%</td><td>9.96%</td></tr><tr><th>c</th><td>91.08%</td><td>100.0%</td><td>99.88%</td><td>92.3%</td><td>90.24%</td><td>9.08%</td></tr><tr><th>rust</th><td>91.19%</td><td>100.12%</td><td>100.0%</td><td>92.41%</td><td>90.36%</td><td>9.09%</td></tr><tr><th>bunjs</th><td>98.68%</td><td>108.34%</td><td>108.21%</td><td>100.0%</td><td>97.77%</td><td>9.83%</td></tr><tr><th>nodejs</th><td>100.93%</td><td>110.81%</td><td>110.67%</td><td>102.28%</td><td>100.0%</td><td>10.06%</td></tr><tr><th>python3.11</th><td>1003.61%</td><td>1101.89%</td><td>1100.53%</td><td>1017.05%</td><td>994.4%</td><td>100.0%</td></tr></table>

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
