# 逻辑非

$$
y = Not(x) = \begin{cases}
1, & \neg x \\ \; \\
0, & x
\end{cases}
$$

## 离散

### 常量

$$
m = max(x)
$$

### 额外变量

$y^{\prime} \in \{ 0, \, 1 \}$ ：多项式非的逻辑值。

### 导出符号

$$
y = y^{\prime}
$$

### 数学模型

$$
\begin{align}
\text{s.t.} \quad & m \cdot (1 - y^{\prime}) & \geq & \, x \\ \; \\
& 1 - y^{\prime} & \leq & \, x
\end{align}
$$

即：

$$
y = \begin{cases}
1, & x = 0 \\ \; \\
0, & x \geq 1
\end{cases}
$$

## 连续（非精确）

### 常量

$$
m = max(x)
$$

### 额外变量

$b \in [0, \, 1]$ ：多项式的归一化值。

$y^{\prime} \in \{ 0, \, 1 \}$ ：多项式非的逻辑值。

### 导出符号

$$
y = y^{\prime}
$$

### 数学模型

$$
\begin{align}
\text{s.t.} \quad & x & = & \, m \cdot b \\ \; \\
& 1 - y^{\prime} & \geq & \, b \\ \; \\
& \epsilon \cdot (1 - y^{\prime}) & \leq & b
\end{align}
$$

即：

$$
y = \begin{cases}
1, & x = 0 \\ \; \\
0, & x \geq \epsilon
\end{cases}
$$

## 连续（精确）

### 导出符号

$$
y = Ulp(x)
$$

$Ulp(x)$ 可参考 [一元分段线性函数](/zh-cn/guide/linear-functional/ulp)，使用的采样点为 $\{ (0, \, 1), \; (p - \epsilon, \, 1), \; (p, \, 0), \; (max(x), \, 0) \}$。

即：

$$
y = \begin{cases}
1, & 0 \leq x < p \\ \; \\
0, & x \geq p
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

val x = BinVar("x")
val not = NotFunction(x, name = "not")
val solver = ScipLinearSolver()

val model1 = LinearMetaModel()
model1.add(x)
model1.add(not)
model1.addConstraint(not)
model1.maximize(x)
val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq Flt64.zero)

val model2 = LinearMetaModel()
model2.add(x)
model2.add(not)
model2.addConstraint(!not)
model2.minimize(x)
val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj eq Flt64.one)
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
x.range.leq(Flt64.one)
val not = NotFunction(x, name = "not")
val solver = ScipLinearSolver()

val model1 = LinearMetaModel()
model1.add(x)
model1.add(not)
model1.addConstraint(not)
model1.maximize(x)
val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq Flt64.zero)

val model2 = LinearMetaModel()
model2.add(x)
model2.add(not)
model2.addConstraint(!not)
model2.minimize(x)
val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj geq Flt64(1e-6))
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/Not.kt)

完整样例请参考：

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/NotTest.kt)
