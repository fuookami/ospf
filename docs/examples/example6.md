# Example 6: Knapsack Problem

## Problem Description

There are some goods, each with a value, a weight and an amount.

|        | Good A  | Good B  | Good C  |
| :----: | :-----: | :-----: | :-----: |
| Weight  | $1\,kg$ | $2\,kg$ | $2\,kg$ |
| Value  |   $6$   |  $10$   |  $20$   |
| Amount  |  $10$   |   $5$   |   $2$   |

Select a portion of the goods from these items to maximize the total value while satisfying the following conditions:

1. The total weight of the selected goods must not exceed $8\,kg$.

## Mathematical Model

### Variables

$x_{c}$ ：amount of good $c$。

### Intermediate Expressions

#### 1. Total Value

$$
Value = \sum_{c \in C} Value_{c} \cdot x_{c}
$$

#### 2. Total Weight

$$
Weight = \sum_{c \in C} Weight_{c} \cdot x_{c}
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

#### 2. Total Amount Limit

$$
s.t. \quad x_{c} \leq Amount^{Max}_{c}, \; \forall c \in C
$$

## Expected Result

Select $4$ units of Good A, $0$ units of Good B, and $2$ units of Good C.

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

data class Cargo(
    val weight: UInt64,
    val value: UInt64,
    val amount: UInt64
) : AutoIndexed(Cargo::class)

private val cargos = ... // cargo data
private val maxWeight = UInt64(8)

// create a model instance
val metaModel = LinearMetaModel("demo6")

// define variables
val x = UIntVariable1("x", Shape1(cargos.size))
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
metaModel.maximize(cargoValue,"value")

// define constraints
for(c in cargos){
    x[c].range.ls(c.amount)
}

metaModel.addConstraint(
    cargoWeight leq maxWeight,"weight"
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
val solution = HashMap<Cargo, UInt64>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! geq Flt64.one && token.variable.belongsTo(x)) {
        solution[cargos[token.variable.vectorView[0]]] = token.result!!.round().toUInt64()
    }
}
```

:::

For the complete implementation, please refer to:

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo6.kt)
