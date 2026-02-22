# Slack

## Function Form

$$
z = \text{slack}(x, \, y) = \begin{cases}
\max(0, \, x - y), & \text{compute positive slack} \\ \; \\
\max(0, \, y - x), & \text{compute negative slack} \\ \; \\
|x - y|, & \text{compute both positive and negative slack}
\end{cases}
$$

## Additional Variables

$neg \in \mathbb{R} - \mathbb{R}^{-}$: Negative slack.

$pos \in \mathbb{R} - \mathbb{R}^{-}$: Positive slack.

## Derived Symbol

$$
z = neg + pos
$$

## Mathematical Model

### Compute Both Positive and Negative Slack

$$
\begin{align}
\text{s.t.} \quad & x + neg - pos = y
\end{align}
$$

### Compute Positive Slack

$$
\begin{align}
\text{s.t.} \quad & x - pos \leq y
\end{align}
$$

### Compute Negative Slack

$$
\begin{align}
\text{s.t.} \quad & x + neg \geq y
\end{align}
$$

## Code Example

::: code-group

```kotlin
import kotlinx.coroutines.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.linear_function.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

val x = RealVar("x")
x.range.leq(Flt64.two)
x.range.geq(-Flt64.three)
val slack = SlackFunction(
    x = x,
    y = Flt64.five,
    name = "slack"
)

val model = LinearMetaModel()
model.add(x)
model.add(slack)
model.minimize(slack)

val solver = ScipLinearSolver()
val result = runBlocking { solver(model) }
assert(result.value!!.obj eq Flt64.three)
assert(result.value!!.solution[0] eq Flt64.two)
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/Slack.kt)

**Complete Example Reference:**

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/SlackTest.kt)
