# 松弛范围

## 当前 API

### Kotlin

`SlackRangeFunction<V>` 接收线性多项式 `x`、`lb`、`ub`。直接求值是区间外的单侧距离：

$$
z_{eval} =
\begin{cases}
lb-x, & x < lb,\\
x-ub, & x > ub,\\
0, & lb \le x \le ub.
\end{cases}
$$

实现位于 [`SlackRange.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/SlackRange.kt#L42-L165)。主构造函数为：

```kotlin
SlackRangeFunction(
    x: LinearPolynomial<V>,
    lb: LinearPolynomial<V>,
    ub: LinearPolynomial<V>,
    type: VariableTypeKind = UContinuous,
    constraint: Boolean = true,
    converter: IntoValue<V>,
    name: String,
    displayName: String? = null
)
```

`invoke` 的多项式重载在 `SlackRange.kt:126-136`；`fromLinearIntermediateSymbol` 接收线性中间符号并返回 `LinearFunctionSymbolAdapter`（`SlackRange.kt:153-165`）。整数 `type` 创建 `UIntVar` 辅助变量，连续类型创建 `URealVar`（`SlackRange.kt:53-58`）。构造函数不检查 `lb <= ub`；调用方必须提供有序区间。

### Rust

Rust 暴露 [`SlackRangeFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/slack_range.rs)：

```rust
SlackRangeFunction::new(
    id: u64,
    name: &str,
    input: Linear<V>,
    lower: V,
    upper: V,
) -> SlackRangeFunction<V>
```

Rust 实现对 `lower - input`、`input - upper` 和零构造一个精确的内部 `MaxFunction`。因此其 `result_variable()` 表示总的单侧距离 max(lower - input, input - upper, 0)。这与 Kotlin 当前只暴露上侧辅助量的 `resultPolynomial` 不同；Rust 没有单独公开的 `neg`/`pos` 多项式。

## 导出变量与约束

模型侧辅助表达式为

$$
polyX = x + neg - pos,
$$

其中 `neg` 表示下侧违约，`pos` 表示上侧违约。当 `constraint = true` 时，实现注册

$$
polyX \le ub,
\qquad
polyX \ge lb
$$

（`SlackRange.kt:73-79,102-109`）。当 `constraint = false` 时不添加区间约束。辅助变量的非负类型允许出现大于最小值的变量值。

这里有一个重要的结果契约差异：`neg`、`pos` 分别公开（`SlackRange.kt:66-71`），但 `resultPolynomial` **只有 `pos`**，不是 `neg + pos`（`SlackRange.kt:60-65`）。因此，最小化函数适配器只衡量上侧违约，不会惩罚下侧违约。若目标要最小化总区间违约，应显式使用公开的 `neg` 与 `pos` 多项式。`evaluate` 仍返回上述单侧距离，且独立于 `resultPolynomial`（`SlackRange.kt:81-92`）。

## 参考

- 实现：[`SlackRange.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/SlackRange.kt)
- 完整样例：[`SlackRangeTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/SlackRangeTest.kt)
- Core 测试：[`SlackRangeFunctionGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/SlackRangeFunctionGenericEvaluateTest.kt)、[`FunctionSymbolPiecewiseGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolPiecewiseGenericRegistrationTest.kt)

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
import fuookami.ospf.kotlin.core.symbol.function.SlackRangeFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.utils.functional.Ok

val x = RealVar("x")
val xPoly = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, x)), Flt64.zero
)
val lbPoly = LinearPolynomial<Flt64>(emptyList(), Flt64(-2.0))
val ubPoly = LinearPolynomial<Flt64>(emptyList(), Flt64.two)
val slackRange = SlackRangeFunction(
    x = xPoly,
    lb = lbPoly,
    ub = ubPoly,
    converter = IntoValue.Identity,
    name = "slack-range"
)
val symbol = LinearFunctionSymbolAdapter(slackRange, IntoValue.Identity)
val model = LinearMetaModel<Flt64>(name = "slack-range-model", converter = IntoValue.Identity)
check(model.add(x) is Ok)
check(model.add(symbol) is Ok)
// resultPolynomial is pos only; use neg + pos explicitly for total violation.
check(model.minimize(symbol) is Ok)
val mechanism = runBlocking {
    LinearMechanismModel.invoke<Flt64>(metaModel = model, concurrent = false)
}
check(mechanism is Ok)
model.close()
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::SlackRangeFunction;

let input = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let slack_range = SlackRangeFunction::new(1, "slack_range", input, -2.0_f64, 2.0_f64);
assert_eq!(slack_range.lower_bound(), &-2.0);
assert_eq!(slack_range.upper_bound(), &2.0);
let _result = slack_range.result_variable();
```

:::

泛型求值测试将三个输入都构造成 `LinearPolynomial<V>` 并提供转换器（`SlackRangeFunctionGenericEvaluateTest.kt:20-38`）。下面是当前包名/API 的用法；当函数要放入模型符号表或目标时，需要适配器：

Rust 源码：[`slack_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/slack_range.rs) 及其内部 [`max.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/max.rs)。
