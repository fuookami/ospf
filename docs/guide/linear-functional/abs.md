# Absolute Value

## Function Form

$$
y = \text{Abs}(x) = |x|
$$

## Constant Definition

$$
M = \max(|x|)
$$

## Additional Variables

$neg \in [0, 1]$: indicates the relative distance to the lower bound in the negative direction.

$pos \in [0, 1]$: indicates the relative distance to the upper bound in the positive direction.

$p \in \{ 0, \, 1 \}$: sign indicator variable.

## Basic Formula

$$
y = M \cdot pos + M \cdot neg
$$

## Mathematical Model

$$
\begin{align}
\text{s.t.} \quad & x = -M \cdot neg + M \cdot pos
\end{align}
$$

The above model only provides an upper bound for $y$ relative to $|x|$, suitable for minimization objective functions. If precise $y$ values are required in equality constraints or maximization objective functions, the following mathematical model must be additionally appended:

$$
\begin{align}
\text{s.t.} \quad & neg + pos & \leq & \; 1 \\
& pos & \leq & \; p \\
& neg & \leq & \; 1 - p
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
val abs = AbsFunction(x, name = "abs")

x.range.leq(Flt64.two)
x.range.geq(-Flt64.three)

val model = LinearMetaModel()
model.add(x)
model.add(abs)
model.maximize(abs)

val solver = ScipLinearSolver()
val result = runBlocking { solver(model) }
assert(result.value!!.obj eq Flt64.three)
assert(result.value!!.solution[0] eq -Flt64.three)
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/Abs.kt)

**Complete Example Reference:**

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/AbsTest.kt)
