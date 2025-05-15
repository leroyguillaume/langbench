# mt-mergesort

Merge Sort algorithm implementation running on multiple threads.

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
    <td>0.03</td>
    <td>0.01</td>
    <td>0.11</td>
    <td>334.0</td>
    <td>15.65</td>
  </tr>
  <tr>
    <td>java</td>
    <td>0.07</td>
    <td>0.02</td>
    <td>0.22</td>
    <td>320.0</td>
    <td>126.64</td>
  </tr>
  <tr>
    <td>bun</td>
    <td>0.3</td>
    <td>0.16</td>
    <td>1.43</td>
    <td>528.0</td>
    <td>432.21</td>
  </tr>
  <tr>
    <td>nodejs</td>
    <td>0.33</td>
    <td>0.15</td>
    <td>1.13</td>
    <td>390.0</td>
    <td>331.85</td>
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
    <td>42.86%</td>
    <td>10.0%</td>
    <td>9.09%</td>
  </tr>
  <tr>
    <th>java</th>
    <td>233.33%</td>
    <td>100.0%</td>
    <td>23.33%</td>
    <td>21.21%</td>
  </tr>
  <tr>
    <th>bun</th>
    <td>1000.0%</td>
    <td>428.57%</td>
    <td>100.0%</td>
    <td>90.91%</td>
  </tr>
  <tr>
    <th>nodejs</th>
    <td>1100.0%</td>
    <td>471.43%</td>
    <td>110.0%</td>
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
    <td>12.36%</td>
    <td>3.62%</td>
    <td>4.72%</td>
  </tr>
  <tr>
    <th>java</th>
    <td>809.11%</td>
    <td>100.0%</td>
    <td>29.3%</td>
    <td>38.16%</td>
  </tr>
  <tr>
    <th>bun</th>
    <td>2761.29%</td>
    <td>341.28%</td>
    <td>100.0%</td>
    <td>130.24%</td>
  </tr>
  <tr>
    <th>nodejs</th>
    <td>2120.14%</td>
    <td>262.03%</td>
    <td>76.78%</td>
    <td>100.0%</td>
  </tr>
</table>
