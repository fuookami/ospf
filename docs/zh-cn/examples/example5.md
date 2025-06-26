# 示例 5：背包问题

## 问题描述

有一些货物，每个货物有价值和重量。

|       | 货物 A  | 货物 B  | 货物 C  | 货物 D  | 货物 E  |
| :---: | :-----: | :-----: | :-----: | :-----: | :-----: |
| 重量  | $2\,kg$ | $2\,kg$ | $6\,kg$ | $5\,kg$ | $4\,kg$ |
| 价值  |   $6$   |   $3$   |   $5$   |   $4$   |   $6$   |

从这些货物中选出部分货物，令总价值最大，并满足以下条件：

1. 这些货物的总重量不超过 $10\,kg$。

## 数学模型

### 变量

$x_{c}$：是否选择货物 $c$。

### 中间值

#### 1. 总价值

$$
Value = \sum_{c \in C}Value_{c} \cdot x_{c}
$$

#### 2. 总重量

$$
Weight = \sum_{c \in C}Weight_{c} \cdot x_{c}
$$

### 目标函数

#### 1. 总价值最大

$$
max \quad Value
$$

### 约束

#### 1. 总重量不超过最大总重量

$$
s.t. \quad Weight \leq Weight^{Max}
$$

## 期望结果

选择货物 A、B、E。

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

data class Cargo(
    val weight: UInt64,
    val value: UInt64
) : AutoIndexed(Cargo::class)

val cargos: List<Cargo> = ... // 货物列表
val maxWeight = UInt64(10U)

// 创建模型实例
val metaModel = LinearMetaModel("demo5")

// 定义变量
val x = BinVariable1("x", Shape1(cargos.size))
for (c in cargos) {
    x[c].name = "${x.name}_${c.index}"
}
metaModel.add(x)

// 定义中间值
val cargoValue = LinearExpressionSymbol(sum(cargos) { c -> c.value * x[c] }, "value")
metaModel.add(cargoValue)

val cargoWeight = LinearExpressionSymbol(sum(cargos) { c -> c.weight * x[c] }, "weight")
metaModel.add(cargoWeight)

// 定义目标函数
metaModel.maximize(cargoValue, "value")

// 定义约束
metaModel.addConstraint(
    cargoWeight leq maxWeight,
    "weight"
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
val solution = HashSet<Cargo>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! eq Flt64.one && token.variable.belongsTo(x)) {
        solution.add(cargos[token.variable.vectorView[0]])
    }
}
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo5.kt)
