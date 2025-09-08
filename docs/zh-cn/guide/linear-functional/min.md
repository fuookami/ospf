# 最小值（下界、下确界）

## 形式

$$
y = \min(x_{1}, \, x_{2}, \, \cdots, \, x_{n})
$$

## 常量

$$
M = \max_{i \in P}(\max(|\min x_{i}|, \, |\max x_{i}|))
$$

## 额外变量

$maxmin \in R$ ：下界。

$u_{i} \in \{0, \, 1 \}$ ：下确界是 $x_{i}$ 的标志位。

## 导出符号

$$
y = maxmin
$$

## 数学模型

$$
s.t. \quad maxmin \leq x_{i}, \; \forall i \in P
$$

上述模型约束了 $y$ 为 $x_{i}$ 的下界，可用于最大值目标函数。如果需要精确 $y$ 用于约束或者最小值目标函数，会额外追加以下数学模型以确保 $y$ 取到下确界：

$$
\begin{align}
s.t. \quad & maxmin & \geq & \, x_{i} - M \cdot (1 - u_{i}), & \; \forall i \in P \\ \; \\
& \sum_{i \in P} u_{i} & = & \, 1
\end{align}
$$

## 样例

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

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/Min.kt)

完整样例请参考：

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/MinTest.kt)
