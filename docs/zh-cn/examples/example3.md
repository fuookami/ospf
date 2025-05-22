# 示例 3：配料问题

## 问题描述

有一些产品和一些原料，每个产品有给定的需求量，每个原料有成本，每个原料可以生产多个多种产品。

|        | 产品 A  | 产品 B  | 产品 C  |
| :----: | :-----: | :-----: | :-----: |
| 需求量 | $15000$ | $15000$ | $10000$ |

|       | 原料 A | 原料 B | 原料 C | 原料 D |
| :---: | :----: | :----: | :----: | :----: |
| 成本  | $115$  |  $97$  |  $82$  |  $76$  |

|        | 原料 A | 原料 B | 原料 C | 原料 D |
| :----: | :----: | :----: | :----: | :----: |
| 产品 A |  $30$  |  $15$  |  $-$   |  $15$  |
| 产品 B |  $10$  |  $-$   |  $25$  |  $15$  |
| 产品 C |  $-$   |  $20$  |  $15$  |  $15$  |

给出每个原料的使用量，令总成本最小。

## 数学模型

### 变量

$x_{m}$ ：原料 $m$ 的使用量。

### 中间值

#### 1. 总成本

$$
Cost = \sum_{m \in M} Cost_{m} \cdot x_{m}
$$

#### 2. 产品产量

$$
Yield^{Product}_{p} = \sum_{m \in M} Yield_{mp} \cdot x_{m}, \; \forall p \in P
$$

### 目标函数

#### 1. 总成本最小

$$
min \quad Cost
$$

### 约束

#### 1. 各个产品的产量要满足需求，且不能浪费

$$
s.t. \quad Yield^{Product}_{p} = Yield^{Product, Min}_{p}, \; \forall p \in P
$$

### 期望结果

原料 A 使用 $284$ 个，原料 B 使用 $8$ 个，原料 C 使用 $232$ 个，原料 D 使用 $424$ 个。

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

data class Product(
    val minYield: Flt64
) : AutoIndexed(Product::class)

data class Material(
    val cost: Flt64,
    val yieldQuantity: Map<Product, Flt64>
) : AutoIndexed(Material::class)

val products: List<Product> = ...  // 产品列表
val materials: List<Material> = ... // 原料列表

// 创建模型实例
val metaModel = LinearMetaModel("demo3")

// 定义变量
val x = = UIntVariable1("x", Shape1(materials.size))
for (c in materials) {
    x[c].name = "${x.name}_${c.index}"
}
metaModel.add(x)

// 定义中间值
val cost = LinearExpressionSymbol(
    sum(materials) { it.cost * x[it] }, 
    "cost"
)
metaModel.add(cost)

val yield = LinearIntermediateSymbols1("yield", Shape1(products.size)) { p, _ ->
    val product = products[p]
    LinearExpressionSymbol(
        sum(materials.filter { it.yieldQuantity.contains(product) }) { m ->
            m.yieldQuantity[product]!! * x[m]
        },
        "yieldProduct_${p}"
    )
}
metaModel.add(yield)

// 定义目标函数
metaModel.minimize(cost)

// 定义约束
for (p in products) {
    metaModel.addConstraint(yield[p.index] eq p.minYield)
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
val solution = HashMap<Material, UInt64>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! eq Flt64.one
        && token.variable.belongsTo(x)
    ) {
        val material = materials[token.variable.vectorView[0]]
        solution[material] = token.result!!.round().toUInt64()
    }
}
```

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo3.kt)
