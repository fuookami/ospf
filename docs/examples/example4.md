# Example 4: Ingredient Problem

## Problem Description

There are some products and some raw materials, each raw material has a given available quantity, each product has a revenue, each product has a maximum production quantity, and each product requires multiple types of raw materials.

|                    | Raw Material A | Raw Material B |
| :----------------: | :------------: | :------------: |
| Availible Quantity |      $24$      |      $8$       |

|        | Product A | Product B |
| :----: | :-------: | :-------: |
| Profit |    $5$    |    $4$    |

|           | Product A | ProductB |
| :-------: | :-------: | :------: |
| Max Yield |    $3$    |   $2$    |

|           | Raw Material A | Raw Material B |
| :-------: | :------------: | :------------: |
| Product A |      $6$       |      $1$       |
| Product B |      $4$       |      $2$       |

Determine the production quantity for each product to maximize total revenue, while satisfying the following conditions:

1. The difference in production quantities between any two products must not exceed one unit.

## Mathematical Model

### Variables

$x_{p}$: yield of product $p$。

### Intermediate Expressions

#### 1. Total Profit

$$
Profit = \sum_{p \in P} Profit_{p} \cdot x_{p}
$$

#### 2. Use of Materials

$$
Use_{m} = \sum_{p \in P} Use_{pm} \cdot x_{p}, \; \forall m \in M
$$

### Objective Function

#### 1. Maximize Total Profit

$$
max \quad Profit
$$

### Constraints

#### 1. Yield Limit

$$
s.t. \quad x_{p} \leq Yield^{Max}_{p}, \; \forall p \in P
$$

#### 2. Use Limit

$$
s.t. \quad Use_{m} \leq Available_{m}, \; \forall m \in M
$$

#### 3. Yield Difference Limit

$$
s.t. \quad x_{p} - x_{p^{\prime}} \leq Diff^{Max}, \; \forall (p, \, p^{\prime}) \in (P^{2} - \Delta P)
$$

## Expected Result

Product A yields $\frac{8}{3}$ units, product B yieldp $\frac{5}{3}$ units.

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

data class Material(
    val available: Flt64
) : AutoIndexed(Material::class)

data class Product(
     val profit: Flt64,
    val maxYield: Flt64,
    val use: Map<Material, Flt64>
) : AutoIndexed(Product::class)

val materials: List<Material> = ... // material data
val products: List<Product> = ...  // product data
val maxDiff = Int64(1)

// create a model instance
val metaModel = LinearMetaModel("demo4")

// define variables
val x = RealVariable1("x", Shape1(products.size))
for (p in products) {
    x[p].name = "${x.name}_${p.index}"
}
metaModel.add(x)

// define intermediate expressions
val profit = LinearExpressionSymbol(sum(products) { 
    p -> p.profit * x[p] 
}, "profit")
metaModel.add(profit)

val use = LinearIntermediateSymbols1("use", Shape1(materials.size)) { m, _ ->
    val material = materials[m]
    val ps = products.filter { it.use.contains(material) }
    LinearExpressionSymbol(
        sum(ps) { p -> p.use[material]!! * x[p] },
        "use_${m}"
    )
}
metaModel.add(use)

// define objective function
metaModel.maximize(profit, "profit")

// define constraints
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

// solve the model
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// parse results
val solution = HashMap<Material, Flt64>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! eq Flt64.one && token.variable.belongsTo(x)) {
        solution[materials[token.variable.vectorView[0]]] = token.result!!
    }
}

```

:::

For the complete implementation, please refer to:

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo4.kt)
