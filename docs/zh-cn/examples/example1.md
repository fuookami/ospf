# 示例 1：指派问题

## 问题描述

现有若干企业，每家企业拥有各自的财产数量、债务数量及收益量。

|        | 财产数量 | 债务数量 | 收益量 |
| :----: | :------: | :------: | :----: |
| 企业 A |  $3.48$  |  $1.28$  | $5400$ |
| 企业 B |  $5.62$  |  $2.53$  | $2300$ |
| 企业 C |  $7.33$  |  $1.02$  | $4600$ |
| 企业 D |  $6.27$  |  $3.55$  | $3300$ |
| 企业 E |  $2.14$  |  $0.53$  | $980$  |

目标是从这些企业中选出部分企业，使得总收益量最大，同时满足以下条件：

1. 所选企业的总财产数量大于 $10$；
2. 所选企业的总债务数量小于 $5$。

## 数学模型

### 变量

$x_{c}$：表示是否选择企业 $c$。

### 中间值

#### 1. 总财产数量

$$
\text{Capital} = \sum_{c \in C} \text{Capital}_{c} \cdot x_{c}
$$

#### 2. 总债务量

$$
\text{Liability} = \sum_{c \in C} \text{Liability}_{c} \cdot x_{c}
$$

#### 3. 总收益量

$$
\text{Profit} = \sum_{c \in C} \text{Profit}_{c} \cdot x_{c}
$$

### 目标函数

#### 1. 总收益量最大

$$
\max \quad \text{Profit}
$$

### 约束

#### 1. 总财产数量需大于最小值

$$
\text{s.t.} \quad \text{Capital} \geq \text{Capital}^{\text{Min}}
$$

#### 2. 总债务数量需小于最大值

$$
\text{s.t.} \quad \text{Liability} \leq \text{Liability}^{\text{Max}}
$$

## 期望结果

选择企业 A、B、C。

## 代码实现

::: code-group

```kotlin
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.concept.*
import fuookami.ospf.kotlin.utils.functional.*
import fuookami.ospf.kotlin.utils.multi_array.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.monomial.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

data class Company(
    val capital: Flt64,
    val liability: Flt64,
    val profit: Flt64
) : AutoIndexed(Company::class)

val companies: List<Company> = ... // 企业列表
val minCapital = Flt64(10.0)
val maxLiability = Flt64(5.0)

// 创建模型实例
val metaModel = LinearMetaModel("demo1")

// 定义变量
val x = BinVariable1("x", Shape1(companies.size))
for (c in companies) {
    x[c].name = "${x.name}_${c.index}"
}
metaModel.add(x)

// 定义中间值
val capital = LinearExpressionSymbol(
    sum(companies) { it.capital * x[it] }, 
    "capital"
)
metaModel.add(capital)

val liability = LinearExpressionSymbol(
    sum(companies) { it.liability * x[it] }, 
    "liability"
)
metaModel.add(liability)

val profit = LinearExpressionSymbol(
    sum(companies) { it.profit * x[it] }, 
    "profit"
)
metaModel.add(profit)

// 定义目标函数
metaModel.maximize(profit)

// 定义约束
metaModel.addConstraint(capital geq minCapital)
metaModel.addConstraint(liability leq maxLiability)

// 调用求解器求解
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// 解析结果
val solution = ArrayList<Company>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! eq Flt64.one && token.variable.belongsTo(x)) {
        solution.add(companies[token.variable.index])
    }
}
```

:::

**完整实现参考：**

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo1.kt)
