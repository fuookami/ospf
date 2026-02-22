# Univariate Linear Piecewise Function

## Function Form

$$
y = \text{Ulp}(x) = k_{i} x + b_{i}, \; x \in [a_{i}, b_{i}], \; i = 0, 1, 2, \ldots
$$

## Constants

Let the point set of this linear piecewise function be $P$, where each point $i$ corresponds to the value $(x_{i}, y_{i})$.

## Additional Variables

$k_{i} \in [0, 1]$: Relative distance to point $i$.

$b_{i} \in \{ 0, 1 \}$: Indicator for being on the line segment formed by points $i$ and $i + 1$.

## Derived Symbol

$$
y = \sum_{i \in P} k_{i} \cdot y_{i}
$$

## Mathematical Model

$$
\begin{align}
\text{s.t.} \quad & x & = & \; \sum_{i \in P} k_{i} \cdot x_{i} \\
& \sum_{i \in P} k_{i} & = & \; 1 \\
& \sum_{i \in P} b_{i} & = & \; 1 \\
& k_{i} & \leq & \; b_{i - 1} + b_{i}, & \; \forall i \in P 
\end{align}
$$

## Code Example

::: code-group

```kotlin
import kotlinx.coroutines.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.utils.math.geometry.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

val x = URealVar("x")
x.range.leq(Flt64.two)

val ulp = UnivariateLinearPiecewiseFunction(
    x = x,
    points = listOf(
        point2(),
        point2(x = Flt64.one, y = Flt64.two),
        point2(x = Flt64.two, y = Flt64.one)
    ),
    name = "y"
)

val model = LinearMetaModel()
model.add(x)
model.add(ulp)
model.maximize(ulp)

val solver = ScipLinearSolver()
val result = runBlocking { solver(model) }
assert(result.value!!.obj eq Flt64.two)
assert(result.value!!.solution[0] eq Flt64.one)
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/UnivariateLinearPiecewise.kt)

**Complete Example Reference:**

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/ULPTest.kt)
