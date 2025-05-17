# st-mergesort

Merge Sort algorithm implementation running on a single thread.

**Architecture:** arm64

**CPU cores:** 8

**Count:** 10000000

**Iterations:** 5

## Results

<table>
  <tr>
    <th>Language</th>
    <th>Elapsed time (s)</th>
    <th>System time (s)</th>
    <th>User time (s)</th>
    <th>CPU usage (%)</th>
    <th>Max memory (MB)</th>
  </tr>
  <tr>
    <td>c</td>
    <td>1.1</td>
    <td>0.04</td>
    <td>1.05</td>
    <td>99.4</td>
    <td>77.52</td>
  </tr>
  <tr>
    <td>java</td>
    <td>1.21</td>
    <td>0.09</td>
    <td>1.14</td>
    <td>100.8</td>
    <td>347.11</td>
  </tr>
  <tr>
    <td>bun</td>
    <td>1.28</td>
    <td>0.06</td>
    <td>1.23</td>
    <td>100.8</td>
    <td>235.58</td>
  </tr>
  <tr>
    <td>nodejs</td>
    <td>2.75</td>
    <td>0.1</td>
    <td>2.77</td>
    <td>104.0</td>
    <td>157.65</td>
  </tr>
  <tr>
    <td>python</td>
    <td>51.58</td>
    <td>0.1</td>
    <td>51.46</td>
    <td>99.0</td>
    <td>123.84</td>
  </tr>
</table>

## Time comparison

<table>
  <tr>
    <th></th>
    <th>c</th>
    <th>java</th>
    <th>bun</th>
    <th>nodejs</th>
    <th>python</th>
  </tr>
  <tr>
    <th>c</th>
    <td>100.0%</td>
    <td>90.76%</td>
    <td>86.07%</td>
    <td>39.94%</td>
    <td>2.13%</td>
  </tr>
  <tr>
    <th>java</th>
    <td>110.18%</td>
    <td>100.0%</td>
    <td>94.84%</td>
    <td>44.01%</td>
    <td>2.35%</td>
  </tr>
  <tr>
    <th>bun</th>
    <td>116.18%</td>
    <td>105.45%</td>
    <td>100.0%</td>
    <td>46.41%</td>
    <td>2.48%</td>
  </tr>
  <tr>
    <th>nodejs</th>
    <td>250.36%</td>
    <td>227.23%</td>
    <td>215.49%</td>
    <td>100.0%</td>
    <td>5.34%</td>
  </tr>
  <tr>
    <th>python</th>
    <td>4689.45%</td>
    <td>4256.11%</td>
    <td>4036.31%</td>
    <td>1873.06%</td>
    <td>100.0%</td>
  </tr>
</table>

## Memory comparison

<table>
  <tr>
    <th></th>
    <th>c</th>
    <th>java</th>
    <th>bun</th>
    <th>nodejs</th>
    <th>python</th>
  </tr>
  <tr>
    <th>c</th>
    <td>100.0%</td>
    <td>22.33%</td>
    <td>32.91%</td>
    <td>49.17%</td>
    <td>62.6%</td>
  </tr>
  <tr>
    <th>java</th>
    <td>447.78%</td>
    <td>100.0%</td>
    <td>147.34%</td>
    <td>220.18%</td>
    <td>280.3%</td>
  </tr>
  <tr>
    <th>bun</th>
    <td>303.9%</td>
    <td>67.87%</td>
    <td>100.0%</td>
    <td>149.43%</td>
    <td>190.24%</td>
  </tr>
  <tr>
    <th>nodejs</th>
    <td>203.37%</td>
    <td>45.42%</td>
    <td>66.92%</td>
    <td>100.0%</td>
    <td>127.3%</td>
  </tr>
  <tr>
    <th>python</th>
    <td>159.75%</td>
    <td>35.68%</td>
    <td>52.57%</td>
    <td>78.55%</td>
    <td>100.0%</td>
  </tr>
</table>
