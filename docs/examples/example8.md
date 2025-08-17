# Example 8: Production Problem

## Problem Description

Given a set of machines and products, each product has an associated profit, each machine has a finite capacity, and the production of each product requires a certain number of hours from each machine.

|        | Product A | Product B | Product C | Product D | Product E |
| :----: | :-------: | :-------: | :-------: | :-------: | :-------: |
| Profit |   $123$   |   $94$    |   $105$   |   $132$   |   $118$   |

|        | Equipment A | Equipment B | Equipment C | Equipment D |
| :----: | :---------: | :---------: | :---------: | :---------: |
| Amount |    $12$     |    $14$     |     $8$     |     $6$     |

|             | Product A | Product B | Product C | Product D | Product E |
| :---------: | :-------: | :-------: | :-------: | :-------: | :-------: |
| Equipment A | $0.23\,h$ | $0.44\,h$ | $0.17\,h$ | $0.08\,h$ | $0.36\,h$ |
| Equipment B | $0.13\,h$ |    $-$    | $0.20\,h$ | $0.37\,h$ | $0.19\,h$ |
| Equipment C |    $-$    | $0.25\,h$ | $0.34\,h$ |    $-$    | $0.18\,h$ |
| Equipment D | $0.55\,h$ | $0.72\,h$ |    $-$    | $0.61\,h$ |    $-$    |

Determine the production quantities of each product that maximize total revenue, while satisfying the following conditions:

1. Each machine's manhours must not exceed $2000\,h$.

## Mathematical Model

### Variables

$x_{p}$ : produce of product $p$.

### Intermediate Expressions

#### 1. Total Profit

$$
Profit = \sum_{p \in P} Profit_{p} \cdot x_{p}
$$

#### 2. Total ManHours

$$
ManHours_{e} = \sum_{p \in P} Cost_{ep} \cdot x_{p}, \; \forall e \in E
$$

### Objective Function

#### 1. Maximize Profit

$$
max \quad Profit
$$

### Constraints

#### 1. ManHours Limit

$$
s.t. \quad ManHours_{e} \leq Amount_{e} \cdot ManHours^{Max}, \; \forall e \in E
$$

## Expected Result

Produce $0$ units of Product A, $0$ units of Product B, $18771$ units of Product C, $19672$ units of Product D, and $53431$ units of Product E.

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
private val products: List<Product> = ... // product data
private val equipments: List<Equipment> = ... // equipment data

// create a model instance
val metaModel = LinearMetaModel("demo8")

// define variables
val x = UIntVariable1("x", Shape1(products.size))
for (p in products) {
    x[p].name = "${x.name}_${p.index}"
}
metaModel.add(x)

// define intermediate expressions
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

// define intermediate expressions
metaModel.maximize(profit, "profit")

// define constraints
for (e in equipments) {
    metaModel.addConstraint(
        manHours[e] leq e.amount.toFlt64() * maxManHours,
        "eq_man_hours_${e.index}"
    )
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
val solution = HashMap<Product, UInt64>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! neq Flt64.one && token.variable.belongsTo(x)) {
        solution[products[token.variable.vectorView[0]]] = token.result!!.round().toUInt64()
    }
}
```

:::

For the complete implementation, please refer to:

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo8.kt)
