# 示例 4：配料问题

## 问题描述

有一些产品和一些原料，每个原料有给定的可用量，每个产品有收益，每个产品有最大生产量，每个产品需要多个多种原料。

|        | 原料A | 原料B |
| :----: | :---: | :---: |
| 可用量 | $24$  |  $8$  |

|       | 产品A | 产品B |
| :---: | :---: | :---: |
| 收益  |  $5$  |  $4$  |

|            | 产品A | 产品B |
| :--------: | :---: | :---: |
| 最大生产量 |  $3$  |  $2$  |

|       | 原料A | 原料B |
| :---: | :---: | :---: |
| 产品A |  $6$  |  $1$  |
| 产品B |  $4$  |  $2$  |

给出每个产品的生产量，令总收益最大，且满足以下条件：

1. 每个产品之间的生产量之差不能大于一个单位。

## 数学模型

### 变量

$x_{p}$：产品 $p$ 的生产量。

### 中间值

#### 1. 总收益

$$
Profit = \sum_{p \in P} Profit_{p} \cdot x_{p}
$$

#### 2. 原料使用量

$$
Use_{m} = \sum_{p \in P} Use_{pm} \cdot x_{p}, \; \forall m \in M
$$

### 目标函数

#### 1. 总收益最大

$$
max \quad Profit
$$

### 约束

#### 1. 各个产品的生产量不能超过最大生产量

$$
s.t. \quad x_{p} \leq Yield^{Max}_{p}, \; \forall p \in P
$$

#### 2. 每个原料的使用量不能超过可用量

$$
s.t. \quad Use_{m} \leq Available_{m}, \; \forall m \in M
$$

#### 3. 每个产品之间的生产量之差不能大于一个单位

$$
s.t. \quad x_{p} - x_{p^{\prime}} \leq Diff^{Max}, \; \forall (p, \, p^{\prime}) \in (P^{2} - \Delta P)
$$

## 期望结果

产品 A 生产 $\frac{8}{3}$ 个单位，产品 B 生产 $\frac{5}{3}$ 个单位。

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

data class Material(
    val available: Flt64
) : AutoIndexed(Material::class)

data class Product(
     val profit: Flt64,
    val maxYield: Flt64,
    val use: Map<Material, Flt64>
) : AutoIndexed(Product::class)

val materials = ... // 原料数据
val products = ...  // 产品数据
val maxDiff = Int64(1)

// 创建模型实例
val metaModel = LinearMetaModel("demo4")

// 定义变量
val x = RealVariable1("x", Shape1(products.size))
for (p in products) {
    x[p].name = "${x.name}_${p.index}"
}
metaModel.add(x)

// 定义中间值
profit = LinearExpressionSymbol(sum(products) { 
    p -> p.profit * x[p] 
}, "profit")
metaModel.add(profit)

use = LinearIntermediateSymbols1("use", Shape1(materials.size)) { m, _ ->
    val material = materials[m]
    val ps = products.filter { it.use.contains(material) }
    LinearExpressionSymbol(
        sum(ps) { p -> p.use[material]!! * x[p] },
        "use_${m}"
    )
}
metaModel.add(use)

// 定义目标函数
metaModel.maximize(profit, "profit")

// 定义约束
for (p in products) {
    x[p].range.ls(p.maxYield)
}

for (m in materials) {
    metaModel.addConstraint(use[m] leq m.available)
}

for (p1 in products) {
    for (p2 in products) {
        if (p1.index == p2.index) {
            continue
        }
        metaModel.addConstraint((x[p1] - x[p2]) leq maxDiff)
    }
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
val solution = HashMap<Material, Flt64>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! eq Flt64.one
        && token.variable.belongsTo(x)
    ) {
        solution[materials[token.variable.vectorView[0]]] = token.result!!
    }
}

```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo4.kt)
