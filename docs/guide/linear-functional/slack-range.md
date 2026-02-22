# Slack (Range)

## Function Form

$$
y = \text{slack\_range}(x, \, lb, \, ub) = \begin{cases}
\max(0, \, x - ub), & \text{compute positive slack} \\ \; \\
\max(0, \, lb - x), & \text{compute negative slack} \\ \; \\
\min(|x - ub|, \, |x - lb|), & \text{compute both positive and negative slack}
\end{cases}
$$

## Additional Variables

$neg \in \mathbb{R} - \mathbb{R}^{-}$: Negative slack.

$pos \in \mathbb{R} - \mathbb{R}^{-}$: Positive slack.

## Derived Symbol

$$
y = neg + pos
$$

## Mathematical Model

$$
\begin{align}
\text{s.t.} \quad & x - pos \leq ub \\ \; \\
& x + neg \geq lb
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
val slack = SlackRangeFunction(
    x = x,
    lb = Flt64.five,
    ub = Flt64(6.0),
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

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/SlackRange.kt)

**Complete Example Reference:**

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/SlackRangeTest.kt)
