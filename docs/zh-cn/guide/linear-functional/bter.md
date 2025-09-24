# 平衡三值化

## 形式

$$
y = BTer(x) = \begin{cases}
1, & x \\ \; \\
-1, & \neg x \\ \; \\
0, & \text{otherwise}
\end{cases}
$$

## 离散

### 常量

$$
m = max(|x|)
$$

### 额外变量

$y^{\prime}_{p} \in \{ 0, \, 1 \}$ ：多项式为真的逻辑值。

$y^{\prime}_{n} \in \{ 0, \, 1 \}$ ：多项式为假的逻辑值。

### 导出符号

$$
y = y^{\prime}_{p} - y^{\prime}_{n}
$$

### 数学模型

$$
\begin{align}
s.t. \quad & m \cdot y^{\prime}_{p} & \geq & \, x \\ \; \\
& -m \cdot y^{\prime}_{n} & \leq & \, x \\ \; \\
& x & \geq & \, (-m - 1) \cdot (1 - y^{\prime}_{p}) + 1 \\ \; \\
& x & \leq & \, (m + 1) \cdot (1 - y^{\prime}_{n}) - 1 \\ \; \\
& y^{\prime}_{p} + y^{\prime}_{n} & = & 1
\end{align}
$$

## 连续（非精确）

### 常量

$$
m = max(|x|)
$$

### 额外变量

$b_{p} \in [0, \, 1]$ ：多项式为正数的归一化值。

$b_{n} \in [0, \, 1]$ ：多项式为负数的归一化值。

$y^{\prime}_{p} \in \{ 0, \, 1 \}$ ：多项式为真的逻辑值。

$y^{\prime}_{n} \in \{ 0, \, 1 \}$ ：多项式为假的逻辑值。

### 导出符号

$$
y = y^{\prime}_{p} - y^{\prime}_{n}
$$

### 数学模型

$$
\begin{align}
s.t. \quad & x & = & \, -m \cdot b_{n} + m \cdot b_{p} \\ \; \\
& y_{p} & \geq & \, b_{p} \\ \; \\
& \epsilon \cdot y_{p} & \leq & b \\ \; \\
& y_{n} & \geq & \, b_{n} \\ \; \\
& \epsilon \cdot y_{n} & \leq & b \\ \; \\
& b_{n} + y_{p} & \leq & 1 \\ \; \\
& b_{p} + y_{n} & \leq & 1
\end{align}
$$

即：

$$
y = \begin{cases}
1, & x \geq \epsilon \\ \; \\
0, & x = 0 \\ \; \\
-1, & x \leq -\epsilon
\end{cases}
$$

## 连续（精确）

### 导出符号

$$
y = Ulp(x)
$$

$Ulp(x)$ 可参考 [一元分段线性函数](/zh-cn/guide/linear-functional/ulp)，使用的采样点为 $\{ (min(x), \, -1) \; (-p, \, -1), \; (-p + \epsilon, \, 0), \; (0, \, 0), \; (p - \epsilon, \, 0), \; (p, \, 1), \; (max(x), \, 1) \}$。

即：

$$
y = \begin{cases}
1, & x \geq p \\ \; \\
0, & -p < x < p \\ \; \\
-1, & x \leq -p
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

val x = IntVar("x")
x.range.leq(Int64.two)
x.range.geq(-Int64.two)
val bter = BalanceTernaryzationFunction(x, name = "bter")
val solver = ScipLinearSolver()

val model1 = LinearMetaModel()
model1.add(x)
model1.add(bter)
model1.minimize(bter)
val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq -Flt64.one)

val model2 = LinearMetaModel()
model2.add(x)
model2.add(bter)
model2.maximize(bter)
val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj eq Flt64.one)

val model3 = LinearMetaModel()
model3.add(x)
model3.add(bter)
model3.addConstraint(x geq Flt64.zero)
model3.minimize(bter)
val result3 = runBlocking { solver(model3) }
assert(result3.value!!.obj eq Flt64.zero)

val model4 = LinearMetaModel()
model4.add(x)
model4.add(bter)
model4.addConstraint(x geq Flt64.zero)
model4.maximize(bter)
val result4 = runBlocking { solver(model4) }
assert(result4.value!!.obj eq Flt64.one)

val model5 = LinearMetaModel()
model5.add(x)
model5.add(bter)
model5.addConstraint(x leq 0)
model5.minimize(bter)
val result5 = runBlocking { solver(model5) }
assert(result5.value!!.obj eq -Flt64.one)

val model6 = LinearMetaModel()
model6.add(x)
model6.add(bter)
model6.addConstraint(x leq 0)
model6.maximize(bter)
val result6 = runBlocking { solver(model6) }
assert(result6.value!!.obj eq Flt64.zero)
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

val x = RealVar("x")
x.range.leq(Flt64.two)
x.range.geq(-Flt64.two)
val bter = BalanceTernaryzationFunction(x, name = "bter")
val solver = ScipLinearSolver()

val model1 = LinearMetaModel()
model1.add(x)
model1.add(bter)
model1.minimize(bter)
val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq -Flt64.one)

val model2 = LinearMetaModel()
model2.add(x)
model2.add(bter)
model2.maximize(bter)
val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj eq Flt64.one)

val model3 = LinearMetaModel()
model3.add(x)
model3.add(bter)
model3.addConstraint(x geq Flt64.zero)
model3.minimize(bter)
val result3 = runBlocking { solver(model3) }
assert(result3.value!!.obj eq Flt64.zero)

val model4 = LinearMetaModel()
model4.add(x)
model4.add(bter)
model4.addConstraint(x geq Flt64.zero)
model4.maximize(bter)
val result4 = runBlocking { solver(model4) }
assert(result4.value!!.obj eq Flt64.one)

val model5 = LinearMetaModel()
model5.add(x)
model5.add(bter)
model5.addConstraint(x leq 0)
model5.minimize(bter)
val result5 = runBlocking { solver(model5) }
assert(result5.value!!.obj eq -Flt64.one)

val model6 = LinearMetaModel()
model6.add(x)
model6.add(bter)
model6.addConstraint(x leq 0)
model6.maximize(bter)
val result6 = runBlocking { solver(model6) }
assert(result6.value!!.obj eq Flt64.zero)

val model7 = LinearMetaModel()
model7.add(x)
model7.add(bter)
model7.addConstraint(x leq Flt64(0.3))
model7.maximize(bter)
val result7 = runBlocking { solver(model7) }
assert(result7.value!!.obj eq Flt64.one)

val model8 = LinearMetaModel()
model8.add(x)
model8.add(bter)
model8.addConstraint(x geq -Flt64(0.3))
model8.minimize(bter)
val result8 = runBlocking { solver(model8) }
assert(result8.value!!.obj eq -Flt64.one)
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/BalanceTernaryzation.kt)

完整样例请参考：

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/BTerTest.kt)
