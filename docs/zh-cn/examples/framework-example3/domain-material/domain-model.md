# Demo3 Material 上下文领域模型

[toc]

## 1. 概述

Material 上下文维护产品需求、原材料库存与单位消耗，向 Produce 上下文提供可验证的物料约束。

### 1. 依赖上下文

1. Produce 上下文提供生产量 $x_p$。

## 2. 概念 / 实体

### 1. ProductDemand

产品 $p$ 的需求量为 $d_p$，单位消耗为 $a_{mp}$。

### 2. Material

原材料 $m$ 的可用量为 $b_m$，单位与消耗系数一致。

## 3. 变量

### 1. 决策变量

Material 上下文不重复注册生产变量；它消费 Produce 上下文的 $x_p\in\mathbb Z_{\ge0}$。

### 2. 辅助变量

**$u_m$**：材料 $m$ 的用量，物料单位，$u_m\ge0$，$\forall m\in M$。

## 4. 谓词

**ActiveProduct**：产品在当前 RMP 中存在有效生产列。

**MaterialDemanded**：产品的需求量大于零。

## 5. 集合

**$P$**：产品全集；**$P^{A}$**：当前活跃产品子集。

**$M$**：原材料全集；**$P_m$**：消耗材料 $m$ 的产品集合。

## 6. 中间值

### 1. 材料用量

**说明**：材料 $m$ 的总消耗由所有活跃产品的生产量和单位消耗系数计算。

$$
u_m=\sum_{p\in P_m}a_{mp}x_p,\qquad \forall m\in M.
$$

### 2. 剩余库存

**说明**：剩余库存是可用量减去当前用量。

$$
remaining_m=b_m-u_m,\qquad \forall m\in M.
$$

## 7. 断言

### 1. 非负库存

**说明**：输入材料库存不能为负。

$$
\forall m\in M\;(b_m\ge0).
$$

### 2. 系数一致

**说明**：产品与材料的消耗系数必须使用同一单位体系。

$$
\forall(m,p)\in M\times P\;(a_{mp}\ge0).
$$

## 8. 约束

### 1. 材料可用量 [Material Availability]

**说明**：所有活跃生产量产生的材料用量不得超过库存。

$$
s.t.\quad u_m\le b_m,\qquad \forall m\in M.
$$

### 2. 需求覆盖 [Demand Coverage]

**说明**：生产上下文负责需求覆盖；Material 上下文只提供系数和库存边界。

$$
s.t.\quad \sum_{p\in P^{A}}x_p\ge d_p,\qquad \forall p\in P^{A}.
$$

## 9. 目标函数

本上下文不单独注册目标；材料成本由 Produce 上下文组合到 RMP 目标中。

## 10. 算法引用

| 算法 | 路径 | 用途 |
|---|---|---|
| Column generation | `csp1d-domain-produce-context` | 生成生产列并更新材料系数 |

## 11. 统一语言

| 术语 | 符号 | 定义 |
|---|---|---|
| 产品 | $p$ | 可生产的需求品 |
| 原材料 | $m$ | 受库存限制的资源 |
| 材料用量 | $u_m$ | 产品生产造成的消耗 |

## 12. 设计决策

| 决策 | 依据 |
|---|---|
| 不复制 $x_p$ | 生产变量由 Produce/RMP 统一注册 |

## 13. 变更记录

| 版本 | 变更 | 原因 |
|---|---|---|
| 1.0 | 按 Material 上下文补齐模型 | 与当前实现一致 |

## 源码与验证

[CSP1D Material 上下文](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-framework-csp1d/csp1d-domain-material-context)
