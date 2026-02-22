# Modulo Function (Remainder)

## Function Form

$$
y = \text{Mod}(x, d) = x \mod d
$$

## Additional Variables

$q \in \mathbb{Z}$: Integer part of $\frac{x}{d}$.

$r \in [0, |d|)$: Remainder part of $\frac{x}{d}$.

## Derived Symbol

$$
y = r
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
x.range.eq(Flt64.three)
val mod = ModFunction(x, Flt64(0.7), name = "mod")

val model = LinearMetaModel()
model.add(x)
model.add(mod)
model.minimize(mod)

val solver = ScipLinearSolver()
val result = runBlocking { solver(model) }
assert(result.value!!.obj eq Flt64(0.2))
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/Mod.kt)

**Complete Example Reference:**

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/ModTest.kt)
