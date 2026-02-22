# Example 9: Facility Location Problem

## Problem Description

In a city divided into regular blocks along east-west and north-south directions, residential points are scattered across different blocks. Use $x$ coordinate to represent east-west direction and $y$ coordinate to represent north-south direction. The position of each residential point can be represented by coordinates $(x, \, y)$.

Select a location for building a post office to minimize the total distance from residential points to the post office. Distance is measured using Manhattan distance.

|       | Residential Point A | Residential Point B | Residential Point C | Residential Point D | Residential Point E | Residential Point F |
| :---: | :-----------------: | :-----------------: | :-----------------: | :-----------------: | :-----------------: | :-----------------: |
|  $x$  |      $9\,km$        |      $2\,km$        |      $3\,km$        |      $3\,km$        |      $5\,km$        |      $4\,km$        |
|  $y$  |      $2\,km$        |      $1\,km$        |      $8\,km$        |     $-2\,km$        |      $9\,km$        |     $-2\,km$        |

## Mathematical Model

### Variables

$x, \, y$: Post office coordinates.

### Intermediate Values

#### 1. East-West Distance

$$
dx_{s} = \text{Slack}(x, \, x_{s}) = |x - x_{s}|, \; \forall s \in S
$$

#### 2. North-South Distance

$$
dy_{s} = \text{Slack}(y, \, y_{s}) = |y - y_{s}|, \; \forall s \in S
$$

#### 3. Distance

$$
\text{Distance}_{s} = dx_{s} + dy_{s}, \; \forall s \in S
$$

### Objective Function

#### 1. Minimize Sum of Distances

$$
\min \quad \sum_{s \in S} \text{Distance}_{s}
$$

## Expected Results

The post office should be located at $(3, \, 1)$.

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

data class Settlement(
    val x: Flt64,
    val y: Flt64
) : AutoIndexed(Settlement::class)

val settlements: List<Settlement> = ... // Residential point list

// Create model instance
val metaModel = LinearMetaModel("demo9")

// Define variables
val x = IntVar("x")
val y = IntVar("y")
metaModel.add(x)
metaModel.add(y)

// Define intermediate values
val dx = LinearIntermediateSymbols1("dx", Shape1(settlements.size)) { i, _ ->
    SlackFunction(
        type = UInteger,
        x = LinearPolynomial(x),
        y = LinearPolynomial(settlements[i].x),
        name = "dx_$i"
    )
}
metaModel.add(dx)

val dy = LinearIntermediateSymbols1("dy", Shape1(settlements.size)) { i, _ ->
    SlackFunction(
        type = UInteger,
        x = LinearPolynomial(y),
        y = LinearPolynomial(settlements[i].y),
        name = "dy_$i"
    )
}
metaModel.add(dy)

val distance = LinearIntermediateSymbols1("distance", Shape1(settlements.size)) { i, _ ->
    LinearExpressionSymbol(
        dx[i] + dy[i],
        name = "distance_$i"
    )
}
metaModel.add(distance)

// Define objective function
metaModel.minimize(sum(distance[_a]))

// Call solver to solve
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// Parse results
val position = ArrayList<Flt64>()
for (token in metaModel.tokens.tokens) {
    if (token.variable.belongsTo(x)) {
        position.add(token.result!!)
    }
}
for (token in metaModel.tokens.tokens) {
    if (token.variable.belongsTo(y)) {
        position.add(token.result!!)
    }
}
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo9.kt)
