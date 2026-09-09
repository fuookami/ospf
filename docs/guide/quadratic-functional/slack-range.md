# Slack Range in a Quadratic Model

## Availability

Kotlin has no quadratic-specific `SlackRangeFunction` and no overload accepting `QuadraticPolynomial<V>`. Its available implementation is the linear [`SlackRangeFunction`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/SlackRange.kt#L42-L165), which accepts `LinearPolynomial<V>` bounds (or a linear intermediate input through its adapter overload). Rust has a direct `QuadraticSlackRangeFunction` counterpart, documented below.

In Kotlin, `QuadraticMechanismModel` has a fallback dispatch for `MathFunctionSymbolBase` after its quadratic-symbol branch (`MechanismModel.kt:1350-1355`). Consequently, a linear `SlackRangeFunction` can be carried by a `LinearFunctionSymbolAdapter` in a quadratic model. This fallback does not promote a quadratic expression: `QuadraticPolynomial<V>` cannot be passed to Kotlin's `SlackRangeFunction`.

## Formula and implementation contract

For an ordered interval `lb <= ub`, direct evaluation is

$$
z_{eval} =
\begin{cases}
lb-x, & x < lb,\\
x-ub, & x > ub,\\
0, & lb \le x \le ub.
\end{cases}
$$

The linear model uses non-negative helpers and

$$
polyX = x + neg - pos,
\qquad polyX \le ub,
\qquad polyX \ge lb.
$$

`constraint = false` skips the two inequalities. `type` selects `UIntVar` or `URealVar` helpers. The class does not validate `lb <= ub`, so the caller must do so.

The result contract is not the same as the evaluation formula: `neg` and `pos` are separately exposed, but `resultPolynomial` is **only `pos`**. Therefore, minimizing the adapted function measures upper violation only; it does not minimize lower violation. Use `neg + pos` explicitly when total interval violation is required. See [Linear Slack Range](/guide/linear-functional/slack-range) for the full linear API and this distinction.

## Current API

### Kotlin

Wrap the linear function before adding it to a quadratic model. The Kotlin mechanism fallback then registers its linear helpers and constraints:

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

Rust provides [`QuadraticSlackRangeFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs):

```rust
QuadraticSlackRangeFunction::new(
    id: u64,
    name: &str,
    input: Quadratic<V>,
    lower: V,
    upper: V,
) -> QuadraticSlackRangeFunction<V>
```

The quadratic input is bridged to a linear variable and passed to the inner `SlackRangeFunction`. Direct evaluation returns the distance to the closed interval `[lower, upper]`: `lower - input` below the interval, `input - upper` above it, and zero inside. `result_variable` exposes the inner non-negative slack result. The mechanism registers the quadratic bridge plus the linear range-slack constraints; it does not turn Kotlin's adapter-based linear API into a quadratic one.

```rust
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticSlackRangeFunction;

let input = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 0.0);
let slack_range = QuadraticSlackRangeFunction::new(23, "qslack_range", input, 1.0, 2.0);
assert!(slack_range.result_variable().name().contains("qslack_range_max"));
```

Kotlin and Rust therefore have different constructor surfaces: Kotlin requires the linear `SlackRangeFunction` plus `LinearFunctionSymbolAdapter`, while Rust accepts `Quadratic<V>` directly.

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

- Linear-in-quadratic-model example: [`SlackRangeTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/SlackRangeTest.kt)
- Adapter/fallback validation: [`MechanismModelTokenSynchronizationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/model/mechanism/MechanismModelTokenSynchronizationTest.kt#L254-L301)

- Rust implementation and focused tests: [`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)

## References

- Linear API: [Slack Range](/guide/linear-functional/slack-range) and [`SlackRange.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/SlackRange.kt)
- Mechanism fallback: [`MechanismModel.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/model/mechanism/MechanismModel.kt#L1350-L1355)
- Adapter/fallback test: [`MechanismModelTokenSynchronizationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/model/mechanism/MechanismModelTokenSynchronizationTest.kt#L254-L301)
- Related complete example (linear API; no dedicated quadratic Slack Range example): [`SlackRangeTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/SlackRangeTest.kt)
