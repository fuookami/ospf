# 半函数

## 形式

$$
y = semi(x) = max(0, \, x) = \begin{cases}
x, & x > 0 \\ \; \\
0, & x \leq 0
\end{cases}
$$

## 常量

$$
m = \max(|x|)
$$

## 额外变量

$u \in \{ 0, 1 \}$：$x > 0$ 的判定。

$y^{\prime} \in R - R^{-}$：表示 $max(0, x)$。

## 导出符号

$$
y = y^{\prime}
$$

## 数学模型

$$
\begin{align}
s.t. \quad & y \geq x \\ \; \\
& y \leq x + m \cdot u \\ \; \\
& y \leq m \cdot (1 - u)
\end{align}
$$

## 样例

::: code-group

```kotlin
import kotlinx.coroutines.*
import fuookami.ospf.kotlin.utils.math.*
import fuookami.ospf.kotlin.core.frontend.variable.*
import fuookami.ospf.kotlin.core.frontend.expression.polynomial.*
import fuookami.ospf.kotlin.core.frontend.expression.symbol.quadratic_function.*
import fuookami.ospf.kotlin.core.frontend.inequality.*
import fuookami.ospf.kotlin.core.frontend.model.mechanism.*
import fuookami.ospf.kotlin.core.backend.plugins.scip.*

val x = URealVar("x")
x.range.leq(Flt64.three)
val y = URealVar("y")
y.range.geq(Flt64.two)
y.range.leq(Flt64.five)
val z = BTerVar("z")
val semi = SemiFunction(
    z * x - y,
    name = "semi"
)
val solver = ScipQuadraticSolver()

val model1 = QuadraticMetaModel()
model1.add(x)
model1.add(y)
model1.add(z)
model1.add(semi)
model1.minimize(semi)

val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq Flt64.zero)
assert(result1.value!!.solution[1] geq result1.value!!.solution[2] * result1.value!!.solution[0])

val model2 = QuadraticMetaModel()
model2.add(x)
model2.add(y)
model2.add(z)
model2.add(semi)
model2.maximize(semi)

val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj eq Flt64.one)
assert(result2.value!!.solution[0] eq Flt64.three)
assert(result2.value!!.solution[1] eq Flt64.two)
assert(result2.value!!.solution[2] eq Flt64.one)
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/quadratic_function/Semi.kt)

完整样例请参考：

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/quadratic_function/SemiTest.kt)
