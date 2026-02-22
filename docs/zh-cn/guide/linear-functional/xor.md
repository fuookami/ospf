# 逻辑异或

## 形式

$$
y = Xor(x_{1}, \, x_{2}, \, .. \, , \, x_{i}) = \begin{cases}
1, & \neg \bigvee_{i} x_{i} \wedge \neg \bigwedge_{i} x_{i} \\ \; \\
0, & \bigvee_{i} x_{i} \vee \bigwedge_{i} x_{i}
\end{cases}
$$

## 两个多项式

### 额外变量

$y^{\prime} \in \{0, 1 \}$ ：逻辑异或值。

### 导出符号

$$
y = y^{\prime}
$$

### 数学模型

$$
\begin{align}
\text{s.t.} \quad & y^{\prime} & \geq & \, Bin(x_{i}) - \sum_{i^{\prime} \in \{ i^{\prime} \in P | i^{\prime} \neq i \}} Bin(x_{i^{\prime}}), & \; \forall i \in P \\ \; \\
& y & \leq & \, \sum_{i \in P} Bin(x_{i}) \\ \; \\
& y & \leq & \, |P| - \sum_{i \in P} Bin(x_{i})
& 
\end{align}
$$

$Bin(x)$ 可参考 [二值化](/zh-cn/guide/linear-functional/bin)。

## 任意个多项式

### 导出符号

$$
y = Xor(\min(x_{i}), \max(x_{i}))
$$

$\min(x)$ 可参考[最小值](/zh-cn/guide/linear-functional/min)，\max(x)$ 可参考[最大值](/zh-cn/guide/linear-functional/max)。

## 样例

### 两个多项式

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
val y = BinVar("y")
val xor = XorFunction(listOf(x, y), name = "xor")
val solver = ScipLinearSolver()

val model1 = LinearMetaModel()
model1.add(x)
model1.add(y)
model1.add(xor)
model1.addConstraint(xor)
model1.minimize(x + y)
val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq Flt64.one)

val model2 = LinearMetaModel()
model2.add(x)
model2.add(y)
model2.add(xor)
model2.addConstraint(xor)
model2.maximize(x + y)
val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj eq Flt64.one)

val model3 = LinearMetaModel()
model3.add(x)
model3.add(y)
model3.add(xor)
model3.addConstraint(!xor)
model3.minimize(x + y)
val result3 = runBlocking { solver(model3) }
assert(result3.value!!.obj eq Flt64.zero)

val model4 = LinearMetaModel()
model4.add(x)
model4.add(y)
model4.add(xor)
model4.addConstraint(!xor)
model4.maximize(x + y)
val result4 = runBlocking { solver(model4) }
assert(result4.value!!.obj eq Flt64.two)
```

:::

### 任意个多项式

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
val y = BinVar("y")
val z = BinVar("z")
val xor = XorFunction(listOf(x, y, z), name = "xor")
val solver = ScipLinearSolver()

val model1 = LinearMetaModel()
model1.add(x)
model1.add(y)
model1.add(z)
model1.add(xor)
model1.addConstraint(xor)
model1.minimize(x + y + z)
val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq Flt64.one)

val model2 = LinearMetaModel()
model2.add(x)
model2.add(y)
model2.add(z)
model2.add(xor)
model2.addConstraint(xor)
model2.maximize(x + y + z)
val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj eq Flt64.two)

val model3 = LinearMetaModel()
model3.add(x)
model3.add(y)
model3.add(z)
model3.add(xor)
model3.addConstraint(!xor)
model3.minimize(x + y + z)
val result3 = runBlocking { solver(model3) }
assert(result3.value!!.obj eq Flt64.zero)

val model4 = LinearMetaModel()
model4.add(x)
model4.add(y)
model4.add(z)
model4.add(xor)
model4.addConstraint(!xor)
model4.maximize(x + y + z)
val result4 = runBlocking { solver(model4) }
assert(result4.value!!.obj eq Flt64.three)
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/Xor.kt)

完整样例请参考：

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/XorTest.kt)
