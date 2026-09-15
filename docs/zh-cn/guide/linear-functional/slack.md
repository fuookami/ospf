# 松弛

## 当前 API

### Kotlin

对线性多项式 `x`、`y`，令 `d = x - y`。`SlackFunction<V>` 通过非负辅助变量表示违约量：

```text
withNegative = true,  withPositive = false: max(0, -d)
withNegative = false, withPositive = true:  max(0,  d)
withNegative = true,  withPositive = true:  |d|
```

实现位于 [`Slack.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Slack.kt#L42-L196)。主构造函数为：

```kotlin
SlackFunction(
    x: LinearPolynomial<V>,
    y: LinearPolynomial<V>,
    type: VariableTypeKind = UContinuous,
    withNegative: Boolean = true,
    withPositive: Boolean = true,
    threshold: Boolean = false,
    constraint: Boolean = true,
    converter: IntoValue<V>,
    name: String,
    displayName: String? = null
)
```

同一文件还提供 `LinearIntermediateSymbol<V>` 与 `ToLinearPolynomial<V>` 重载（`Slack.kt:173-280`）。`withNegative` 与 `withPositive` 至少有一个必须为 `true`（`Slack.kt:54-56`）。整数 `type` 创建 `UIntVar` 辅助变量，连续类型创建 `URealVar`（`Slack.kt:98-100`）。两个方向同时启用时，Kotlin 还创建一个二值分支变量（`<name>_side`），因此辅助变量为 `neg`、`pos`、`side`。

### Rust

Rust 将 [`SlackFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/slack.rs) 实现为绝对差：

```rust
SlackFunction::new(
    id: u64,
    name: &str,
    left: Linear<V>,
    right: Linear<V>,
) -> SlackFunction<V>

SlackFunction::with_target(
    id: u64,
    name: &str,
    left: Linear<V>,
    right_value: V,
) -> SlackFunction<V>

SlackFunction::with_big_m(
    id: u64,
    name: &str,
    left: Linear<V>,
    right: Linear<V>,
    big_m: V,
) -> SlackFunction<V>
```

Rust 结果始终是 |left - right|，并通过 `result_variable()` 暴露。它没有 Kotlin 的 `withNegative`/`withPositive`、`threshold`、`constraint` 或变量类型参数；右侧为常数时使用 `with_target`，无界默认值 ($2\times10^6$) 不合适时使用 `with_big_m`。

## 导出符号与求值

辅助表达式为

$$
polyX = x + neg - pos,
\qquad z = neg + pos.
$$

`neg`、`pos` 以可空线性多项式公开，`resultPolynomial` 是所启用辅助变量之和（`Slack.kt:58-90`）。直接求值只使用 `x`、`y`：两类辅助变量返回 `|x-y|`，仅 `neg` 返回 `max(0,y-x)`，仅 `pos` 返回 `max(0,x-y)`（`Slack.kt:102-115`）；任一输入未解析时返回 `null`。

## 求解器数学模型

当 `constraint = true` 时：

- 两个方向同时启用时，无论 `threshold` 为何，Kotlin 都注册以下精确四行绝对值模型：

  $$
  s-d\ge0,\qquad s+d\ge0,\qquad
  s-d+Mu\le M,\qquad s+d-Mu\le0,
  $$

  其中 $s=neg+pos$、$d=x-y$，$u$ 为二值分支变量。这些行无需目标函数即可强制 $s=|d|$。
- 只启用一个辅助变量时，`threshold = false` 注册等式 `x + neg - pos = y`；`threshold = true` 注册对应的单向不等式（`neg` 为 `x + neg >= y`，`pos` 为 `x - pos <= y`）。

当 `constraint = false` 时，不注册输入与辅助变量之间的关系（`Slack.kt:125-128`）。`evaluate` 始终根据输入计算数学违约量，与注册的辅助变量值无关。因此，模型侧的值只有在最小化相应辅助表达式或另加最小性约束时才精确；不最小化时，关系允许出现被放大的松弛量。

Rust 同样创建有符号结果 $s$、二值选择变量 $u$，并令 $d=x-y$，实际传入精确绝对差约束

$$
s-d\ge0,\qquad s+d\ge0,
$$

$$
s-d+Mu\le M,\qquad s+d-Mu\le0.
$$

只要 $M$ 有效，这些约束无需目标函数就能保证 $s=|x-y|$。无界默认值为 $2\times10^6$，因为未激活分支可能暴露两倍最大绝对差；可用 `with_big_m` 指定更紧的界。

## 参考

- 实现：[`Slack.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Slack.kt)
- 完整样例：[`SlackTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/SlackTest.kt)
- Core 测试：[`FunctionSymbolRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolRegressionTest.kt)、[`FunctionSymbolPiecewiseGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolPiecewiseGenericRegistrationTest.kt)

## 示例与测试

::: code-group

```kotlin [Kotlin]
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.model.mechanism.LinearMechanismModel
import fuookami.ospf.kotlin.core.model.mechanism.LinearMetaModel
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.LinearFunctionSymbolAdapter
import fuookami.ospf.kotlin.core.symbol.function.SlackFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.utils.functional.Ok

val x = RealVar("x")
val xPoly = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, x)), Flt64.zero
)
val zeroPoly = LinearPolynomial<Flt64>(emptyList(), Flt64.zero)
val slack = SlackFunction(
    x = xPoly,
    y = zeroPoly,
    converter = IntoValue.Identity,
    name = "slack"
)
val symbol = LinearFunctionSymbolAdapter(slack, IntoValue.Identity)
val model = LinearMetaModel<Flt64>(name = "slack-model", converter = IntoValue.Identity)
check(model.add(x) is Ok)
check(model.add(symbol) is Ok)
check(model.minimize(symbol) is Ok)
val mechanism = runBlocking {
    LinearMechanismModel.invoke<Flt64>(metaModel = model, concurrent = false)
}
check(mechanism is Ok)
model.close()
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::SlackFunction;

let left = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let slack = SlackFunction::with_target(1, "slack", left, 0.0_f64);
let _result = slack.result_variable();
```

:::

`SlackFunction` 是 `MathFunctionSymbol`，本身不是 `LinearIntermediateSymbol`。将结果加入模型目标前，应使用 `LinearFunctionSymbolAdapter` 包装；当前用法见 [`FunctionSymbolRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolRegressionTest.kt#L24-L164) 与 [`MinimizeMaximizeSymbolTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/intermediate_model/MinimizeMaximizeSymbolTest.kt#L88-L110)：

Rust 源码：[`slack.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/slack.rs)。
