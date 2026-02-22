# Example 5: Knapsack Problem

## Problem Description

There are several items, each with corresponding value and weight.

|       | Item A  | Item B  | Item C  | Item D  | Item E  |
| :---: | :-----: | :-----: | :-----: | :-----: | :-----: |
| Weight | $2\,kg$ | $2\,kg$ | $6\,kg$ | $5\,kg$ | $4\,kg$ |
| Value |   $6$   |   $3$   |   $5$   |   $4$   |   $6$   |

Objective: Select a subset of these items to maximize total value, while satisfying the following condition:

1. The total weight of selected items does not exceed $10\,kg$.

## Mathematical Model

### Variables

$x_{c}$: Indicates whether item $c$ is selected.

### Intermediate Values

#### 1. Total Value

$$
\text{Value} = \sum_{c \in C} \text{Value}_{c} \cdot x_{c}
$$

#### 2. Total Weight

$$
\text{Weight} = \sum_{c \in C} \text{Weight}_{c} \cdot x_{c}
$$

### Objective Function

#### 1. Maximize Total Value

$$
\max \quad \text{Value}
$$

### Constraints

#### 1. Total Weight Does Not Exceed Maximum Weight

$$
\text{s.t.} \quad \text{Weight} \leq \text{Weight}^{\text{Max}}
$$

## Expected Results

Select items A, B, and E.

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

val cargos: List<Cargo> = ... // Item list
val maxWeight = UInt64(10U)

// Create model instance
val metaModel = LinearMetaModel("demo5")

// Define variables
val x = BinVariable1("x", Shape1(cargos.size))
for (c in cargos) {
    x[c].name = "${x.name}_${c.index}"
}
metaModel.add(x)

// Define intermediate values
val cargoValue = LinearExpressionSymbol(sum(cargos) { c -> c.value * x[c] }, "value")
metaModel.add(cargoValue)

val cargoWeight = LinearExpressionSymbol(sum(cargos) { c -> c.weight * x[c] }, "weight")
metaModel.add(cargoWeight)

// Define objective function
metaModel.maximize(cargoValue, "value")

// Define constraints
metaModel.addConstraint(
    cargoWeight leq maxWeight,
    "weight"
)

// Call solver to solve
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// Parse results
val solution = HashSet<Cargo>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! eq Flt64.one && token.variable.belongsTo(x)) {
        solution.add(cargos[token.variable.vectorView[0]])
    }
}
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo5.kt)
