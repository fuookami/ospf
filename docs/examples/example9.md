# Example 9: Location Selection Problem

In a city divided into regular blocks along the east-west and north-south directions, residential areas are scattered across different blocks. The $x$-coordinate represents the east-west direction, while the $y$-coordinate represents the north-south direction. The location of each residential area can be represented by the coordinates $(x, \, y)$.

Determine the post office location such that the total distance from the residential areas to the post office is minimized. The distance is measured using the Manhattan distance.

|       | Residential Area A | Residential Area B | Residential Area C | Residential Area D | Residential Area E | Residential F |
| :---: | :----------------: | :----------------: | :----------------: | :----------------: | :----------------: | :-----------: |
|  $x$  |      $9\,km$       |      $2\,km$       |      $3\,km$       |      $3\,km$       |      $5\,km$       |    $4\,km$    |
|  $y$  |      $2\,km$       |      $1\,km$       |      $8\,km$       |      $-2\,km$      |      $9\,km$       |   $-2\,km$    |

## Mathematical Model

### Variables

$x, \, y$ : Post office location.

### Intermediate Expressions

#### 1. East-west Distance

$$
dx_{s} = Slack(x, \, x_{s}) = |x - x_{s}|, \; \forall s \in S
$$

#### 2. North-south Distance

$$
dy_{s} = Slack(y, \, y_{s}) = |y - y_{s}|, \; \forall s \in S
$$

#### 3. Distance

$$
Distance_{s} = dx_{s} + dy_{s}, \; \forall s \in S
$$

### Objective Function

#### 1. Minimize Distance

$$
min \quad \sum_{s \in S} Distance_{s}
$$

## Expected Result

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

val settlements: List<Settlement> = ... // residential area data

// create a model instance
val metaModel = LinearMetaModel("demo9")

// define variables
val x = IntVar("x")
val y = IntVar("y")
metaModel.add(x)
metaModel.add(y)

// define intermediate expressions
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

// define objective function
metaModel.minimize(sum(distance[_a]))

// solve the model
val solver = ScipLinearSolver()
when (val ret = solver(metaModel)) {
    is Ok -> {
        metaModel.tokens.setSolution(ret.value.solution)
    }

    is Failed -> {}
}

// parse results
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

For the complete implementation, please refer to:

- [Kotlin](https://github.com/fuookami/ospf/blob/main/examples/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/core_demo/Demo9.kt)
