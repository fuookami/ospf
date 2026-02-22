# 最大值（上界、上确界）

## 函数形式

$$
y = \max(x_{1}, \, x_{2}, \, \cdots, \, x_{n})
$$

## 常量定义

$$
M = \max_{i \in P}(\max(|\min x_{i}|, \, |\max x_{i}|))
$$

## 额外变量

$\text{minmax} \in \mathbb{R}$：上界。

$u_{i} \in \{0, \, 1 \}$：表示上确界是否是 $x_{i}$ 的标志位。

## 导出符号

$$
y = \text{minmax}
$$

## 数学模型

$$
\text{s.t.} \quad \text{minmax} \geq x_{i}, \; \forall i \in P
$$

上述模型约束了 $y$ 为 $x_{i}$ 的上界，适用于最小值目标函数。若需在约束条件或最大值目标函数中使用精确的 $y$ 值，需额外追加以下数学模型以确保 $y$ 取到上确界：

$$
\begin{align}
\text{s.t.} \quad & \text{minmax} & \leq & \, x_{i} + M \cdot (1 - u_{i}), & \; \forall i \in P \\ \; \\
& \sum_{i \in P} u_{i} & = & \, 1
\end{align}
$$

## 代码示例

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

**完整实现参考：**

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/Max.kt)

**完整样例参考：**

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/MaxTest.kt)
