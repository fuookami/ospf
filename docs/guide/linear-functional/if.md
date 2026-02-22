# If Function

## Function Form

$$
y = \text{If}(x \cdot rhs) = \begin{cases}
1, & x \cdot rhs \\ \; \\
0, & \neg (x \cdot rhs) 
\end{cases}
$$

where $\cdot$ can be $\leq$, $\geq$, or $=$.

## Additional Variables

$k_{i} \in [0, 1]$: Linear piecewise weights.

$y^{\prime} \in \{ 0, \, 1 \}$: Logical value of the inequality.

## Derived Symbol

$$
y = y^{\prime}
$$

## Mathematical Model

If $\cdot$ is $\leq$, we have:

$$
\text{s.t.} \quad \begin{cases}
  \begin{cases}
    x = k_{0} \cdot \min(x) + k_{1} \cdot rhs + k_{2} \cdot \max(x) \\ \; \\
    k_{0} \leq y^{\prime} \\ \; \\
    k_{2} \leq 1 - y^{\prime} \\ \; \\
    k_{2} \geq \epsilon \cdot (1 - y^{\prime})
  \end{cases}, & \; rhs \in [\min(x), \, \max(x)] \\ \; \\
  \quad y^{\prime} = \text{bin}(\max(x) \leq rhs), & \; rhs \notin [\min(x), \, \max(x)]
\end{cases}
$$

If $\cdot$ is $\geq$, we have:

$$
\text{s.t.} \quad \begin{cases}
  \begin{cases}
    x = k_{0} \cdot \min(x) + k_{1} \cdot rhs + k_{2} \cdot \max(x) \\ \; \\
    k_{0} \leq 1 - y^{\prime} \\ \; \\
    k_{0} \geq \epsilon \cdot (1 - y^{\prime}) \\ \; \\
    k_{2} \geq y^{\prime}
  \end{cases}, & \; rhs \in [\min(x), \, \max(x)] \\ \; \\
  \quad y^{\prime} = \text{bin}(\max(x) \leq rhs), & \; rhs \notin [\min(x), \, \max(x)]
\end{cases}
$$

If $\cdot$ is $=$, we have:

$$
\text{s.t.} \quad \begin{cases}
  \begin{cases}
    x = k_{0} \cdot \min(x) + k_{1} \cdot rhs + k_{2} \cdot \max(x) \\ \; \\
    k_{0} + k_{2} \leq 1 - y^{\prime} \\ \; \\
    k_{0} + k_{2} \geq \epsilon \cdot (1 - y^{\prime})
  \end{cases}, & \; rhs \in [\min(x), \, \max(x)] \\ \; \\
  \quad y^{\prime} = \text{bin}(\max(x) \leq rhs), & \; rhs \notin [\min(x), \, \max(x)]
\end{cases}
$$

where,

$$
\text{bin}(cond) = \begin{cases}
1, \; cond \\ \; \\
0, \; \neg cond
\end{cases}
$$

and $\epsilon$ satisfies the following property:

$$
(\epsilon = 0) \Leftrightarrow ((x = rhs) \Rightarrow \neg(x \cdot rhs))
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

val x = URealVar("x")
x.range.geq(Flt64.two)
x.range.leq(Flt64.five)
val condition1 = IfFunction(x geq Flt64.three, name = "c1")
val condition2 = IfFunction(x geq Flt64.three, epsilon = Flt64.zero, name = "c2")
val condition3 = IfFunction(x leq Flt64.one, name = "c3")
val solver = ScipLinearSolver()

val model1 = LinearMetaModel()
model1.add(x)
model1.add(condition1)
model1.addConstraint(condition1 eq true)
model1.minimize(x)
val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq Flt64.three)

val model2 = LinearMetaModel()
model2.add(x)
model2.add(condition1)
model2.addConstraint(condition1 eq false)
model2.maximize(x)
val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj ls Flt64.three)

val model3 = LinearMetaModel()
model3.add(x)
model3.add(condition2)
model3.addConstraint(condition2 eq false)
model3.maximize(x)
val result3 = runBlocking { solver(model3) }
assert(result3.value!!.obj eq Flt64.three)

val model4 = LinearMetaModel()
model4.add(x)
model4.add(condition3)
model4.maximize(condition3)
val result4 = runBlocking { solver(model4) }
assert(result4.value!!.obj eq Flt64.zero)
```

:::

**Complete Implementation Reference:**

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/If.kt)

**Complete Example Reference:**

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/IfTest.kt)
