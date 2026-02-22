# Minimum Value (Lower Bound, Infimum)

## Function Form

$$
y = \min(x_{1}, \, x_{2}, \, \cdots, \, x_{n})
$$

## Constant Definition

$$
M = \max_{i \in P}(\max(|x_{i}|))
$$

## Additional Variables

$\text{minmax} \in \mathbb{R}$: Lower bound.

$u_{i} \in \{0, \, 1 \}$: Indicator variable for whether the infimum is $x_{i}$.

## Derived Symbol

$$
y = \text{maxmin}
$$

## Mathematical Model

$$
\text{s.t.} \quad \text{maxmin} \leq x_{i}, \; \forall i \in P
$$

The above model constrains $y$ to be a lower bound of $x_{i}$, suitable for maximization objective functions. If precise $y$ values are required in equality constraints or minimization objective functions, the following mathematical model must be additionally appended to ensure $y$ reaches the infimum:

$$
\begin{align}
\text{s.t.} \quad & \text{maxmin} & \geq & \, x_{i} - M \cdot (1 - u_{i}), & \; \forall i \in P \\ \; \\
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

val maxmin = MaxMinFunction(listOf(x, y), name = "maxmin")
val model1 = LinearMetaModel()
model1.add(x)
model1.add(y)
model1.add(maxmin)
model1.minimize(maxmin)
val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq Flt64.two)

val model2 = LinearMetaModel()
model2.add(x)
model2.add(y)
model2.add(maxmin)
model2.maximize(maxmin)
val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj eq Flt64.five)

val min = MinFunction(listOf(x, y), name = "min")
val model3 = LinearMetaModel()
model3.add(x)
model3.add(y)
model3.add(min)
model3.minimize(min)
val result3 = runBlocking { solver(model3) }
assert(result3.value!!.obj ls Flt64.zero)       // -inf

val model4 = LinearMetaModel()
model4.add(x)
model4.add(y)
model4.add(min)
model4.maximize(min)
val result4 = runBlocking { solver(model4) }
assert(result4.value!!.obj eq Flt64.five)
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/Min.kt)

**Complete Example Reference:**

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/MinTest.kt)
