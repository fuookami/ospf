# Bivariate Linear Piecewise Function

## Function Form

$$
z = \text{Blp}(x, y) = k_{i} x + k^{\prime}_{i} y + b_{i}, \; x \in [a_{i}, b_{i}], \; y \in [a^{\prime}_{i}, b^{\prime}_{i}] \; i = 0, 1, 2, \ldots
$$

## Constants

$$
M = \max_{t \in T}{({\max{(\max_{x \in \mathbb{R}, \, y \in \mathbb{R}} fu_{t}(x, y), \, \max_{x \in \mathbb{R}, \, y \in \mathbb{R}} fv_{t}(x, y))}})}
$$

## Additional Variables

$u_{t} \in [0, 1]$: Weight of vector $\vec{A_{i}B_{i}}$ in the $i$-th triangle.

$v_{t} \in [0, 1]$: Weight of vector $\vec{A_{i}C_{i}}$ in the $i$-th triangle.

$w_{t} \in \{ 0, \, 1\}$: Indicator variable for the $i$-th triangle.

## Derived Symbol

$$
z = \sum_{t \in T} (w_{t} \cdot z_{t, 0} + u_{t} \cdot (z_{t, 1} - z_{t, 0}) + v_{t} \cdot (z_{t, 2} - z_{t, 0}))
$$

## Mathematical Model

$$
\begin{align}
\text{s.t.} \quad & u_{t} + M \cdot (1 - w_{t}) & \geq & \; fu_{t}(x, y), & \; \forall t \in T \\ \; \\
& u_{t} - M \cdot (1 - w_{t}) & \leq & \; fu_{t}(x, y), & \; \forall t \in T \\ \; \\
& v_{t} + M \cdot (1 - w_{t}) & \geq & \; fv_{t}(x, y), & \; \forall t \in T \\ \; \\
& v_{t} - M \cdot (1 - w_{t}) & \leq & \; fv_{t}(x, y), & \; \forall t \in T \\ \; \\
& \sum_{t \in T} w_{t} & = & \; 1 \\
& u_{t} + v_{t} & \leq & \; w_{t}, & \; \forall t \in T
\end{align}
$$

where:

$$
\begin{align}
S_{t} = \frac{1}{2} \cdot (-y_{t, 1} \cdot x_{t, 2} + y_{t, 0} \cdot (-x_{t, 1} + x_{t, 2}) + x_{t, 0} \cdot (y_{t, 1} - y_{t, 2}) + x_{t, 1} \cdot y_{t, 2}), \; \forall t \in T \\
fu_{t}(x, y) = \frac{1}{2S_{t}} \cdot (y_{t, 0} \cdot x_{t, 2} - x_{t, 0} \cdot y_{t, 2} + (y_{t, 2} - y_{t, 0}) \cdot x + (x_{t, 0} - x_{t, 2}) \cdot y), \; \forall t \in T \\
fv_{t}(x, y) = \frac{1}{2S_{t}} \cdot (x_{t, 0} \cdot y_{t, 1} - y_{t, 0} \cdot x_{t, 1} + (y_{t, 0} - y_{t, 1}) \cdot x + (x_{t, 1} - x_{t, 0}) \cdot y), \; \forall t \in T
\end{align}
$$

## Derivation Process

### Fundamental Principle

$$
(\vec{P_{0}P} = u \cdot \vec{P_{0}P_{1}} + v \cdot \vec{P_{0}P_{2}}) \wedge (u \geq 0) \wedge (v \geq 0) \wedge (u + v \leq 1)) \Rightarrow (P \in \triangle P_{0}P_{1}P_{2})
$$

$$
(\exists t \in T((P \in t) \wedge (\not \exists t^{\prime} \in T((t \neq t^{\prime}) \wedge (P \in t^{\prime})))))
$$

### Algebraic Formulation of Fundamental Principle

$$
\begin{align}
S_{t} = \frac{1}{2} \cdot (-y_{t, 1} \cdot x_{t, 2} + y_{t, 0} \cdot (-x_{t, 1} + x_{t, 2}) + x_{t, 0} \cdot (y_{t, 1} - y_{t, 2}) + x_{t, 1} \cdot y_{t, 2}), \; \forall t \in T \\
fu_{t}(x, y) = \frac{1}{2S_{t}} \cdot (y_{t, 0} \cdot x_{t, 2} - x_{t, 0} \cdot y_{t, 2} + (y_{t, 2} - y_{t, 0}) \cdot x + (x_{t, 0} - x_{t, 2}) \cdot y), \; \forall t \in T \\
fv_{t}(x, y) = \frac{1}{2S_{t}} \cdot (x_{t, 0} \cdot y_{t, 1} - y_{t, 0} \cdot x_{t, 1} + (y_{t, 0} - y_{t, 1}) \cdot x + (x_{t, 1} - x_{t, 0}) \cdot y), \; \forall t \in T
\end{align}
$$

### Logical Properties:

$$
\forall t \in T(w_{t} \in \{ 0, 1 \})
$$

$$
\exists t \in T((w_{t} = 1) \wedge (\not \exists t ^{\prime} \in T((t \neq t^{\prime}) \wedge (w_{t} = 1))))
$$

$$
\forall t \in T((u_{t} \in ([0, 1] \cap \mathbb{R}_{+})) \wedge (v_{t} \in ([0, 1] \cap \mathbb{R}_{+})))
$$

$$
\forall t \in T(((w_{t} = 1) \Rightarrow ((u_{t} = fu_{t}(x, y)) \wedge (v_{t} = fv_{t}(x, y)))) \wedge ((w_{t} = 0) \Rightarrow ((u_{t} = 0) \wedge (v_{t} = 0))))
$$

### (Quadratic) Mathematical Model:

$$
\begin{align}
\text{s.t.} \quad & u_{t} & = & \; w_{t} \cdot fu_{t}(x, y), & \; \forall t \in T \\ \; \\
& v_{t} & = & \; w_{t} \cdot fv_{t}(x, y), & \; \forall t \in T \\ \; \\
& \sum_{t \in T} w_{t} & = & \; 1 \\
& u_{t} + v_{t} & \leq & \; w_{t}, & \; \forall t \in T
\end{align}
$$

Linearization yields the aforementioned mathematical model.

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

val x = URealVar("x")
val y = URealVar("y")
x.range.leq(Flt64.two)
y.range.leq(Flt64.two)

val blp = BivariateLinearPiecewiseFunction(
    x = x,
    y = y,
    points = listOf(
        point3(),
        point3(x = Flt64.two),
        point3(y = Flt64.two),
        point3(x = Flt64.two, y = Flt64.two),
        point3(x = Flt64.one, y = Flt64.one, z = Flt64.one)
    ),
    name = "z"
)

val model = LinearMetaModel()
model.add(x)
model.add(y)
model.add(blp)
model.maximize(blp)

val solver = ScipLinearSolver()
val result = runBlocking { solver(model) }
assert(result.value!!.solution[0] eq Flt64.one)
assert(result.value!!.solution[1] eq Flt64.one)
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/BivariateLinearPiecewise.kt)

**Complete Example Reference:**

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/BLPTest.kt)
