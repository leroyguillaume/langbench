# st-mergesort

Merge Sort algorithm implementation running on a single thread.

**Architecture:** arm64

**CPU cores:** 8

**Count:** 10000000

**Iterations:** 1

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
    <td>1.07</td>
    <td>0.04</td>
    <td>1.03</td>
    <td>99.0</td>
    <td>77.55</td>
  </tr>
  <tr>
    <td>java</td>
    <td>1.16</td>
    <td>0.06</td>
    <td>1.11</td>
    <td>101.0</td>
    <td>350.82</td>
  </tr>
  <tr>
    <td>bun</td>
    <td>1.3</td>
    <td>0.09</td>
    <td>1.23</td>
    <td>101.0</td>
    <td>237.79</td>
  </tr>
  <tr>
    <td>nodejs</td>
    <td>2.71</td>
    <td>0.13</td>
    <td>2.69</td>
    <td>104.0</td>
    <td>157.12</td>
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
  </tr>
  <tr>
    <th>c</th>
    <td>100.0%</td>
    <td>92.24%</td>
    <td>82.31%</td>
    <td>39.48%</td>
  </tr>
  <tr>
    <th>java</th>
    <td>108.41%</td>
    <td>100.0%</td>
    <td>89.23%</td>
    <td>42.8%</td>
  </tr>
  <tr>
    <th>bun</th>
    <td>121.5%</td>
    <td>112.07%</td>
    <td>100.0%</td>
    <td>47.97%</td>
  </tr>
  <tr>
    <th>nodejs</th>
    <td>253.27%</td>
    <td>233.62%</td>
    <td>208.46%</td>
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
  </tr>
  <tr>
    <th>c</th>
    <td>100.0%</td>
    <td>22.1%</td>
    <td>32.61%</td>
    <td>49.35%</td>
  </tr>
  <tr>
    <th>java</th>
    <td>452.4%</td>
    <td>100.0%</td>
    <td>147.53%</td>
    <td>223.28%</td>
  </tr>
  <tr>
    <th>bun</th>
    <td>306.64%</td>
    <td>67.78%</td>
    <td>100.0%</td>
    <td>151.34%</td>
  </tr>
  <tr>
    <th>nodejs</th>
    <td>202.61%</td>
    <td>44.79%</td>
    <td>66.08%</td>
    <td>100.0%</td>
  </tr>
</table>
