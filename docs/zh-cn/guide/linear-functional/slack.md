# 松弛

## 形式

$$
z = slack(x, \, y) = \begin{cases}
max(0, \, x - y), & 计算正松弛\\
max(0, \, y - x), & 计算负松弛\\
|x - y|, & 计算正负松弛
\end{cases}
$$

## 额外变量

$neg \in R - R^{-}$：负松弛。

$pos \in R - R^{-}$：正松弛。

## 导出符号

$$
z = neg + pos
$$

## 数学模型

### 计算正负松弛

$$
\begin{align}
s.t. \quad & x + neg - pos = y
\end{align}
$$

### 计算正松弛

$$
\begin{align}
s.t. \quad & x - pos \leq y
\end{align}
$$

### 计算负松弛

$$
\begin{align}
s.t. \quad & x + neg \geq y
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
x.range.leq(Flt64.two)
x.range.geq(-Flt64.three)
val slack = SlackFunction(
    x = x,
    y = Flt64.five,
    name = "slack"
)

val model = LinearMetaModel()
model.add(x)
model.add(slack)
model.minimize(slack)

val solver = ScipLinearSolver()
val result = runBlocking { solver(model) }
assert(result.value!!.obj eq Flt64.three)
assert(result.value!!.solution[0] eq Flt64.two)
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/Slack.kt)

完整样例请参考：

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/SlackTest.kt)
