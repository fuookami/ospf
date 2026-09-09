# Slack in a Quadratic Model

## Availability

Kotlin has no quadratic-specific `SlackFunction` and no overload accepting `QuadraticPolynomial<V>`. Its only implementation is the linear [`SlackFunction`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Slack.kt#L42-L280), whose inputs are `LinearPolynomial<V>` (or a linear intermediate/conversion view). Rust has a direct `QuadraticSlackFunction` counterpart, documented below.

This distinction matters when the surrounding model is quadratic. In Kotlin, `QuadraticMechanismModel` first dispatches quadratic function symbols and then has a fallback for `MathFunctionSymbolBase` (`MechanismModel.kt:1350-1355`). A linear `SlackFunction` can therefore be registered in a quadratic model through `LinearFunctionSymbolAdapter`; this is still a linear slack encoding, not a quadratic one. A quadratic expression such as `x * y` cannot be passed to Kotlin's `SlackFunction`.

## Formula and semantics

For linear expressions `x` and `y`, let `d = x - y`. The linear implementation provides:

```text
both helpers:  |d|      (when the helper result is minimized)
negative:      max(0,-d)
positive:      max(0, d)
```

Its model expression is `polyX = x + neg - pos`; `threshold` and `constraint` control the registered equality/inequality as described in [Linear Slack](/guide/linear-functional/slack). Without minimizing the exposed helper expression, the relation permits inflated slack. `SlackFunction` uses `type` to choose integer (`UIntVar`) or continuous (`URealVar`) helpers and requires a converter.

## Current API

### Kotlin

The adapter is a linear intermediate symbol, so it can be added to `QuadraticMetaModel` and is then registered by the quadratic mechanism fallback:

The adapter is required because a bare `MathFunctionSymbol` is not a model `IntermediateSymbol`. The fallback registers its linear helper constraints; it does not make a quadratic slack operator available in Kotlin.

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

Rust provides [`QuadraticSlackFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs), which bridges two quadratic inputs and applies the absolute-difference slack encoding:

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

It creates quadratic-linear bridges for both inputs, then applies the inner `SlackFunction` to those bridge variables. `calculate_value` returns `abs(left - right)`, `result_variable` exposes the inner `name + "_slack"` variable, and the quadratic mechanism registers the bridge constraints plus the four linear slack constraints. With token bounds, the implementation can infer a tighter Big-M for the inner encoding.

```rust
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticSlackFunction;

let left = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 0.0);
let right = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
let slack = QuadraticSlackFunction::with_big_m(21, "qslack", left, right, 100.0);
assert!(slack.result_variable().name().contains("qslack_slack"));
```

There is no Kotlin/Rust constructor-level parity here: Kotlin exposes only the linear function plus an adapter, while Rust's `QuadraticSlackFunction` accepts `Quadratic<V>` directly.

## Examples and tests

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

- Linear-in-quadratic-model example: [`SlackTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/SlackTest.kt)
- Adapter/fallback validation: [`MechanismModelTokenSynchronizationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/model/mechanism/MechanismModelTokenSynchronizationTest.kt#L254-L301)

- Rust implementation and focused tests: [`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)

## References

- Linear API: [Slack](/guide/linear-functional/slack) and [`Slack.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Slack.kt)
- Mechanism fallback: [`MechanismModel.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/model/mechanism/MechanismModel.kt#L1350-L1355)
- Adapter/fallback test: [`MechanismModelTokenSynchronizationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/model/mechanism/MechanismModelTokenSynchronizationTest.kt#L254-L301)
- Related complete example (linear API; no dedicated quadratic Slack example): [`SlackTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/SlackTest.kt)
