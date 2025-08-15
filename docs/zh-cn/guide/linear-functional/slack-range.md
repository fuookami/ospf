# 松弛（范围）

## 形式

$$
y = slack \_ range(x, \, lb, \, ub) = \begin{cases}
max(0, \, x - ub), & 计算正松弛\\
max(0, \, lb - x), & 计算负松弛\\
min(|x - ub|, \, |x - lb|), & 计算正负松弛
\end{cases}
$$
