# 示例 16：生产库存问题

## 问题描述

某公司生产背包。产品需求通常出现在每年的 3-6 月，预估这四个月的需求量分别为 100、200、180、300。据估 3-6 月各月能生产 50、180、280、270 个背包。因各月生产能力差异，当月需求可通过如下三种方法满足：

1. 当月生产背包的生产费用为 40 美元每个；
2. 以前某个月多余的产品，每个背包每个月的储存费用为 0.5 美元;
3. 延期交货，每个背包每月的延期交货费用为 2 美元。

制订四个月的最优生产计划，最小化成本，要求:

1. 四个月需求都能得到满足；
2. 每个月生成量不超过估计生产能力。

## 数学模型

### 变量

$x_{ij}$：第 $i$ 个月供应第 $j$ 个月的背包量。

### 中间值

#### 1. 每月总生产量

$$
Produce_{i} = \sum_{j \in M}x_{ij}, \; \forall i \in M
$$

#### 2. 每个月背包供应量

$$
Supply_{j} = \sum_{i \in M} x_{ij}, \; \forall j \in M
$$

#### 3. 延迟交货成本

$$
Cost^d = C^{d} \cdot \sum_{i \in M} \sum_{j \in M, i < j} (i - j) \cdot x_{ji}
$$

#### 4. 背包储存成本

$$
Cost^{s} = C^{s} \cdot \sum_{i \in M} \sum_{j \in M, \; i < j} (j - i) \cdot x_{ij}
$$

#### 5. 背包生产成本

$$
Cost^{p} = \sum_{i \in M} Produce_{i}
$$

### 目标函数

#### 1. 最小化背包生产总成本

$$
min \quad Cost^{d} + Cost^{s} + Cost^{p}
$$

### 约束

#### 1. 需求都能得到满足

$$
s.t. \quad Supply_{i} = Demand_{i}, \; \forall i \in M
$$

#### 2. 每月生产量不超预期生产能力

$$
s.t. \quad Produce_{i} \leq Produce^{e}_{i}, \; \forall i \in M
$$

## 期望结果

## 代码实现

::: code-group

```kotlin
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*


```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo16.kt)
