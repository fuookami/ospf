# Demo5 路线生成上下文领域模型

[toc]

## 1. 概述

Route generation 上下文构建资源图，使用 ESPPRC 标签定价可行路线并向主问题返回新列。

### 1. 依赖上下文

1. VRPTW 上下文提供客户、车辆和可行性规则。
2. Route compilation 提供主问题对偶价格。

## 2. 概念 / 实体

### 1. Label

标签记录当前节点、已访问客户、载荷、时间和累计成本。

### 2. Pricing result

定价结果包含路线和约化成本，用于判断是否应加入受限主问题。

## 3. 变量

### 1. 决策变量

路线生成上下文不注册持久决策变量；标签扩展产生候选路线 $r$。

### 2. 辅助变量

**$d_r$**：路线 $r$ 的约化成本，成本单位。

**$load_r$**：标签路线载荷，需求单位，$load_r\ge0$。

## 4. 谓词

**Extendable**：标签扩展后仍满足容量和时间窗。

**ImprovingRoute**：路线约化成本小于零。

## 5. 集合

**$N$**：客户与仓库节点集合；**$A$**：有向弧集合；**$L$**：当前标签集合。

**$R^{-}$**：约化成本为负的路线集合。

## 6. 中间值

### 1. 标签约化成本

$$
d_r=routeCost_r-\sum_{i\in C}dual_i\,cover_{i,r},\qquad r\in R.
$$

### 2. 标签资源

$$
load_{\ell'}=load_\ell+q_j,\qquad time_{\ell'}=\max(e_j,time_\ell+service_\ell+travelTime_{ij}).
$$

## 7. 断言

### 1. 资源单调

$$
\forall \ell\in L\;(load_\ell\ge0\wedge time_\ell\ge0).
$$

### 2. 路线改进

$$
\forall r\in R^{-}\;(d_r<0).
$$

## 8. 约束

### 1. 容量扩展 [Capacity Extension]

$$
s.t.\quad load_\ell+q_j\le Q_v,\qquad \forall\ell\in L,\ j\in N.
$$

### 2. 时间窗扩展 [Time-window Extension]

$$
s.t.\quad arrival_j\le l_j,\qquad \forall j\in N.
$$

## 9. 目标函数

定价子问题寻找最小约化成本路线：

$$
\min_{r\in R}d_r.
$$

## 10. 算法引用

| 算法 | 用途 |
|---|---|
| ESPPRC | 在资源约束下枚举并定价路线 |

## 11. 统一语言

| 术语 | 符号 | 定义 |
|---|---|---|
| 标签 | $\ell$ | 路线扩展的状态 |
| 对偶价格 | $dual_i$ | 主问题覆盖约束的价格 |

## 12. 设计决策

| 决策 | 依据 |
|---|---|
| 只返回负约化成本列 | 与列生成停止条件一致 |

## 13. 变更记录

| 版本 | 变更 | 原因 |
|---|---|---|
| 1.0 | 翻译并补齐路线生成模型 | 中文页面与英文页面语义对应 |

## 源码与验证

[路线生成上下文源码](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-framework-network-scheduling/network-scheduling-domain-route-generation-context)
