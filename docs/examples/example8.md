# Example 8: Production Problem

## Problem Description

There are several pieces of equipment and several products. Each product has a corresponding profit, each piece of equipment has a corresponding quantity, and producing each product consumes a certain amount of man-hours from each piece of equipment.

|       | Product A | Product B | Product C | Product D | Product E |
| :---: | :-------: | :-------: | :-------: | :-------: | :-------: |
| Value |   $123$   |   $94$    |   $105$   |   $132$   |   $118$   |

|       | Equipment A | Equipment B | Equipment C | Equipment D |
| :---: | :---------: | :---------: | :---------: | :---------: |
| Quantity |    $12$    |    $14$    |    $8$     |    $6$     |

|        | Product A | Product B | Product C | Product D | Product E |
| :----: | :-------: | :-------: | :-------: | :-------: | :-------: |
| Equipment A | $0.23\,h$ | $0.44\,h$ | $0.17\,h$ | $0.08\,h$ | $0.36\,h$ |
| Equipment B | $0.13\,h$ |    $-$    | $0.20\,h$ | $0.37\,h$ | $0.19\,h$ |
| Equipment C |    $-$    | $0.25\,h$ | $0.34\,h$ |    $-$    | $0.18\,h$ |
| Equipment D | $0.55\,h$ | $0.72\,h$ |    $-$    | $0.61\,h$ |    $-$    |

Determine the production quantity of each product to maximize total profit, while satisfying the following condition:

1. The man-hours for each piece of equipment are less than $2000\,h$.

## Mathematical Model

### Variables

$x_{p}$: Production quantity of product $p$.

### Intermediate Values

#### 1. Total Profit

$$
\text{Profit} = \sum_{p \in P} \text{Profit}_{p} \cdot x_{p}
$$

#### 2. Total Man-Hours per Equipment

$$
\text{ManHours}_{e} = \sum_{p \in P} \text{Cost}_{ep} \cdot x_{p}, \; \forall e \in E
$$

### Objective Function

#### 1. Maximize Total Profit

$$
\max \quad \text{Profit}
$$

### Constraints

#### 1. Equipment Man-Hours Cannot Exceed Maximum

$$
\text{s.t.} \quad \text{ManHours}_{e} \leq \text{Amount}_{e} \cdot \text{ManHours}^{\text{Max}}, \; \forall e \in E
$$

## Expected Results

Produce $0$ of product A, $0$ of product B, $18771$ of product C, $19672$ of product D, and $53431$ of product E.

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
    val profit: Flt64
) : AutoIndexed(Product::class)

data class Equipment(
    val amount: UInt64,
    val manHours: Map<Product, Flt64>
) : AutoIndexed(Equipment::class)

private val maxManHours = Flt64(2000)
private val products: List<Product> = ... // Product list
private val equipments: List<Equipment> = ... // Equipment list

// Create model instance
val metaModel = LinearMetaModel("demo8")

// Define variables
val x = UIntVariable1("x", Shape1(products.size))
for (p in products) {
    x[p].name = "${x.name}_${p.index}"
}
metaModel.add(x)

// Define intermediate values
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

// Define objective function
metaModel.maximize(profit, "profit")

// Define constraints
for (e in equipments) {
    metaModel.addConstraint(
        manHours[e] leq e.amount.toFlt64() * maxManHours,
        "eq_man_hours_${e.index}"
    )
}

// Call solver to solve
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// Parse results
val solution = HashMap<Product, UInt64>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! neq Flt64.one && token.variable.belongsTo(x)) {
        solution[products[token.variable.vectorView[0]]] = token.result!!.round().toUInt64()
    }
}
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo8.kt)
