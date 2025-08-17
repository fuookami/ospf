# 示例 8：生产问题

## 问题描述

有一些设备与一些产品，每个产品有收益，每个设备有数量，生产每个产品会消耗每个设备一定工时。

|       | 产品 A | 产品 B | 产品 C | 产品 D | 产品 E |
| :---: | :----: | :----: | :----: | :----: | :----: |
| 价值  | $123$  |  $94$  | $105$  | $132$  | $118$  |

|       | 设备 A | 设备 B | 设备 C | 设备 D |
| :---: | :----: | :----: | :----: | :----: |
| 数量  |  $12$  |  $14$  |  $8$   |  $6$   |

|        |  产品 A   |  产品 B   |  产品 C   |  产品 D   |  产品 E   |
| :----: | :-------: | :-------: | :-------: | :-------: | :-------: |
| 设备 A | $0.23\,h$ | $0.44\,h$ | $0.17\,h$ | $0.08\,h$ | $0.36\,h$ |
| 设备 B | $0.13\,h$ |    $-$    | $0.20\,h$ | $0.37\,h$ | $0.19\,h$ |
| 设备 C |    $-$    | $0.25\,h$ | $0.34\,h$ |    $-$    | $0.18\,h$ |
| 设备 D | $0.55\,h$ | $0.72\,h$ |    $-$    | $0.61\,h$ |    $-$    |

给出每个产品的产量，令总收益最大，并满足以下条件：

1. 每个设备的工时小于 $2000\,h$。

## 数学模型

### 变量

$x_{p}$ ：产品 $p$ 的产量。

### 中间值

#### 1. 总收益

$$
Profit = \sum_{p \in P} Profit_{p} \cdot x_{p}
$$

#### 2. 设备总工时

$$
ManHours_{e} = \sum_{p \in P} Cost_{ep} \cdot x_{p}, \; \forall e \in E
$$

### 目标函数

#### 1. 总收益最大

$$
max \quad Profit
$$

### 约束

#### 1. 设备工时不能超过最大值

$$
s.t. \quad ManHours_{e} \leq Amount_{e} \cdot ManHours^{Max}, \; \forall e \in E
$$

## 期望结果

产品 A 生产 $0$ 个，产品 B 生产 $0$ 个，产品 C 选择 $18771$ 个，产品 D 生产 $19672$ 个，产品 E 生产 $53431$ 个。

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

data class Product(
    val profit: Flt64
) : AutoIndexed(Product::class)

data class Equipment(
    val amount: UInt64,
    val manHours: Map<Product, Flt64>
) : AutoIndexed(Equipment::class)

private val maxManHours = Flt64(2000)
private val products: List<Product> = ... // 产品列表
private val equipments: List<Equipment> = ... // 设备列表

// 创建模型实例
val metaModel = LinearMetaModel("demo8")

// 定义变量
val x = UIntVariable1("x", Shape1(products.size))
for (p in products) {
    x[p].name = "${x.name}_${p.index}"
}
metaModel.add(x)

// 定义中间值
val profit = LinearExpressionSymbol(sum(products.map { p ->
    p.profit * x[p]
}), "profit")
metaModel.add(profit)

val manHours = LinearIntermediateSymbols1(
    "man_hours",
    Shape1(equipments.size)
) { i, _ ->
    val e = equipments[i]
    LinearExpressionSymbol(
        sum(products.mapNotNull { p -> e.manHours[p]?.let { it * x[p] } }),
        "man_hours_${e.index}"
    )
}
metaModel.add(manHours)

// 定义目标函数
metaModel.maximize(profit, "profit")

// 定义约束
for (e in equipments) {
    metaModel.addConstraint(
        manHours[e] leq e.amount.toFlt64() * maxManHours,
        "eq_man_hours_${e.index}"
    )
}

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
    if (token.result!! neq Flt64.one && token.variable.belongsTo(x)) {
        solution[products[token.variable.vectorView[0]]] = token.result!!.round().toUInt64()
    }
}
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo8.kt)
