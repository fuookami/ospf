# 二值化

## 形式

$$
y = Bin(x) = \begin{cases}
1, & x \\ \; \\
0, & \neg x
\end{cases}
$$

## 离散

### 常量

$$
m = max(x)
$$

### 额外变量

$y^{\prime} \in \{ 0, \, 1 \}$ ：多项式的逻辑值。

### 导出符号

$$
y = y^{\prime}
$$

### 数学模型

$$
\begin{align}
s.t. \quad & m \cdot y^{\prime} & \geq & \, x \\ \; \\
& y^{\prime} & \leq & \, x
\end{align}
$$

即：

$$
y = \begin{cases}
1, & x \geq 1 \\ \; \\
0, & x = 0
\end{cases}
$$

## 连续（非精确）

### 常量

$$
m = max(x)
$$

### 额外变量

$b \in [0, \, 1]$ ：多项式的归一化值。

$y^{\prime} \in \{ 0, \, 1 \}$ ：多项式的逻辑值。

### 导出符号

$$
y = y^{\prime}
$$

### 数学模型

$$
\begin{align}
s.t. \quad & x & = & \, m \cdot b \\ \; \\
& y^{\prime} & \geq & \, b \\ \; \\
& \epsilon \cdot y^{\prime} & \leq & b
\end{align}
$$

即：

$$
y = \begin{cases}
1, & x \geq \epsilon \\ \; \\
0, & x = 0
\end{cases}
$$

## 连续（精确）

### 导出符号

$$
y = Ulp(x)
$$

$Ulp(x)$ 可参考 [一元分段线性函数](/zh-cn/guide/linear-functional/ulp)，使用的采样点为 $\{ (0, \, 0), \; (p - \epsilon, \, 0), \; (p, \, 1), \; (max(x), \, 1) \}$。

即：

$$
y = \begin{cases}
1, & x \geq p \\ \; \\
0, & 0 \leq x < p
\end{cases}
$$

## 样例

### 离散

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

val x = UIntVar("x")
x.range.leq(UInt64.two)
val bin = BinaryzationFunction(x, name = "bin")
val solver = ScipLinearSolver()

val model1 = LinearMetaModel()
model1.add(x)
model1.add(bin)
model1.minimize(bin)
val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq Flt64.zero)

val model2 = LinearMetaModel()
model2.add(x)
model2.add(bin)
model2.maximize(bin)
val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj eq Flt64.one)

val model3 = LinearMetaModel()
model3.add(x)
model3.add(bin)
model3.addConstraint(x eq Flt64.zero)
model3.maximize(bin)
val result3 = runBlocking { solver(model3) }
assert(result3.value!!.obj eq Flt64.zero)

val model4 = LinearMetaModel()
model4.add(x)
model4.add(bin)
model4.addConstraint(x eq Flt64.zero)
model4.minimize(bin)
val result4 = runBlocking { solver(model4) }
assert(result4.value!!.obj eq Flt64.zero)
```

:::

### 连续

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
x.range.leq(Flt64.two)
val bin = BinaryzationFunction(
    x,
    name = "bin"
)
val solver = ScipLinearSolver()

val model1 = LinearMetaModel()
model1.add(x)
model1.add(bin)
model1.minimize(bin)
val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq Flt64.zero)

val model2 = LinearMetaModel()
model2.add(x)
model2.add(bin)
model2.maximize(bin)
val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj eq Flt64.one)

val model3 = LinearMetaModel()
model3.add(x)
model3.add(bin)
model3.addConstraint(x eq Flt64.zero)
model3.maximize(bin)
val result3 = runBlocking { solver(model3) }
assert(result3.value!!.obj eq Flt64.zero)

val model4 = LinearMetaModel()
model4.add(x)
model4.add(bin)
model4.addConstraint(x eq Flt64(0.3))
model4.maximize(bin)
val result4 = runBlocking { solver(model4) }
assert(result4.value!!.obj eq Flt64.one)
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/Binaryzation.kt)

完整样例请参考：

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/BinTest.kt)
