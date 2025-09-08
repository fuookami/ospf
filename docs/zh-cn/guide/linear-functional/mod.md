# 整除（取余）

## 形式

$$
y = Mod(x, d) = x \mod d
$$

## 额外变量

$q \in Z$：$\frac{x}{d}$ 的整数部分。

$r \in [0, |d|)$：$\frac{x}{d}$ 的余数部分。

## 导出符号

$$
y = r
$$

## 数学模型

$$
\begin{align}
s.t. \quad & x = d \cdot q + r
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
x.range.eq(Flt64.three)
val mod = ModFunction(x, Flt64(0.7), name = "mod")

val model = LinearMetaModel()
model.add(x)
model.add(mod)
model.minimize(mod)

val solver = ScipLinearSolver()
val result = runBlocking { solver(model) }
assert(result.value!!.obj eq Flt64(0.2))
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/Mod.kt)

完整样例请参考：

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/ModTest.kt)
