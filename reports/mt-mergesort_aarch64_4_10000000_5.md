# mt-mergesort

Merge Sort algorithm implementation running on multiple threads.

**Architecture:** aarch64

**CPU cores:** 4

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
    <td>0.34</td>
    <td>0.07</td>
    <td>1.01</td>
    <td>316.2</td>
    <td>96.52</td>
  </tr>
  <tr>
    <td>java</td>
    <td>0.52</td>
    <td>0.16</td>
    <td>1.13</td>
    <td>257.0</td>
    <td>417.58</td>
  </tr>
  <tr>
    <td>rust</td>
    <td>0.62</td>
    <td>0.1</td>
    <td>1.59</td>
    <td>270.2</td>
    <td>168.57</td>
  </tr>
  <tr>
    <td>bun</td>
    <td>1.85</td>
    <td>0.27</td>
    <td>6.31</td>
    <td>357.2</td>
    <td>414.87</td>
  </tr>
  <tr>
    <td>nodejs</td>
    <td>3.22</td>
    <td>0.9</td>
    <td>7.66</td>
    <td>265.2</td>
    <td>838.53</td>
  </tr>
  <tr>
    <td>python</td>
    <td>109.72</td>
    <td>0.33</td>
    <td>169.34</td>
    <td>154.0</td>
    <td>371.01</td>
  </tr>
</table>

## Time comparison

<table>
  <tr>
    <th></th>
    <th>c</th>
    <th>java</th>
    <th>rust</th>
    <th>bun</th>
    <th>nodejs</th>
    <th>python</th>
  </tr>
  <tr>
    <th>c</th>
    <td>100.0%</td>
    <td>65.5%</td>
    <td>54.17%</td>
    <td>18.31%</td>
    <td>10.49%</td>
    <td>0.31%</td>
  </tr>
  <tr>
    <th>java</th>
    <td>152.66%</td>
    <td>100.0%</td>
    <td>82.69%</td>
    <td>27.95%</td>
    <td>16.01%</td>
    <td>0.47%</td>
  </tr>
  <tr>
    <th>rust</th>
    <td>184.62%</td>
    <td>120.93%</td>
    <td>100.0%</td>
    <td>33.8%</td>
    <td>19.37%</td>
    <td>0.57%</td>
  </tr>
  <tr>
    <th>bun</th>
    <td>546.15%</td>
    <td>357.75%</td>
    <td>295.83%</td>
    <td>100.0%</td>
    <td>57.29%</td>
    <td>1.68%</td>
  </tr>
  <tr>
    <th>nodejs</th>
    <td>953.25%</td>
    <td>624.42%</td>
    <td>516.35%</td>
    <td>174.54%</td>
    <td>100.0%</td>
    <td>2.94%</td>
  </tr>
  <tr>
    <th>python</th>
    <td>32460.36%</td>
    <td>21262.79%</td>
    <td>17582.69%</td>
    <td>5943.45%</td>
    <td>3405.21%</td>
    <td>100.0%</td>
  </tr>
</table>

## Memory comparison

<table>
  <tr>
    <th></th>
    <th>c</th>
    <th>java</th>
    <th>rust</th>
    <th>bun</th>
    <th>nodejs</th>
    <th>python</th>
  </tr>
  <tr>
    <th>c</th>
    <td>100.0%</td>
    <td>23.11%</td>
    <td>57.26%</td>
    <td>23.26%</td>
    <td>11.51%</td>
    <td>26.01%</td>
  </tr>
  <tr>
    <th>java</th>
    <td>432.64%</td>
    <td>100.0%</td>
    <td>247.72%</td>
    <td>100.65%</td>
    <td>49.8%</td>
    <td>112.55%</td>
  </tr>
  <tr>
    <th>rust</th>
    <td>174.65%</td>
    <td>40.37%</td>
    <td>100.0%</td>
    <td>40.63%</td>
    <td>20.1%</td>
    <td>45.44%</td>
  </tr>
  <tr>
    <th>bun</th>
    <td>429.83%</td>
    <td>99.35%</td>
    <td>246.11%</td>
    <td>100.0%</td>
    <td>49.48%</td>
    <td>111.82%</td>
  </tr>
  <tr>
    <th>nodejs</th>
    <td>868.78%</td>
    <td>200.81%</td>
    <td>497.43%</td>
    <td>202.12%</td>
    <td>100.0%</td>
    <td>226.01%</td>
  </tr>
  <tr>
    <th>python</th>
    <td>384.39%</td>
    <td>88.85%</td>
    <td>220.09%</td>
    <td>89.43%</td>
    <td>44.25%</td>
    <td>100.0%</td>
  </tr>
</table>
