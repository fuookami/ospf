# Example 6: Knapsack Problem with Multiple Items

## Problem Description

There are several items, each with corresponding value, weight, and quantity.

|       | Item A  | Item B  | Item C  |
| :---: | :-----: | :-----: | :-----: |
| Weight | $1\,kg$ | $2\,kg$ | $2\,kg$ |
| Value |   $6$   |  $10$   |  $20$   |
| Quantity |  $10$   |   $5$   |   $2$   |

Select a subset of these items to maximize total value, while satisfying the following condition:

1. The total weight of these items does not exceed $8\,kg$.

## Mathematical Model

### Variables

$x_{c}$: Quantity of item $c$.

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

#### 2. Quantity of Each Item Cannot Exceed Maximum Quantity

$$
\text{s.t.} \quad x_{c} \leq \text{Amount}^{\text{Max}}_{c}, \; \forall c \in C
$$

## Expected Results

Select $4$ of item A, $0$ of item B, and $2$ of item C.

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
    val value: UInt64,
    val amount: UInt64
) : AutoIndexed(Cargo::class)

private val cargos: List<Cargo> = ... // Item list
private val maxWeight = UInt64(8)

// Create model instance
val metaModel = LinearMetaModel("demo6")

// Define variables
val x = UIntVariable1("x", Shape1(cargos.size))
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
for(c in cargos){
    x[c].range.ls(c.amount)
}

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
val solution = HashMap<Cargo, UInt64>()
for (token in metaModel.tokens.tokens) {
    if (token.result!! geq Flt64.one && token.variable.belongsTo(x)) {
        solution[cargos[token.variable.vectorView[0]]] = token.result!!.round().toUInt64()
    }
}
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo6.kt)
