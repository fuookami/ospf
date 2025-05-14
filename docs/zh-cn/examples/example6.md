# 示例 6：背包问题

## 问题描述

有一些货物，每个货物有价值、重量和数量。

|       | 货物 A  | 货物 B  | 货物 C  |
| :---: | :-----: | :-----: | :-----: |
| 重量  | $1\,kg$ | $2\,kg$ | $2\,kg$ |
| 价值  |   $6$   |  $10$   |  $20$   |
| 数量  |  $10$   |   $5$   |   $2$   |

从这些货物中选出部分货物，令总价值最大，并满足以下条件：

1. 这些货物的总重量不超过 $8\,kg$。

## 数学模型

### 变量

$x_{c}$ ：货物 $c$ 的个数。

### 中间值

#### 1. 总价值

$$
Value = \sum_{c \in C} Value_{c} \cdot x_{c}
$$

#### 2. 总重量

$$
Weight = \sum_{c \in C} Weight_{c} \cdot x_{c}
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

#### 2. 每个货物的数量不能超过最大数量

$$
s.t. \quad x_{c} \leq Amount^{Max}_{c}, \; \forall c \in C
$$

## 期望结果

货物 A 选择 $4$ 个，货物 B 选择 $0$ 个，货物 C 选择 $2$ 个。

### 代码实现

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
    val value: UInt64,
    val amount: UInt64
) : AutoIndexed(Cargo::class)

private val cargos = ... // 货物列表
private val maxWeight = UInt64(8)

// 创建模型实例
val metaModel = LinearMetaModel("demo6")

// 定义变量
val x = UIntVariable1("x", Shape1(cargos.size))
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
for(c in cargos){
    x[c].range.ls(c.amount)
}

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
val solution = HashMap<Cargo, UInt64>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! geq Flt64.one && token.variable.belongsTo(x)) {
        solution[cargos[token.variable.vectorView[0]]] = token.result!!.round().toUInt64()
    }
}
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo6.kt)
