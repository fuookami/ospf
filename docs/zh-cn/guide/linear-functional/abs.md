# 绝对值

## 函数形式

$$
y = \text{Abs}(x) = |x|
$$

## 常量定义

$$
M = \max(|x|)
$$

## 额外变量

$neg \in [0, 1]$：表示向负数下限的相对距离。

$pos \in [0, 1]$：表示向正数上限的相对距离。

$p \in \{ 0, \, 1 \}$：正数标记变量。

## 导出符号

$$
y = M \cdot pos + M \cdot neg
$$

## 数学模型

$$
\begin{align}
\text{s.t.} \quad & x = -M \cdot neg + M \cdot pos
\end{align}
$$

上述模型约束 $y$ 为 $|x|$ 的上界，适用于最小值目标函数。若需在约束条件或最大值目标函数中使用精确的 $y$ 值，需额外追加以下数学模型：

$$
\begin{align}
\text{s.t.} \quad & neg + pos & \leq & \; 1 \\
& pos & \leq & \; p \\
& neg & \leq & \; 1 - p
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
val abs = AbsFunction(x, name = "abs")

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

**完整实现参考：**

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/Abs.kt)

**完整样例参考：**

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/AbsTest.kt)
