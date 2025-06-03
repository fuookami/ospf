# 示例 12：投资组合问题

## 问题描述

市场上有n种资产 $S_{i}$ （ $i=1, 2, ···, n$ ）可以选择，先用数额为 $M$ 的相当大的资金作为一个时期的投资。这 $n$ 种资产在这一时期内购买 $S_{i}$ 的平均收益率为 $r_{i}$ ，风险损失率为 $q_{i}$ ，投资越分散，总的风险越少，总体风险可用投资的 $S_{i}$ 的投资风险加权投资金额得到。

购买 $S_{i}$ 时要付交易费，费率 $p_{i}$ ，当购买额不超过给定值 $u_{i}$ 时，交易费按购买 $u_{i}$ 计算。另外，假定同时期银行存款利率是 $r_{0}$ ，既无交易费又无风险。（ $r_{0}$ = 5%）

| $S_{i}$ | $r_{i}$(%) | $q_{i}$(%) | $p_{i}$(%) | $u_{i}$(元) |
| :-----: | :--------: | :--------: | :--------: | :---------: |
| $S_{1}$ |     28     |    4.0     |     8      |     103     |
| $S_{2}$ |     21     |    1.5     |     2      |     198     |
| $S_{3}$ |     23     |    5.5     |    4.5     |     52      |
| $S_{4}$ |     25     |    2.6     |     4      |     40      |

设定一个投资组合方案，给定资金 $1000000$ 元，有选择的购买资产或者银行存款，在风险控制在 $2\%$ 以内的情况下，在使净收益尽可能大。

## 数学模型

### 变量

$x_{i}$：$i$ 种类投资品的购买资金。

### 中间值

#### 1. 是否有购置

$$
Assignment_{i} = If(x_{i} > 0), \; \forall i \in S
$$

#### 2. 手续费

$$
Premium_{i} = Max(p_{i} \cdot x_{i}, \, u_{i} \cdot Assignment_{i}), \; \forall i \in S
$$

#### 3. 收益

$$
Yield = \sum_{i \in S} (r_{i} \cdot x_{i} - Premium_{i})
$$

#### 4. 风险

$$
Risk = \frac{\sum_{i \in S} q_{i} \cdot x_{i}}{M}
$$

### 目标函数

#### 1. 收益最大

$$
max \quad Yield
$$

### 约束

#### 1. 资金和为 $M$

$$
s.t. \quad \sum_{i \in S}(1 + p_{i}) \cdot x_{i} = M
$$

#### 2. 风险限制

$$
s.t. \quad \sum_{i \in S} Risk_{i} \leq Risk^{Max}
$$

## 期望结果

$S_{4}$ 投资 $494505$ 元，$S_{2}$ 投资 $476191$ 元。

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

data class Product(
    val yield: Flt64,
    val risk: Flt64,
    val premium: Flt64,
    val minPremium: Flt64
) : AutoIndexed(Product::class)

val products: List<Product> = ... // 产品列表
val funds = Flt64(1000000.0)
val maxRisk = Flt64(0.02)

// 创建模型实例
val metaModel = LinearMetaModel("demo12")

// 定义变量
val x = UIntVariable1("x", Shape1(products.size))
metaModel.add(x)

// 定义中间值
val assignment = LinearIntermediateSymbols1(
    "assignment",
    Shape1(products.size)
) { i, _ ->
    BinaryzationFunction(
        x = LinearPolynomial(x[i]),
        name = "assignment_$i"
    )
}
metaModel.add(assignment)

val premium = LinearIntermediateSymbols1(
    "premium",
    Shape1(products.size)
) { i, _ ->
    val product = products[i]
    MaxFunction(
        listOf(
            LinearPolynomial(product.premium * x[i]),
            LinearPolynomial(product.minPremium * assignment[i])
        ),
        "premium_$i"
    )
}
metaModel.add(premium)

val risk = LinearExpressionSymbol(
    sum(products.map { p -> p.risk * x[p] / funds }),
    "risk"
)
metaModel.add(risk)

val yield = LinearExpressionSymbol(
    sum(products.map { p -> p.yield * x[p] - premium[p] }),
    "yield"
)
metaModel.add(yield)

// 定义目标函数
metaModel.maximize(yield, "yield")

// 定义约束
metaModel.addConstraint(
    sum(products.map { p -> x[p] + premium[p] }) eq funds,
    "funs"
)

metaModel.addConstraint(
    risk leq maxRisk,
    "risk"
)

// 调用求解器求解
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// 解析结果
val solution = HashMap<Product, UInt64>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! geq Flt64.one && token.variable.belongsTo(x)) {
        val vector = token.variable.vectorView
        val product = products[vector[0]]
        solution[product] = token.result!!.round().toUInt64()
    }
}
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo12.kt)
