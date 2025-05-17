# mt-mergesort

Merge Sort algorithm implementation running on multiple threads.

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
    <td>0.29</td>
    <td>0.05</td>
    <td>1.36</td>
    <td>486.0</td>
    <td>85.6</td>
  </tr>
  <tr>
    <td>java</td>
    <td>0.45</td>
    <td>0.09</td>
    <td>1.43</td>
    <td>337.0</td>
    <td>391.02</td>
  </tr>
  <tr>
    <td>bun</td>
    <td>1.45</td>
    <td>0.41</td>
    <td>9.23</td>
    <td>661.0</td>
    <td>626.16</td>
  </tr>
  <tr>
    <td>nodejs</td>
    <td>2.47</td>
    <td>0.87</td>
    <td>9.29</td>
    <td>410.0</td>
    <td>1429.02</td>
  </tr>
  <tr>
    <td>python</td>
    <td>65.81</td>
    <td>0.47</td>
    <td>128.48</td>
    <td>195.0</td>
    <td>352.12</td>
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
    <td>64.44%</td>
    <td>20.0%</td>
    <td>11.74%</td>
    <td>0.44%</td>
  </tr>
  <tr>
    <th>java</th>
    <td>155.17%</td>
    <td>100.0%</td>
    <td>31.03%</td>
    <td>18.22%</td>
    <td>0.68%</td>
  </tr>
  <tr>
    <th>bun</th>
    <td>500.0%</td>
    <td>322.22%</td>
    <td>100.0%</td>
    <td>58.7%</td>
    <td>2.2%</td>
  </tr>
  <tr>
    <th>nodejs</th>
    <td>851.72%</td>
    <td>548.89%</td>
    <td>170.34%</td>
    <td>100.0%</td>
    <td>3.75%</td>
  </tr>
  <tr>
    <th>python</th>
    <td>22693.1%</td>
    <td>14624.44%</td>
    <td>4538.62%</td>
    <td>2664.37%</td>
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
    <td>21.89%</td>
    <td>13.67%</td>
    <td>5.99%</td>
    <td>24.31%</td>
  </tr>
  <tr>
    <th>java</th>
    <td>456.82%</td>
    <td>100.0%</td>
    <td>62.45%</td>
    <td>27.36%</td>
    <td>111.05%</td>
  </tr>
  <tr>
    <th>bun</th>
    <td>731.52%</td>
    <td>160.13%</td>
    <td>100.0%</td>
    <td>43.82%</td>
    <td>177.83%</td>
  </tr>
  <tr>
    <th>nodejs</th>
    <td>1669.46%</td>
    <td>365.46%</td>
    <td>228.22%</td>
    <td>100.0%</td>
    <td>405.84%</td>
  </tr>
  <tr>
    <th>python</th>
    <td>411.36%</td>
    <td>90.05%</td>
    <td>56.23%</td>
    <td>24.64%</td>
    <td>100.0%</td>
  </tr>
</table>
