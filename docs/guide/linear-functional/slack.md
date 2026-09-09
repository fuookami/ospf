# Slack

## Current API

### Kotlin

For two linear polynomials `x` and `y`, let `d = x - y`. `SlackFunction<V>` exposes non-negative violation helpers:

```text
withNegative = true,  withPositive = false: max(0, -d)
withNegative = false, withPositive = true:  max(0,  d)
withNegative = true,  withPositive = true:  |d|   (when the result is minimized)
```

The implementation is [`Slack.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Slack.kt#L42-L196). Its primary constructor is:

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

Overloads accept `LinearIntermediateSymbol<V>` and `ToLinearPolynomial<V>` (`Slack.kt:173-280`). At least one of `withNegative` and `withPositive` must be true (`Slack.kt:54-56`). Integer `type` creates `UIntVar` helpers; a continuous type creates `URealVar` helpers (`Slack.kt:98-100`).

### Rust

Rust exposes [`SlackFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/slack.rs) as an absolute difference:

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

The Rust result is always |left - right| and exposes `result_variable()`. It has no Kotlin `withNegative`/`withPositive`, `threshold`, `constraint`, or variable-type parameters; use `with_target` for a constant right side and `with_big_m` when the default (10^6) is not suitable.

## Derived symbol and evaluation

The helper expression is

$$
polyX = x + neg - pos,
\qquad z = neg + pos.
$$

`neg` and `pos` are exposed as nullable linear polynomials, and `resultPolynomial` is the sum of whichever helpers were requested (`Slack.kt:58-90`). Direct evaluation uses only `x` and `y`: both helpers give `|x-y|`, only `neg` gives `max(0,y-x)`, and only `pos` gives `max(0,x-y)` (`Slack.kt:102-115`). It returns `null` when either input is unresolved.

## Constraint modes and non-minimized models

With `constraint = true`:

- `threshold = false` registers `x + neg - pos = y` (`Slack.kt:125-134`). This determines a difference but does not prevent the non-negative helpers from being larger than the minimum.
- `threshold = true` registers `x + neg >= y` when `withNegative` is enabled; otherwise it registers `x - pos <= y` when `withPositive` is enabled (`Slack.kt:135-141`). When both flags are true, the negative branch wins the implementation's `if/else` order; both threshold inequalities are not registered.

With `constraint = false`, no relation between the inputs and helpers is registered (`Slack.kt:125-128`). `evaluate` always computes the mathematical violation from the inputs, independently of registered helper values. The model-side value is therefore exact only when the relevant helper expression is minimized or otherwise constrained to its minimum; without minimization, the relation permits inflated slack.

## References

- Implementation: [`Slack.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Slack.kt)
- Complete example: [`SlackTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/SlackTest.kt)
- Core tests: [`FunctionSymbolRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolRegressionTest.kt), [`FunctionSymbolPiecewiseGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolPiecewiseGenericRegistrationTest.kt)

## Examples and tests

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

`SlackFunction` is a `MathFunctionSymbol`, not itself a `LinearIntermediateSymbol`. Wrap it with `LinearFunctionSymbolAdapter` before adding its result to a model objective, as in [`FunctionSymbolRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolRegressionTest.kt#L24-L164) and [`MinimizeMaximizeSymbolTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/intermediate_model/MinimizeMaximizeSymbolTest.kt#L88-L110):

Rust source: [`slack.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/slack.rs).
