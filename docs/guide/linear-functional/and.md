# Logical AND

## Function Form

$$
y = \text{And}(x_{1}, \, x_{2}, \, \ldots, \, x_{i}) = \begin{cases}
1, & \bigwedge_{i} x_{i} \\ \; \\
0, & \neg \bigwedge_{i} x_{i}
\end{cases}
$$

## Binary Case

### Additional Variables

$y^{\prime} \in \{ 0, 1 \}$: logical AND value.

### Basic Formula

$$
y = y^{\prime}
$$

### Mathematical Model

$$
\begin{align}
\text{s.t.} \quad & y & \leq & \, x_{i}, & \; \forall i \in P \\ \; \\
& y & \geq & \, \sum_{i \in P} x_{i} - |P| + 1
\end{align}
$$

## Non-Binary Case

### Basic Formula

$$
y = \text{Bin}(\min(x_{1}, \, x_{2}, \, \ldots, \, x_{i}))
$$

where $\text{Bin}(x)$ can refer to [Binarization](/guide/linear-functional/bin), and $\min(x)$ can refer to [Minimum Value](/guide/linear-functional/min).

## Code Example

### Binary Case

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

val x = BinVar("x")
val y = BinVar("y")
val and = AndFunction(
    listOf(x, y),
    "and"
)
val solver = ScipLinearSolver()

val model1 = LinearMetaModel()
model1.add(x)
model1.add(y)
model1.add(and)
model1.addConstraint(!and)
model1.maximize(x + y)
val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq Flt64.one)

val model2 = LinearMetaModel()
model2.add(x)
model2.add(y)
model2.add(and)
model2.addConstraint(and)
model2.minimize(x + y)
val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj eq Flt64.two)
```

:::

### Non-Binary Case

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

val x = UIntVar("x")
x.range.leq(UInt64.one)
val y = UIntVar("y")
y.range.leq(UInt64.two)
val and = AndFunction(
    listOf(x, y),
    "and"
)
val solver = ScipLinearSolver()

val model1 = LinearMetaModel()
model1.add(x)
model1.add(y)
model1.add(and)
model1.addConstraint(and)
model1.maximize(x + y)
val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq Flt64.three)

val model2 = LinearMetaModel()
model2.add(x)
model2.add(y)
model2.add(and)
model2.addConstraint(and)
model2.minimize(x + y)
val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj eq Flt64.two)
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/And.kt)

**Complete Example Reference:**

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/AndTest.kt)
