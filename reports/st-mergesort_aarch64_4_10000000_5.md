# st-mergesort

Merge Sort algorithm implementation running on a single thread.

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
    <td>1.03</td>
    <td>0.06</td>
    <td>0.97</td>
    <td>99.0</td>
    <td>77.48</td>
  </tr>
  <tr>
    <td>java</td>
    <td>1.21</td>
    <td>0.12</td>
    <td>1.09</td>
    <td>100.2</td>
    <td>325.1</td>
  </tr>
  <tr>
    <td>bun</td>
    <td>1.34</td>
    <td>0.1</td>
    <td>1.27</td>
    <td>102.0</td>
    <td>233.12</td>
  </tr>
  <tr>
    <td>rust</td>
    <td>1.42</td>
    <td>0.07</td>
    <td>1.32</td>
    <td>98.0</td>
    <td>131.92</td>
  </tr>
  <tr>
    <td>nodejs</td>
    <td>3.49</td>
    <td>0.3</td>
    <td>3.68</td>
    <td>113.6</td>
    <td>157.78</td>
  </tr>
  <tr>
    <td>python</td>
    <td>92.73</td>
    <td>0.12</td>
    <td>92.56</td>
    <td>99.0</td>
    <td>123.76</td>
  </tr>
</table>

## Time comparison

<table>
  <tr>
    <th></th>
    <th>c</th>
    <th>java</th>
    <th>bun</th>
    <th>rust</th>
    <th>nodejs</th>
    <th>python</th>
  </tr>
  <tr>
    <th>c</th>
    <td>100.0%</td>
    <td>85.45%</td>
    <td>77.16%</td>
    <td>72.71%</td>
    <td>29.61%</td>
    <td>1.12%</td>
  </tr>
  <tr>
    <th>java</th>
    <td>117.02%</td>
    <td>100.0%</td>
    <td>90.3%</td>
    <td>85.09%</td>
    <td>34.65%</td>
    <td>1.3%</td>
  </tr>
  <tr>
    <th>bun</th>
    <td>129.59%</td>
    <td>110.74%</td>
    <td>100.0%</td>
    <td>94.23%</td>
    <td>38.37%</td>
    <td>1.45%</td>
  </tr>
  <tr>
    <th>rust</th>
    <td>137.52%</td>
    <td>117.52%</td>
    <td>106.12%</td>
    <td>100.0%</td>
    <td>40.72%</td>
    <td>1.53%</td>
  </tr>
  <tr>
    <th>nodejs</th>
    <td>337.72%</td>
    <td>288.6%</td>
    <td>260.6%</td>
    <td>245.57%</td>
    <td>100.0%</td>
    <td>3.77%</td>
  </tr>
  <tr>
    <th>python</th>
    <td>8967.89%</td>
    <td>7663.47%</td>
    <td>6920.0%</td>
    <td>6520.96%</td>
    <td>2655.44%</td>
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
    <th>rust</th>
    <th>nodejs</th>
    <th>python</th>
  </tr>
  <tr>
    <th>c</th>
    <td>100.0%</td>
    <td>23.83%</td>
    <td>33.24%</td>
    <td>58.74%</td>
    <td>49.11%</td>
    <td>62.61%</td>
  </tr>
  <tr>
    <th>java</th>
    <td>419.58%</td>
    <td>100.0%</td>
    <td>139.45%</td>
    <td>246.44%</td>
    <td>206.05%</td>
    <td>262.69%</td>
  </tr>
  <tr>
    <th>bun</th>
    <td>300.88%</td>
    <td>71.71%</td>
    <td>100.0%</td>
    <td>176.72%</td>
    <td>147.76%</td>
    <td>188.37%</td>
  </tr>
  <tr>
    <th>rust</th>
    <td>170.25%</td>
    <td>40.58%</td>
    <td>56.59%</td>
    <td>100.0%</td>
    <td>83.61%</td>
    <td>106.59%</td>
  </tr>
  <tr>
    <th>nodejs</th>
    <td>203.63%</td>
    <td>48.53%</td>
    <td>67.68%</td>
    <td>119.6%</td>
    <td>100.0%</td>
    <td>127.49%</td>
  </tr>
  <tr>
    <th>python</th>
    <td>159.73%</td>
    <td>38.07%</td>
    <td>53.09%</td>
    <td>93.82%</td>
    <td>78.44%</td>
    <td>100.0%</td>
  </tr>
</table>
