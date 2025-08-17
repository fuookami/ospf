# 掩码函数

## 形式

$$
y = masking(x, \, mask) = x \cdot mask = \begin{cases}
x, & mask \\ \; \\
0, & \neg mask
\end{cases}
$$

其中，$mask \in \{ 0, \, 1 \}$ 。

## 常量

$$
m = \max(|x|)
$$

## 额外变量

$y^{\prime} \in R$：表示掩码后的值。

## 导出符号

$$
y = y^{\prime}
$$

## 数学模型

$$
\begin{align}
s.t. \quad & y^{\prime} \leq x + m \cdot (1 - mask) \\ \; \\
& y^{\prime} \geq x - m \cdot (1 - mask) \\ \; \\
& y^{\prime} \leq m \cdot mask \\ \; \\
& y^{\prime} \geq -m \cdot mask \\ \; \\
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
val mask = BinVar("mask")
val masking = MaskingFunction(
    x = x,
    mask = mask,
    name = "masking"
)
val solver = ScipLinearSolver()

val model1 = LinearMetaModel()
model1.add(x)
model1.add(mask)
model1.add(masking)
model1.minimize(masking)

val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq -Flt64.three)
assert(result1.value!!.solution[0] eq -Flt64.three)
assert(result1.value!!.solution[1] eq Flt64.one)

val model2 = LinearMetaModel()
model2.add(x)
model2.add(mask)
model2.add(masking)
model2.addConstraint(mask eq false)
model2.minimize(masking)

val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj eq Flt64.zero)
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/Masking.kt)

完整样例请参考：

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/MaskingTest.kt)
