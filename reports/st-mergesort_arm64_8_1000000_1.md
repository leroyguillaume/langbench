# st-mergesort

Merge Sort algorithm implementation running on a single thread.

**Architecture:** arm64

**CPU cores:** 8

**Count:** 1000000

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
    <td>0.09</td>
    <td>0.0</td>
    <td>0.09</td>
    <td>98.0</td>
    <td>8.83</td>
  </tr>
  <tr>
    <td>java</td>
    <td>0.14</td>
    <td>0.04</td>
    <td>0.13</td>
    <td>116.0</td>
    <td>119.44</td>
  </tr>
  <tr>
    <td>bun</td>
    <td>0.16</td>
    <td>0.04</td>
    <td>0.13</td>
    <td>104.0</td>
    <td>95.27</td>
  </tr>
  <tr>
    <td>nodejs</td>
    <td>0.27</td>
    <td>0.03</td>
    <td>0.25</td>
    <td>105.0</td>
    <td>59.62</td>
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
    <td>64.29%</td>
    <td>56.25%</td>
    <td>33.33%</td>
  </tr>
  <tr>
    <th>java</th>
    <td>155.56%</td>
    <td>100.0%</td>
    <td>87.5%</td>
    <td>51.85%</td>
  </tr>
  <tr>
    <th>bun</th>
    <td>177.78%</td>
    <td>114.29%</td>
    <td>100.0%</td>
    <td>59.26%</td>
  </tr>
  <tr>
    <th>nodejs</th>
    <td>300.0%</td>
    <td>192.86%</td>
    <td>168.75%</td>
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
    <td>7.39%</td>
    <td>9.27%</td>
    <td>14.81%</td>
  </tr>
  <tr>
    <th>java</th>
    <td>1352.32%</td>
    <td>100.0%</td>
    <td>125.37%</td>
    <td>200.33%</td>
  </tr>
  <tr>
    <th>bun</th>
    <td>1078.64%</td>
    <td>79.76%</td>
    <td>100.0%</td>
    <td>159.79%</td>
  </tr>
  <tr>
    <th>nodejs</th>
    <td>675.06%</td>
    <td>49.92%</td>
    <td>62.58%</td>
    <td>100.0%</td>
  </tr>
</table>
