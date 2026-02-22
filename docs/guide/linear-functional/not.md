# Logical NOT

$$
y = \text{Not}(x) = \begin{cases}
1, & \neg x \\ \; \\
0, & x
\end{cases}
$$

## Discrete Case

### Constants

$$
m = \max(x)
$$

### Additional Variables

$y^{\prime} \in \{ 0, \, 1 \}$: Logical value indicating the polynomial is false.

### Derived Symbol

$$
y = y^{\prime}
$$

### Mathematical Model

$$
\begin{align}
\text{s.t.} \quad & m \cdot (1 - y^{\prime}) & \geq & \, x \\ \; \\
& 1 - y^{\prime} & \leq & \, x
\end{align}
$$

That is:

$$
y = \begin{cases}
1, & x = 0 \\ \; \\
0, & x \geq 1
\end{cases}
$$

## Continuous (Non-Precise) Case

### Constants

$$
m = \max(x)
$$

### Additional Variables

$b \in [0, \, 1]$: Normalized value of the polynomial.

$y^{\prime} \in \{ 0, \, 1 \}$: Logical value indicating the polynomial is false.

### Derived Symbol

$$
y = y^{\prime}
$$

### Mathematical Model

$$
\begin{align}
\text{s.t.} \quad & x & = & \, m \cdot b \\ \; \\
& 1 - y^{\prime} & \geq & \, b \\ \; \\
& \epsilon \cdot (1 - y^{\prime}) & \leq & b
\end{align}
$$

That is:

$$
y = \begin{cases}
1, & x = 0 \\ \; \\
0, & x \geq \epsilon
\end{cases}
$$

## Continuous (Precise) Case

### Derived Symbol

$$
y = \text{Ulp}(x)
$$

$\text{Ulp}(x)$ can refer to [Unit Linear Piecewise Function](/guide/linear-functional/ulp), using the sampling points $\{ (0, \, 1), \; (p - \epsilon, \, 1), \; (p, \, 0), \; (\max(x), \, 0) \}$.

That is:

$$
y = \begin{cases}
1, & 0 \leq x < p \\ \; \\
0, & x \geq p
\end{cases}
$$

## Code Example

### Discrete Case

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
val not = NotFunction(x, name = "not")
val solver = ScipLinearSolver()

val model1 = LinearMetaModel()
model1.add(x)
model1.add(not)
model1.addConstraint(not)
model1.maximize(x)
val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq Flt64.zero)

val model2 = LinearMetaModel()
model2.add(x)
model2.add(not)
model2.addConstraint(!not)
model2.minimize(x)
val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj eq Flt64.one)
```

:::

### Continuous Case

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

val x = URealVar("x")
x.range.leq(Flt64.one)
val not = NotFunction(x, name = "not")
val solver = ScipLinearSolver()

val model1 = LinearMetaModel()
model1.add(x)
model1.add(not)
model1.addConstraint(not)
model1.maximize(x)
val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq Flt64.zero)

val model2 = LinearMetaModel()
model2.add(x)
model2.add(not)
model2.addConstraint(!not)
model2.minimize(x)
val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj geq Flt64(1e-6))
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/Not.kt)

**Complete Example Reference:**

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/NotTest.kt)
