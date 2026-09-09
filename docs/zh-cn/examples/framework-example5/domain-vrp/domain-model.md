# Demo5 VRPTW 领域上下文模型

[toc]

## 1. 概述

VRPTW 上下文定义客户、车辆、仓库、路线和时间窗，并验证后续定价与路线编译使用的领域对象。

### 1. 依赖上下文

1. 输入适配器提供客户和车辆数据。
2. Route generation、Route compilation 消费已验证的路线对象。

## 2. 概念 / 实体

### 1. Customer

客户 $i$ 具有需求 $q_i$、服务时间 $s_i$ 和时间窗 $[e_i,l_i]$。

### 2. VehicleType

车辆类型 $v$ 具有容量 $Q_v$、固定成本 $f_v$ 和行驶速度 $speed_v$。

### 3. Route

路线 $r$ 是从仓库出发并返回仓库的客户序列。

## 3. 变量

### 1. 决策变量

VRP 上下文不直接注册主问题变量；Route compilation 注册路线选择变量 $x_r\in\mathbb Z_{\ge0}$。

### 2. 辅助变量

**$load_r$**：路线载荷，需求单位，$load_r\ge0$。

**$arrival_{i,r}$**：路线 $r$ 到达客户 $i$ 的时间，时间单位。

## 4. 谓词

**ValidRoute**：路线满足仓库、容量和时间窗规则。

**Serves**：路线 $r$ 包含客户 $i$。

## 5. 集合

**$C$**：客户集合；**$V$**：车辆类型集合；**$R$**：可行路线集合。

**$C_r$**：路线 $r$ 服务的客户子集；**$R_i$**：覆盖客户 $i$ 的路线集合。

## 6. 中间值

### 1. 行驶时间

$$
travelTime_{ijv}=\frac{distance_{ij}}{speed_v},\qquad (i,j)\in A,\ v\in V.
$$

### 2. 路线成本

$$
routeCost_r=f_{v(r)}+\sum_{(i,j)\in r}arcCost_{ijv(r)},\qquad r\in R.
$$

### 3. 路线载荷

$$
load_r=\sum_{i\in C_r}q_i,\qquad r\in R.
$$

## 7. 断言

### 1. 容量可行

$$
\forall r\in R\;(load_r\le Q_{v(r)}).
$$

### 2. 时间窗可行

$$
\forall(i,r)\;(i\in C_r\Rightarrow e_i\le arrival_{i,r}\le l_i).
$$

## 8. 约束

### 1. 路线容量 [Route Capacity]

**说明**：每条候选路线的客户需求不能超过对应车辆容量。

$$
s.t.\quad \sum_{i\in C_r}q_i\le Q_{v(r)},\qquad \forall r\in R.
$$

### 2. 路线时间窗 [Route Time Window]

**说明**：路线的到达时间必须落在客户服务时间窗内。

$$
s.t.\quad e_i\le arrival_{i,r}\le l_i,\qquad \forall r\in R,\ i\in C_r.
$$

## 9. 目标函数

VRP 上下文只计算路线成本；主问题由 Route compilation 最小化所选路线成本。

$$
\min\;\sum_{r\in R}routeCost_r x_r.
$$

## 10. 算法引用

| 算法 | 用途 |
|---|---|
| ESPPRC | 生成满足容量和时间窗的路线 |

## 11. 统一语言

| 术语 | 符号 | 定义 |
|---|---|---|
| 客户 | $i$ | 需要服务的节点 |
| 路线 | $r$ | 一条可行车辆行程 |
| 时间窗 | $[e_i,l_i]$ | 客户允许服务的时间区间 |

## 12. 设计决策

| 决策 | 依据 |
|---|---|
| 路线作为列传递 | 让生成和编译上下文共享同一可行性定义 |

## 13. 变更记录

| 版本 | 变更 | 原因 |
|---|---|---|
| 1.0 | 补齐 VRPTW 上下文数学模型 | 与当前框架实现保持一致 |

## 源码与验证

[VRP 上下文源码](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-framework-network-scheduling/network-scheduling-domain-vrp-context)
