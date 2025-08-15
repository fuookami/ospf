# 绝对值

## 形式

$$
y = Abs(x) = |x|
$$

## 常量

$$
m = \max(|x|)
$$

## 额外变量

$neg \in [0, 1]$：向负数下限的相对距离。

$pos \in [0, 1]$：向正数上限的相对距离。

$p \in \{ 0, \, 1 \}$：正数标记。

## 导出符号

$$
y = m \cdot pos + m \cdot neg
$$

## 数学模型

$$
\begin{align}
s.t. \quad & x = -m \cdot neg + m \cdot pos
\end{align}
$$

上述模型约束了 $y$ 为 $|x|$ 的上界，可用于最小值目标函数。如果需要精确 $y$ 用于约束或者最大值目标函数，会额外追加以下数学模型：

$$
\begin{align}
s.t. \quad & neg + pos \leq 1 \\
& p \geq pos \\
& neg \leq 1 - p
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
val abs = AbsFunction(LinearPolynomial(x), name = "abs")

x.range.leq(Flt64.two)
x.range.geq(-Flt64.three)

val model = LinearMetaModel()
model.add(x)
model.add(abs)
model.maximize(abs)

val solver = ScipLinearSolver()
val result = runBlocking { solver(model) }
assert(result.value!!.obj eq Flt64.three)
assert(result.value!!.solution[0] eq -Flt64.three)
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/Abs.kt)

完整样例请参考：

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/AbsTest.kt)
