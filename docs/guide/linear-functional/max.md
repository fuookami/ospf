# Maximum Value (Upper Bound, Upper Limit)

## Function Form

$$
y = \max(x_{1}, \, x_{2}, \, \cdots, \, x_{n})
$$

## Constant Definition

$$
M = \max_{i \in P}(\max(|\min x_{i}|, \, |\max x_{i}|))
$$

## Additional Variables

$\text{minmax} \in \mathbb{R}$: upper bound.

$u_{i} \in \{0, \, 1 \}$: indicates whether the upper limit is $x_{i}$.

## Basic Formula

$$
y = \text{minmax}
$$

## Mathematical Model

$$
\text{s.t.} \quad \text{minmax} \geq x_{i}, \; \forall i \in P
$$

The above model only provides an upper bound for $y$ relative to $x_{i}$, suitable for minimization objective functions. If precise $y$ values are required in equality constraints or maximization objective functions, the following mathematical model must be additionally appended to enforce $y$ to reach the upper limit:

$$
\begin{align}
\text{s.t.} \quad & \text{minmax} & \leq & \, x_{i} + M \cdot (1 - u_{i}), & \; \forall i \in P \\ \; \\
& \sum_{i \in P} u_{i} & = & \, 1
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
x.range.leq(Flt64.five)
x.range.geq(Flt64.three)
val y = RealVar("y")
y.range.leq(Flt64.ten)
y.range.geq(Flt64.two)
val solver = ScipLinearSolver()

val minmax = MinMaxFunction(listOf(x, y), name = "minmax")
val model1 = LinearMetaModel()
model1.add(x)
model1.add(y)
model1.add(minmax)
model1.minimize(minmax)
val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq Flt64.three)

val model2 = LinearMetaModel()
model2.add(x)
model2.add(y)
model2.add(minmax)
model2.maximize(minmax)
val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj eq Flt64.ten)

val max = MaxFunction(listOf(x, y), name = "max")
val model3 = LinearMetaModel()
model3.add(x)
model3.add(y)
model3.add(max)
model3.minimize(max)
val result3 = runBlocking { solver(model3) }
assert(result3.value!!.obj eq Flt64.three)

val model4 = LinearMetaModel()
model4.add(x)
model4.add(y)
model4.add(max)
model4.maximize(max)
val result4 = runBlocking { solver(model4) }
assert(result4.value!!.obj gr Flt64.ten)    // inf
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/Max.kt)

**Complete Example Reference:**

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/MaxTest.kt)
