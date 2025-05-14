# Example 5: Knapsack Problem

## Problem Description

There are some goods, each with a value and a weight.

|        | Good A  | Good B  | Good C  | Good D  | Good E  |
| :----: | :-----: | :-----: | :-----: | :-----: | :-----: |
| Weight | $2\,kg$ | $2\,kg$ | $6\,kg$ | $5\,kg$ | $4\,kg$ |
| Value  |   $6$   |   $3$   |   $5$   |   $4$   |   $6$   |

Select a subset of these goods to maximize the total value while satisfying the following conditions:

1. The total weight of the selected goods must not exceed $10\,kg$.

## Mathematical Model

### Variables 

$x_{c}$：whether to select good $c$。

### Intermediate Expressions

#### 1. Total Value

$$
Value = \sum_{c \in C}Value_{c} \cdot x_{c}
$$

#### 2. Total Weight

$$
Weight = \sum_{c \in C}Weight_{c} \cdot x_{c}
$$

### Objective Function

#### 1. Maximize Value

$$
max \quad Value
$$

### Constraints

#### 1. Total Weight Limit

$$
s.t. \quad Weight \leq Weight^{Max}
$$

## Expected Result

Select goods A, B, E.

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

data class Cargo(
    val weight: UInt64,
    val value: UInt64
) : AutoIndexed(Cargo::class)

val cargos = ... // cargo data
val maxWeight = UInt64(10U)

// create a model instance
val metaModel = LinearMetaModel("demo5")

// define variables
val x = BinVariable1("x", Shape1(cargos.size))
for (c in cargos) {
    x[c].name = "${x.name}_${c.index}"
}
metaModel.add(x)

// define intermediate expressions
val cargoValue = LinearExpressionSymbol(sum(cargos) { c -> c.value * x[c] }, "value")
metaModel.add(cargoValue)

val cargoWeight = LinearExpressionSymbol(sum(cargos) { c -> c.weight * x[c] }, "weight")
metaModel.add(cargoWeight)

// define objective function
metaModel.maximize(cargoValue, "value")

// define constraints
metaModel.addConstraint(
    cargoWeight leq maxWeight,
    "weight"
)

// solve the model
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// parse results
val solution = HashSet<Cargo>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! eq Flt64.one
        && token.variable.belongsTo(x)
    ) {
        solution.add(cargos[token.variable.vectorView[0]])
    }
}
```

:::

For the complete implementation, please refer to:

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo5.kt)
