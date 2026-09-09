# Demo5 路线编译上下文领域模型

[toc]

## 1. 概述

Route compilation 上下文把候选路线编译为客户覆盖和车队规模主问题，并接收路线生成上下文的新列。

### 1. 依赖上下文

1. VRPTW 上下文提供客户覆盖关系和路线成本。
2. Route generation 提供候选路线列。

## 2. 概念 / 实体

### 1. Route column

路线列 $r$ 包含成本 $c_r$、车辆类型和客户覆盖系数 $a_{ir}$。

### 2. Artificial coverage

人工覆盖变量用于 Phase I，帮助主问题从不可行初始列开始求解。

## 3. 变量

### 1. 决策变量

**$x_r$**：选择路线 $r$ 的数量，无量纲计数，域为 $\mathbb Z_{\ge0}$，$\forall r\in R$。

### 2. 辅助变量

**$a_i$**：客户 $i$ 的人工覆盖量，非负连续量，$\forall i\in C$。

## 4. 谓词

**Covers**：路线 $r$ 覆盖客户 $i$。

**ArtificiallyCovered**：客户由 Phase I 人工变量覆盖。

## 5. 集合

**$C$**：客户集合；**$R$**：当前路线列集合；**$V$**：车辆类型集合。

**$R_i$**：覆盖客户 $i$ 的路线集合；**$R_v$**：使用车辆类型 $v$ 的路线集合。

## 6. 中间值

### 1. 客户覆盖

$$
coverage_i=\sum_{r\in R_i}x_r,\qquad i\in C.
$$

### 2. 车辆使用量

$$
fleet_v=\sum_{r\in R_v}x_r,\qquad v\in V.
$$

### 3. 路线目标成本

$$
routeCost=\sum_{r\in R}c_rx_r.
$$

## 7. 断言

### 1. 覆盖系数二值

$$
\forall(i,r)\in C\times R\;(a_{ir}\in\{0,1\}).
$$

### 2. 路线成本非负

$$
\forall r\in R\;(c_r\ge0).
$$

## 8. 约束

### 1. 客户覆盖 [Customer Coverage]

**说明**：每个客户必须由一条有效路线覆盖；Phase I 允许暂时使用人工覆盖。

$$
s.t.\quad \sum_{r\in R}a_{ir}x_r+a_i=1,\qquad \forall i\in C.
$$

### 2. 车队规模 [Fleet Size]

$$
s.t.\quad \sum_{r\in R_v}x_r\le fleetLimit_v,\qquad \forall v\in V.
$$

### 3. 整数域 [Route Integrality]

$$
s.t.\quad x_r\in\mathbb Z_{\ge0},\qquad \forall r\in R.
$$

## 9. 目标函数

Phase I 惩罚人工覆盖；Phase II 最小化路线成本：

$$
\min\;M\sum_{i\in C}a_i+\sum_{r\in R}c_rx_r.
$$

## 10. 算法引用

| 算法 | 用途 |
|---|---|
| Column generation | 将负约化成本路线加入主问题 |
| Branch-and-price | 在分支节点继续求解受限主问题 |

## 11. 统一语言

| 术语 | 符号 | 定义 |
|---|---|---|
| 路线列 | $r$ | 主问题中的一列 |
| 客户覆盖 | $coverage_i$ | 客户被路线覆盖的数量 |
| 人工覆盖 | $a_i$ | Phase I 的不可行性补偿 |

## 12. 设计决策

| 决策 | 依据 |
|---|---|
| 分 Phase I/II 目标 | 先消除人工覆盖，再优化路线成本 |

## 13. 变更记录

| 版本 | 变更 | 原因 |
|---|---|---|
| 1.0 | 翻译并补齐路线编译模型 | 中文页面与当前主问题一致 |

## 源码与验证

[路线编译上下文源码](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-framework-network-scheduling/network-scheduling-domain-route-compilation-context)
