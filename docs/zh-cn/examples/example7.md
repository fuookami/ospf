# 示例 7：运输问题

## 问题描述

有一些仓库与一些商店，每个仓库有货物量，每个商店有货物需求量，从每个仓库往每个商店每运输 $1\,kg$ 货物有成本。

|        |  仓库 A   |  仓库 B   |  仓库 C   |
| :----: | :-------: | :-------: | :-------: |
| 货物量 | $510\,kg$ | $470\,kg$ | $520\,kg$ |

|            |  商店 A   |  商店 B   |  商店 C   |  商店 D   |
| :--------: | :-------: | :-------: | :-------: | :-------: |
| 货物需求量 | $200\,kg$ | $400\,kg$ | $600\,kg$ | $300\,kg$ |

|        | 商店 A | 商店 B | 商店 C | 商店 D |
| :----: | :----: | :----: | :----: | :----: |
| 仓库 A |  $12$  |  $13$  |  $21$  |  $7$   |
| 仓库 B |  $14$  |  $17$  |  $8$   |  $18$  |
| 仓库 C |  $10$  |  $11$  |  $9$   |  $15$  |

给出每个仓库往每个商店的货物运输量，令总成本最小。

## 数学模型

### 变量

$x_{ws}$ ：仓库 $w$ 往商店 $s$ 的货物运输量。

### 中间值

#### 1. 总成本

$$
Cost = \sum_{w \in W}\sum_{s \in S} Cost_{ws} \cdot x_{ws}
$$

#### 2. 仓库的出货量

$$
Shipment_{w} = \sum_{s \in S} x_{ws}, \; \forall w \in W
$$

#### 3. 商店的进货量

$$
Purchase_{s} = \sum_{w \in W} x_{ws}, \; \forall s \in S
$$

### 目标函数

#### 1. 总成本最小

$$
min \quad Cost
$$

### 约束

#### 1. 仓库的出货量不能超过货物量

$$
s.t. \quad Shipment_{w} \leq Storage_{w}, \; \forall w \in W
$$

#### 2. 商店的进货量要大于需求量

$$
s.t. \quad Purchase_{s} \geq Demand_{s}, \; \forall s \in S
$$

## 期望结果

|        |  商店 A   |  商店 B   |  商店 C   |  商店 D   |
| :----: | :-------: | :-------: | :-------: | :-------: |
| 仓库 A | $200\,kg$ | $10\,kg$  |  $0\,kg$  | $300\,kg$ |
| 仓库 B |  $0\,kg$  |  $0\,kg$  | $470\,kg$ |  $0\,kg$  |
| 仓库 C |  $0\,kg$  | $390\,kg$ | $130\,kg$ |  $0\,kg$  |

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

data class Store(
    val demand: Flt64
) : AutoIndexed(Store::class)

data class Warehouse(
    val stowage: Flt64,
    val cost: Map<Store, Flt64>
) : AutoIndexed(Warehouse::class)

val stores: List<Store> = ... // 商店列表
val warehouses: List<Warehouse> = ... // 仓库列表

// 创建模型实例
val metaModel = LinearMetaModel("demo7")

// 定义变量
val x = UIntVariable2("x", Shape2(warehouses.size, stores.size))
for (w in warehouses) {
    for (s in stores) {
        x[w, s].name = "${x.name}_(${w.index},${s.index})"
    }
}
metaModel.add(x)

// 定义中间值
val cost = LinearExpressionSymbol(sum(warehouses.map { w ->
    sum(stores.filter { w.cost.contains(it) }.map { s ->
        w.cost[s]!! * x[w, s]
    })
}), "cost")
metaModel.add(cost)

val shipment = LinearIntermediateSymbols1(
    "shipment",
    Shape1(warehouses.size)
) { i, _ ->
    val w = warehouses[i]
    LinearExpressionSymbol(
        sum(stores.filter { w.cost.contains(it) }.map { s -> x[w, s] }),
        "shipment_${w.index}"
    )
}
metaModel.add(shipment)

val purchase = LinearIntermediateSymbols1(
    "purchase",
    Shape1(stores.size)
) { i, _ ->
    val s = stores[i]
    LinearExpressionSymbol(
        sum(warehouses.filter { w -> w.cost.contains(s) }.map { w -> x[w, s] }),
        "purchase_${s.index}"
    )
}
metaModel.add(purchase)

// 定义目标函数
metaModel.minimize(cost, "cost")

// 定义约束
for(w in warehouses){
    metaModel.addConstraint(
        shipment[w] leq w.stowage,
        "stowage_${w.index}"
    )
}

for(s in stores){
    metaModel.addConstraint(
        purchase[s] geq s.demand,
        "demand_${s.index}"
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
val solution = stores.associateWith { warehouses.associateWith { Flt64.zero }.toMutableMap() }
for (token in metaModel.tokens.tokens) {
    if (token.result!! geq Flt64.one
        && token.variable.belongsTo(x)
    ) {
        val warehouse = warehouses[token.variable.vectorView[0]]
        val store = stores[token.variable.vectorView[1]]
        solution[store]!![warehouse] = solution[store]!![warehouse]!! + token.result!!
    }
}
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo7.kt)
