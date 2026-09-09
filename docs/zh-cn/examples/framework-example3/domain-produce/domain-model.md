# Demo3 Produce 上下文领域模型

[toc]

## 1. 概述

Produce 上下文建立一维分切的受限主问题，注册生产量、材料容量、需求和废料目标。

### 1. 依赖上下文

1. Material 上下文提供材料库存与单位消耗。
2. Pricing/column generation 提供新生产列。

## 2. 概念 / 实体

### 1. Product

产品 $p$ 有需求 $d_p$、收益 $c_p$ 与材料消耗系数 $a_{mp}$。

### 2. Production plan

生产计划 $k$ 描述一根原料或一批机器容量上可执行的切割组合。

## 3. 变量

### 1. 决策变量

**$x_p$**：产品 $p$ 的整数生产量，产品单位，域为 $\mathbb Z_{\ge0}$，$\forall p\in P$。

**$\lambda_k$**：生产列 $k$ 的选择量，列单位，域为 $\mathbb Z_{\ge0}$，$\forall k\in K$。

### 2. 辅助变量

**$w_k$**：计划 $k$ 的废料量，长度单位，$w_k\ge0$，$\forall k\in K$。

## 4. 谓词

**ActivePlan**：计划已经加入当前受限主问题。

**FeasiblePlan**：计划满足原料长度、机器批量和非负废料规则。

## 5. 集合

**$P$**：产品集合；**$K$**：当前生产列集合；**$E$**：机器或资源集合。

**$K^{A}$**：活跃列集合；**$P_e$**：机器 $e$ 可生产的产品集合。

## 6. 中间值

### 1. 机器工时

**说明**：机器 $e$ 的工时由其生产列和产品工时系数得到。

$$
H_e=\sum_{p\in P_e}h_{ep}x_p,\qquad \forall e\in E.
$$

### 2. 计划容量

**说明**：计划容量是机器数量与单机最大工时的乘积。

$$
C_e=amount_e\,H^{max}_e,\qquad \forall e\in E.
$$

### 3. 废料

**说明**：每根原料的未使用长度为容量减去切割长度。

$$
w_k=L_k-\sum_{p\in P}width_{kp}x_p,\qquad \forall k\in K^{A}.
$$

## 7. 断言

### 1. 需求非负

$$
\forall p\in P\;(d_p\ge0).
$$

### 2. 计划可行

$$
\forall k\in K^{A}\;(w_k\ge0).
$$

## 8. 约束

### 1. 需求覆盖 [Demand Coverage]

**说明**：活跃列提供的产品总量必须满足需求。

$$
s.t.\quad \sum_{k\in K^{A}}a_{pk}\lambda_k\ge d_p,\qquad \forall p\in P.
$$

### 2. 机器容量 [Machine Capacity]

**说明**：每台机器的工时不能超过可用容量。

$$
s.t.\quad H_e\le C_e,\qquad \forall e\in E.
$$

### 3. 列非负 [Non-negative Plan]

**说明**：生产量和列选择量均不能为负。

$$
s.t.\quad x_p\ge0,\quad \lambda_k\ge0.
$$

## 9. 目标函数

**说明**：当前 RMP 最小化生产和废料成本。

$$
\min\;\sum_{k\in K^{A}}cost_k\lambda_k+\sum_{k\in K^{A}}w_k\lambda_k.
$$

## 10. 算法引用

| 算法 | 用途 |
|---|---|
| Column generation | 定价并把可行生产列加入 RMP |

## 11. 统一语言

| 术语 | 符号 | 定义 |
|---|---|---|
| 生产列 | $k$ | 一种切割组合 |
| 受限主问题 | RMP | 当前列集合上的优化模型 |

## 12. 设计决策

| 决策 | 依据 |
|---|---|
| 列变量与直接生产量并存 | 分别描述 RMP 列和底层业务量 |

## 13. 变更记录

| 版本 | 变更 | 原因 |
|---|---|---|
| 1.0 | 按 Produce 上下文补齐 13 节模型 | 与当前实现对齐 |

## 源码与验证

[CSP1D Produce 上下文](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-framework-csp1d/csp1d-domain-produce-context)
