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
    <td>316.6</td>
    <td>96.52</td>
  </tr>
  <tr>
    <td>rust</td>
    <td>0.46</td>
    <td>0.08</td>
    <td>1.45</td>
    <td>331.6</td>
    <td>130.42</td>
  </tr>
  <tr>
    <td>java</td>
    <td>0.68</td>
    <td>0.17</td>
    <td>1.13</td>
    <td>197.0</td>
    <td>432.63</td>
  </tr>
  <tr>
    <td>bun</td>
    <td>1.84</td>
    <td>0.28</td>
    <td>6.49</td>
    <td>368.0</td>
    <td>414.15</td>
  </tr>
  <tr>
    <td>nodejs</td>
    <td>3.22</td>
    <td>0.91</td>
    <td>7.75</td>
    <td>268.0</td>
    <td>845.59</td>
  </tr>
  <tr>
    <td>python</td>
    <td>109.71</td>
    <td>0.34</td>
    <td>169.36</td>
    <td>154.0</td>
    <td>371.01</td>
  </tr>
</table>

## Time comparison

<table>
  <tr>
    <th></th>
    <th>c</th>
    <th>rust</th>
    <th>java</th>
    <th>bun</th>
    <th>nodejs</th>
    <th>python</th>
  </tr>
  <tr>
    <th>c</th>
    <td>100.0%</td>
    <td>73.8%</td>
    <td>50.0%</td>
    <td>18.41%</td>
    <td>10.48%</td>
    <td>0.31%</td>
  </tr>
  <tr>
    <th>rust</th>
    <td>135.5%</td>
    <td>100.0%</td>
    <td>67.75%</td>
    <td>24.95%</td>
    <td>14.21%</td>
    <td>0.42%</td>
  </tr>
  <tr>
    <th>java</th>
    <td>200.0%</td>
    <td>147.6%</td>
    <td>100.0%</td>
    <td>36.82%</td>
    <td>20.97%</td>
    <td>0.62%</td>
  </tr>
  <tr>
    <th>bun</th>
    <td>543.2%</td>
    <td>400.87%</td>
    <td>271.6%</td>
    <td>100.0%</td>
    <td>56.95%</td>
    <td>1.67%</td>
  </tr>
  <tr>
    <th>nodejs</th>
    <td>953.85%</td>
    <td>703.93%</td>
    <td>476.92%</td>
    <td>175.6%</td>
    <td>100.0%</td>
    <td>2.94%</td>
  </tr>
  <tr>
    <th>python</th>
    <td>32459.76%</td>
    <td>23955.02%</td>
    <td>16229.88%</td>
    <td>5975.71%</td>
    <td>3403.04%</td>
    <td>100.0%</td>
  </tr>
</table>

## Memory comparison

<table>
  <tr>
    <th></th>
    <th>c</th>
    <th>rust</th>
    <th>java</th>
    <th>bun</th>
    <th>nodejs</th>
    <th>python</th>
  </tr>
  <tr>
    <th>c</th>
    <td>100.0%</td>
    <td>74.0%</td>
    <td>22.31%</td>
    <td>23.3%</td>
    <td>11.41%</td>
    <td>26.01%</td>
  </tr>
  <tr>
    <th>rust</th>
    <td>135.13%</td>
    <td>100.0%</td>
    <td>30.15%</td>
    <td>31.49%</td>
    <td>15.42%</td>
    <td>35.15%</td>
  </tr>
  <tr>
    <th>java</th>
    <td>448.24%</td>
    <td>331.71%</td>
    <td>100.0%</td>
    <td>104.46%</td>
    <td>51.16%</td>
    <td>116.61%</td>
  </tr>
  <tr>
    <th>bun</th>
    <td>429.09%</td>
    <td>317.54%</td>
    <td>95.73%</td>
    <td>100.0%</td>
    <td>48.98%</td>
    <td>111.63%</td>
  </tr>
  <tr>
    <th>nodejs</th>
    <td>876.09%</td>
    <td>648.34%</td>
    <td>195.45%</td>
    <td>204.17%</td>
    <td>100.0%</td>
    <td>227.92%</td>
  </tr>
  <tr>
    <th>python</th>
    <td>384.39%</td>
    <td>284.46%</td>
    <td>85.76%</td>
    <td>89.58%</td>
    <td>43.88%</td>
    <td>100.0%</td>
  </tr>
</table>
