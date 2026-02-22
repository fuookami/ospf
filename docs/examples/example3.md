# Example 3: Blending Problem

## Problem Description

There are several products and several raw materials. Each product has a given demand, each raw material has a corresponding cost, and each raw material can be used to produce multiple products.

|        | Product A  | Product B  | Product C  |
| :----: | :--------: | :--------: | :--------: |
| Demand | $15000$ | $15000$ | $10000$ |

|       | Material A | Material B | Material C | Material D |
| :---: | :--------: | :--------: | :--------: | :--------: |
| Cost  | $115$  |  $97$  |  $82$  |  $76$  |

|        | Material A | Material B | Material C | Material D |
| :----: | :--------: | :--------: | :--------: | :--------: |
| Product A |  $30$  |  $15$  |  $-$   |  $15$  |
| Product B |  $10$  |  $-$   |  $25$  |  $15$  |
| Product C |  $-$   |  $20$  |  $15$  |  $15$  |

Objective: Determine the usage amount of each raw material to minimize the total cost.

## Mathematical Model

### Variables

$x_{m}$: indicates the usage amount of raw material $m$.

### Intermediate Values

#### 1. Total Cost

$$
\text{Cost} = \sum_{m \in M} \text{Cost}_{m} \cdot x_{m}
$$

#### 2. Product Yield

$$
\text{Yield}^{\text{Product}}_{p} = \sum_{m \in M} \text{Yield}_{mp} \cdot x_{m}, \; \forall p \in P
$$

### Objective Function

#### 1. Minimize Total Cost

$$
\min \quad \text{Cost}
$$

### Constraints

#### 1. Each Product Yield Must Meet Demand Without Waste

$$
\text{s.t.} \quad \text{Yield}^{\text{Product}}_{p} = \text{Yield}^{\text{Product, Min}}_{p}, \; \forall p \in P
$$

## Expected Results

Material A: $284$ units, Material B: $8$ units, Material C: $232$ units, Material D: $424$ units.

## Code Implementation

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

val products: List<Product> = ...  // Product list
val materials: List<Material> = ... // Material list

// Create model instance
val metaModel = LinearMetaModel("demo3")

// Define variables
val x = UIntVariable1("x", Shape1(materials.size))
for (c in materials) {
    x[c].name = "${x.name}_${c.index}"
}
metaModel.add(x)

// Define intermediate values
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

// Define objective function
metaModel.minimize(cost)

// Define constraints
for (p in products) {
    metaModel.addConstraint(yield[p.index] eq p.minYield)
}

// Invoke solver
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// Parse results
val solution = HashMap<Material, UInt64>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! eq Flt64.one && token.variable.belongsTo(x)) {
        val material = materials[token.variable.vectorView[0]]
        solution[material] = token.result!!.round().toUInt64()
    }
}
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo3.kt)
