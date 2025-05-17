# mt-mergesort

Merge Sort algorithm implementation running on multiple threads.

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
    <td>0.3</td>
    <td>0.06</td>
    <td>1.4</td>
    <td>488.4</td>
    <td>88.32</td>
  </tr>
  <tr>
    <td>java</td>
    <td>0.46</td>
    <td>0.1</td>
    <td>1.46</td>
    <td>342.8</td>
    <td>421.82</td>
  </tr>
  <tr>
    <td>bun</td>
    <td>1.48</td>
    <td>0.43</td>
    <td>9.45</td>
    <td>666.6</td>
    <td>643.59</td>
  </tr>
  <tr>
    <td>nodejs</td>
    <td>2.57</td>
    <td>0.73</td>
    <td>9.51</td>
    <td>398.2</td>
    <td>1406.94</td>
  </tr>
  <tr>
    <td>python</td>
    <td>65.95</td>
    <td>0.52</td>
    <td>130.14</td>
    <td>197.6</td>
    <td>352.02</td>
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
    <td>64.07%</td>
    <td>20.0%</td>
    <td>11.54%</td>
    <td>0.45%</td>
  </tr>
  <tr>
    <th>java</th>
    <td>156.08%</td>
    <td>100.0%</td>
    <td>31.22%</td>
    <td>18.0%</td>
    <td>0.7%</td>
  </tr>
  <tr>
    <th>bun</th>
    <td>500.0%</td>
    <td>320.35%</td>
    <td>100.0%</td>
    <td>57.68%</td>
    <td>2.24%</td>
  </tr>
  <tr>
    <th>nodejs</th>
    <td>866.89%</td>
    <td>555.41%</td>
    <td>173.38%</td>
    <td>100.0%</td>
    <td>3.89%</td>
  </tr>
  <tr>
    <th>python</th>
    <td>22280.41%</td>
    <td>14274.89%</td>
    <td>4456.08%</td>
    <td>2570.15%</td>
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
    <td>20.94%</td>
    <td>13.72%</td>
    <td>6.28%</td>
    <td>25.09%</td>
  </tr>
  <tr>
    <th>java</th>
    <td>477.61%</td>
    <td>100.0%</td>
    <td>65.54%</td>
    <td>29.98%</td>
    <td>119.83%</td>
  </tr>
  <tr>
    <th>bun</th>
    <td>728.72%</td>
    <td>152.58%</td>
    <td>100.0%</td>
    <td>45.74%</td>
    <td>182.83%</td>
  </tr>
  <tr>
    <th>nodejs</th>
    <td>1593.03%</td>
    <td>333.54%</td>
    <td>218.61%</td>
    <td>100.0%</td>
    <td>399.68%</td>
  </tr>
  <tr>
    <th>python</th>
    <td>398.58%</td>
    <td>83.45%</td>
    <td>54.7%</td>
    <td>25.02%</td>
    <td>100.0%</td>
  </tr>
</table>
