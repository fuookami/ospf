# 二次模型中的松弛范围

## 可用性

Kotlin 当前没有专用的二次 `SlackRangeFunction`，也没有接收 `QuadraticPolynomial<V>` 的重载。现有实现是线性 [`SlackRangeFunction`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/SlackRange.kt#L42-L165)，它接收 `LinearPolynomial<V>` 形式的表达式、下界和上界（或通过适配器重载接收线性中间输入）。Rust 提供直接的 `QuadraticSlackRangeFunction` 对应物，见下文。

对 Kotlin 而言，`QuadraticMechanismModel` 在二次函数符号分支之后，对 `MathFunctionSymbolBase` 使用回退分支（`MechanismModel.kt:1350-1355`）。因此，线性 `SlackRangeFunction` 可以通过 `LinearFunctionSymbolAdapter` 携带到二次模型中。这个回退不会把表达式提升为二次表达式：`QuadraticPolynomial<V>` 不能传给 Kotlin 的 `SlackRangeFunction`。

## 公式与实现契约

对有序区间 `lb <= ub`，直接求值为

$$
z_{eval} =
\begin{cases}
lb-x, & x < lb,\\
x-ub, & x > ub,\\
0, & lb \le x \le ub.
\end{cases}
$$

线性模型使用非负辅助变量，并注册

$$
polyX = x + neg - pos,
\qquad polyX \le ub,
\qquad polyX \ge lb.
$$

`constraint = false` 会跳过这两个不等式；`type` 选择 `UIntVar` 或 `URealVar` 辅助变量。类不会校验 `lb <= ub`，因此调用方必须保证区间有序。

返回结果的契约与求值公式不同：`neg` 和 `pos` 分别公开，但 `resultPolynomial` **只有 `pos`**。所以最小化适配后的函数只衡量上界违约，不会最小化下界违约；需要区间外总违约时必须显式使用 `neg + pos`。完整线性 API 和这一差异见[线性松弛范围](/guide/linear-functional/slack-range)。

## 当前 API

### Kotlin

在加入二次模型前包装线性函数；随后由 Kotlin 二次机制的回退分支注册其线性辅助变量和约束：

```kotlin
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.model.mechanism.QuadraticMechanismModel
import fuookami.ospf.kotlin.core.model.mechanism.QuadraticMetaModel
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
    name = "quadratic-model-slack-range"
)
val symbol = LinearFunctionSymbolAdapter(slackRange, IntoValue.Identity)
val model = QuadraticMetaModel<Flt64>(
    name = "quadratic-model-with-linear-slack-range",
    converter = IntoValue.Identity
)
check(model.add(x) is Ok)
check(model.add(symbol) is Ok)
val mechanism = runBlocking {
    QuadraticMechanismModel.invoke<Flt64>(metaModel = model, concurrent = false)
}
check(mechanism is Ok)
model.close()
```

### Rust

Rust 提供 [`QuadraticSlackRangeFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)：

```rust
QuadraticSlackRangeFunction::new(
    id: u64,
    name: &str,
    input: Quadratic<V>,
    lower: V,
    upper: V,
) -> QuadraticSlackRangeFunction<V>
```

二次输入先桥接到线性变量，再传给内部 `SlackRangeFunction`。直接求值返回到闭区间 `[lower, upper]` 的距离：低于区间时为 `lower - input`，高于区间时为 `input - upper`，区间内为 zero。`result_variable` 暴露内部非负松弛结果。机理注册二次桥接和线性范围松弛约束；不会把 Kotlin 的适配器线性 API 变成二次 API。

```rust
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticSlackRangeFunction;

let input = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 0.0);
let slack_range = QuadraticSlackRangeFunction::new(23, "qslack_range", input, 1.0, 2.0);
assert!(slack_range.result_variable().name().contains("qslack_range_max"));
```

因此 Kotlin 与 Rust 的构造器接口不同：Kotlin 需要线性 `SlackRangeFunction` 加 `LinearFunctionSymbolAdapter`，Rust 直接接收 `Quadratic<V>`。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.model.mechanism.QuadraticMechanismModel
import fuookami.ospf.kotlin.core.model.mechanism.QuadraticMetaModel
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
    name = "quadratic-model-slack-range"
)
val symbol = LinearFunctionSymbolAdapter(slackRange, IntoValue.Identity)
val model = QuadraticMetaModel<Flt64>(
    name = "quadratic-model-with-linear-slack-range",
    converter = IntoValue.Identity
)
check(model.add(x) is Ok)
check(model.add(symbol) is Ok)
val mechanism = runBlocking {
    QuadraticMechanismModel.invoke<Flt64>(metaModel = model, concurrent = false)
}
check(mechanism is Ok)
model.close()
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticSlackRangeFunction;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
let y = ContinuousVariableItem::create(VariableId::standalone(1), "y");
let mut tokens = VecTokenList::<f64>::new();
let tx = Token::from_generic(x, 0);
tx.set_result(2.0);
tokens.add_token(tx);
let ty = Token::from_generic(y, 1);
ty.set_result(1.5);
tokens.add_token(ty);
let slack_range = QuadraticSlackRangeFunction::new(
    24,
    "qslack_range",
    Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 0.0),
    1.0,
    2.0,
);
assert_eq!(slack_range.calculate_value(&tokens, false), Some(1.0));
```

:::

- 二次模型中的线性样例：[`SlackRangeTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/SlackRangeTest.kt)
- 适配器/回退验证：[`MechanismModelTokenSynchronizationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/model/mechanism/MechanismModelTokenSynchronizationTest.kt#L254-L301)

- Rust 实现与针对性测试：[`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)

## 参考

- 线性 API：[松弛范围](/guide/linear-functional/slack-range) 与 [`SlackRange.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/SlackRange.kt)
- 机制回退：[`MechanismModel.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/model/mechanism/MechanismModel.kt#L1350-L1355)
- 适配器/回退测试：[`MechanismModelTokenSynchronizationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/model/mechanism/MechanismModelTokenSynchronizationTest.kt#L254-L301)
- 相关完整样例（线性 API；没有专用二次 Slack Range 样例）：[`SlackRangeTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/SlackRangeTest.kt)
