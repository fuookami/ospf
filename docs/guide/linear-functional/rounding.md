# Rounding Function (Round to Nearest)

## Function Form

$$
y = \text{Round}(x, d) = \lceil \frac{x}{d} \rfloor
$$

## Additional Variables

$q \in \mathbb{Z}$: Integer part of $\frac{x}{d}$.

$r \in [-\frac{|d|}{2}, \frac{|d|}{2})$: Remainder part of $\frac{x}{d}$.

## Derived Symbol

$$
y = q
$$

## Mathematical Model

$$
\begin{align}
\text{s.t.} \quad & x = d \cdot q + r
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
x.range.geq(Flt64.two)
val rounding = RoundingFunction(x, Flt64(0.7), "rounding")
val solver = ScipLinearSolver()

val model1 = LinearMetaModel()
model1.add(x)
model1.add(rounding)
model1.minimize(rounding)
val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq Flt64.three)
assert(result1.value!!.solution[0].roundTo(5) leq Flt64(2.45))

val model2 = LinearMetaModel()
model2.add(x)
model2.add(rounding)
model2.maximize(rounding)
val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj eq Flt64(7.0))
assert(result2.value!!.solution[0].roundTo(5) geq Flt64(4.55))
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/Rounding.kt)

**Complete Example Reference:**

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/RoundingTest.kt)
