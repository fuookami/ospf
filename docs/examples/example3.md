# Example 3: Ingredient Problem

## Problem Description

There are a number of products and raw materials. Each product has a specified production quantity, each raw material has a cost, and each raw material can be used to produce multiple different products.

|        | Product A | Product B | Product C |
| :----: | :-------: | :-------: | :-------: |
| Demand |  $15000$  |  $15000$  |  $10000$  |

|       | Material A | Material B | Material C | Material D |
| :---: | :--------: | :--------: | :--------: | :--------: |
| Cost  |   $115$    |    $97$    |    $82$    |    $76$    |

|           | Material A | Material B | Material C | Material D |
| :-------: | :--------: | :--------: | :--------: | :--------: |
| Product A |    $30$    |    $15$    |    $-$     |    $15$    |
| Product B |    $10$    |    $-$     |    $25$    |    $15$    |
| Product C |    $-$     |    $20$    |    $15$    |    $15$    |

Determine the amount of each raw material used to minimize the total cost.

## Mathematical Model

### Variables

$x_{m}$ : usage of raw material $m$.

### Intermediate Expressions

#### 1. Total Cost

$$
Cost = \sum_{m \in M} Cost_{m} \cdot x_{m}
$$

#### 2. Product Yield

$$
Yield^{Product}_{p} = \sum_{m \in M} Yield_{mp} \cdot x_{m}, \; \forall p \in P
$$

### Objective Function

#### 1. Minimize Total Cost

$$
min \quad Cost
$$

### Constraints

#### 1. Demand Limit

$$
s.t. \quad Yield^{Product}_{p} = Yield^{Product, Min}_{p}, \; \forall p \in P
$$

### Expected Result

Raw material A uses $284$ units, raw material B uses $8$ units, raw material C uses $232$ units, raw material D uses $424$ units.

### Code Implementation

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

val products: List<Product> = ...  // product data
val materials: List<Material> = ... // material data

// create a model instance
val metaModel = LinearMetaModel("demo3")

// define variables
val x = = UIntVariable1("x", Shape1(materials.size))
for (c in materials) {
    x[c].name = "${x.name}_${c.index}"
}
metaModel.add(x)

// define intermediate expressions
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

// define objective function
metaModel.minimize(cost)

// define constraints
for (p in products) {
    metaModel.addConstraint(yield[p.index] eq p.minYield)
}

// solve the model
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// parse results
val solution = HashMap<Material, UInt64>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! eq Flt64.one && token.variable.belongsTo(x)) {
        val material = materials[token.variable.vectorView[0]]
        solution[material] = token.result!!.round().toUInt64()
    }
}
```

For the complete implementation, please refer to:

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo3.kt)
