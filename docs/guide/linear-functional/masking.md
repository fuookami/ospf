# Masking Function

## Function Form

$$
y = \text{masking}(x, \, mask) = x \cdot mask = \begin{cases}
x, & mask \\ \; \\
0, & \neg mask
\end{cases}
$$

where $mask \in \{ 0, \, 1 \}$.

## Constants

$$
m = \max(|x|)
$$

## Additional Variables

$y^{\prime} \in \mathbb{R}$: Represents the masked value.

## Derived Symbol

$$
y = y^{\prime}
$$

## Mathematical Model

$$
\begin{align}
\text{s.t.} \quad & y^{\prime} \leq x + m \cdot (1 - mask) \\ \; \\
& y^{\prime} \geq x - m \cdot (1 - mask) \\ \; \\
& y^{\prime} \leq m \cdot mask \\ \; \\
& y^{\prime} \geq -m \cdot mask \\ \; \\
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
val mask = BinVar("mask")
val masking = MaskingFunction(
    x = x,
    mask = mask,
    name = "masking"
)
val solver = ScipLinearSolver()

val model1 = LinearMetaModel()
model1.add(x)
model1.add(mask)
model1.add(masking)
model1.minimize(masking)

val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq -Flt64.three)
assert(result1.value!!.solution[0] eq -Flt64.three)
assert(result1.value!!.solution[1] eq Flt64.one)

val model2 = LinearMetaModel()
model2.add(x)
model2.add(mask)
model2.add(masking)
model2.addConstraint(mask eq false)
model2.minimize(masking)

val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj eq Flt64.zero)
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/Masking.kt)

**Complete Example Reference:**

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/MaskingTest.kt)
