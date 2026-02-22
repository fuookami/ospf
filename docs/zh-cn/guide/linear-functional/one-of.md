# 选择

## 形式

$$
y = OneOf((x_{1}, \, c_{1}), \, (x_{2}, \, c_{2}), \, .. \, , \, (x_{i}, \, c_{i})) = \begin{cases}
x_{1}, & c_{1} \\ \; \\
x_{2}, & c_{2} \\ \; \\
\vdots \\ \; \\
x_{i}, & c_{i}
\end{cases}
$$

其中，$c_{i}$ 应当有且仅有一个成立。

## 导出符号

$$
y = \sum_{i} masking(x_{i}, \, c_{i})
$$

$masking(x_{i}, \, c_{i})$ 可参考 [掩码函数](/zh-cn/guide/linear-functional/masking)。

## 数学模型

$$
\text{s.t.} \quad \sum_{i} c_{i} = 1
$$

## 样例

### 选择

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
x.range.geq(Flt64.two)
x.range.leq(Flt64.five)
val condition1 = IfFunction(x geq Flt64.three, name = "c1")
val condition2 = IfFunction(x leq Flt64.one, name = "c2")
val oneOf = OneOfFunction(
    listOf(
        AbstractOneOfFunction.Branch(
            condition1,
            LinearPolynomial(Flt64.zero),
            "ifc1"
        ),
        AbstractOneOfFunction.Branch(
            condition2,
            LinearPolynomial(Flt64.one),
            "ifc2"
        )
    ),
    name = "one_of"
)
val solver = ScipLinearSolver()

val model1 = LinearMetaModel()
model1.add(x)
model1.add(condition1)
model1.add(condition2)
model1.add(oneOf)
model1.minimize(x)
val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq Flt64.three)

val model2 = LinearMetaModel()
model2.add(x)
model2.add(condition1)
model2.add(condition2)
model2.add(oneOf)
model2.maximize(oneOf)
val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj eq Flt64.zero)
```

:::

### 二择

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
x.range.geq(Flt64.two)
x.range.leq(Flt64.five)
val condition = IfFunction(x geq Flt64.three, name = "c")
val ifElse = IfElseFunction(
    IfElseFunction.Branch(
        polynomial = x,
        name = "if"
    ),
    IfElseFunction.Branch(
        polynomial = LinearPolynomial(Flt64.zero),
        name = "else"
    ),
    condition,
    name = "if_else"
)
val solver = ScipLinearSolver()

val model1 = LinearMetaModel()
model1.add(x)
model1.add(condition)
model1.add(ifElse)
model1.addConstraint(x geq Flt64.three)
model1.maximize(ifElse)
val result1 = runBlocking { solver(model1) }
assert(result1.value!!.obj eq Flt64.five)

val model2 = LinearMetaModel()
model2.add(x)
model2.add(condition)
model2.add(ifElse)
model2.minimize(ifElse)
val result2 = runBlocking { solver(model2) }
assert(result2.value!!.obj eq Flt64.zero)
assert(result2.value!!.solution[0] ls Flt64.three)

val model3 = LinearMetaModel()
model3.add(x)
model3.add(condition)
model3.add(ifElse)
model3.addConstraint(x eq Flt64.two)
model3.maximize(ifElse)
val result3 = runBlocking { solver(model3) }
assert(result3.value!!.obj eq Flt64.zero)
```

:::

完整实现请参考：

- [Kotlin](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/frontend/expression/symbol/linear_function/OneOf.kt)

完整样例请参考：

- [Kotlin](https://github.com/fuookami/ospf/tree/main/examples/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/OneOfTest.kt)
