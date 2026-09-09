# 二次模型中的松弛

## 可用性

Kotlin 当前没有专用的二次 `SlackFunction`，也没有接收 `QuadraticPolynomial<V>` 的重载。唯一实现是线性 [`SlackFunction`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Slack.kt#L42-L280)，输入只能是 `LinearPolynomial<V>`（或线性中间符号/转换视图）。Rust 提供直接的 `QuadraticSlackFunction` 对应物，见下文。

这一区别在二次模型中仍然成立。对 Kotlin 而言，`QuadraticMechanismModel` 先分派二次函数符号，然后对 `MathFunctionSymbolBase` 使用回退分支（`MechanismModel.kt:1350-1355`）。因此，线性 `SlackFunction` 可以通过 `LinearFunctionSymbolAdapter` 在二次模型中注册；这仍是线性松弛编码，不是二次松弛编码。不能把 `x * y` 这样的二次表达式传给 Kotlin 的 `SlackFunction`。

## 公式与语义

对线性表达式 `x`、`y`，令 `d = x - y`。线性实现提供：

```text
both helpers:  |d|      (when the helper result is minimized)
negative:      max(0,-d)
positive:      max(0, d)
```

其模型表达式是 `polyX = x + neg - pos`；`threshold` 与 `constraint` 控制注册的等式/不等式，详见[线性松弛](/guide/linear-functional/slack)。不最小化公开的辅助表达式时，关系允许松弛量被放大。`SlackFunction` 用 `type` 选择整数（`UIntVar`）或连续（`URealVar`）辅助变量，并且必须提供转换器。

## 当前 API

### Kotlin

适配器是线性中间符号，因此可以加入 `QuadraticMetaModel`，随后由二次机制的回退分支注册：

之所以需要适配器，是因为裸 `MathFunctionSymbol` 不是模型的 `IntermediateSymbol`。回退分支注册的是线性辅助变量/约束，并不会在 Kotlin 中提供二次松弛算子。

```kotlin
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.model.mechanism.QuadraticMechanismModel
import fuookami.ospf.kotlin.core.model.mechanism.QuadraticMetaModel
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.LinearFunctionSymbolAdapter
import fuookami.ospf.kotlin.core.symbol.function.SlackFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.utils.functional.Ok

val x = RealVar("x")
val xPoly = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, x)), Flt64.zero
)
val slack = SlackFunction(
    x = xPoly,
    y = LinearPolynomial<Flt64>(emptyList(), Flt64.zero),
    converter = IntoValue.Identity,
    name = "quadratic-model-slack"
)
val symbol = LinearFunctionSymbolAdapter(slack, IntoValue.Identity)
val model = QuadraticMetaModel<Flt64>(
    name = "quadratic-model-with-linear-slack",
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

Rust 提供 [`QuadraticSlackFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)，它桥接两个二次输入并应用绝对差松弛编码：

```rust
QuadraticSlackFunction::new(
    id: u64,
    name: &str,
    left: Quadratic<V>,
    right: Quadratic<V>,
) -> QuadraticSlackFunction<V>

QuadraticSlackFunction::with_target(
    id: u64,
    name: &str,
    left: Quadratic<V>,
    right_value: V,
) -> QuadraticSlackFunction<V>

QuadraticSlackFunction::with_big_m(
    id: u64,
    name: &str,
    left: Quadratic<V>,
    right: Quadratic<V>,
    big_m: V,
) -> QuadraticSlackFunction<V>
```

它为两个输入创建二次线性桥接，再把内部 `SlackFunction` 应用于桥接变量。`calculate_value` 返回 `abs(left - right)`，`result_variable` 暴露内部名称为 `name + "_slack"` 的变量，二次机理注册桥接约束和四条线性松弛约束。有 token 边界时，实现可以为内部编码推导更紧的 Big-M。

```rust
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticSlackFunction;

let left = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 0.0);
let right = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
let slack = QuadraticSlackFunction::with_big_m(21, "qslack", left, right, 100.0);
assert!(slack.result_variable().name().contains("qslack_slack"));
```

这里不存在构造器级 Kotlin/Rust 一一对应：Kotlin 只有线性函数加适配器，Rust 的 `QuadraticSlackFunction` 直接接收 `Quadratic<V>`。

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
import fuookami.ospf.kotlin.core.symbol.function.SlackFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.utils.functional.Ok

val x = RealVar("x")
val xPoly = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, x)), Flt64.zero
)
val slack = SlackFunction(
    x = xPoly,
    y = LinearPolynomial<Flt64>(emptyList(), Flt64.zero),
    converter = IntoValue.Identity,
    name = "quadratic-model-slack"
)
val symbol = LinearFunctionSymbolAdapter(slack, IntoValue.Identity)
val model = QuadraticMetaModel<Flt64>(
    name = "quadratic-model-with-linear-slack",
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
use ospf_rust_core::symbol::function::QuadraticSlackFunction;
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
let slack = QuadraticSlackFunction::new(
    22,
    "qslack",
    Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 0.0),
    Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0),
);
assert_eq!(slack.calculate_value(&tokens, false), Some(1.0));
```

:::

- 二次模型中的线性样例：[`SlackTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/SlackTest.kt)
- 适配器/回退验证：[`MechanismModelTokenSynchronizationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/model/mechanism/MechanismModelTokenSynchronizationTest.kt#L254-L301)

- Rust 实现与针对性测试：[`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)

## 参考

- 线性 API：[松弛](/guide/linear-functional/slack) 与 [`Slack.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Slack.kt)
- 机制回退：[`MechanismModel.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/model/mechanism/MechanismModel.kt#L1350-L1355)
- 适配器/回退测试：[`MechanismModelTokenSynchronizationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/model/mechanism/MechanismModelTokenSynchronizationTest.kt#L254-L301)
- 相关完整样例（线性 API；没有专用二次 Slack 样例）：[`SlackTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/SlackTest.kt)
