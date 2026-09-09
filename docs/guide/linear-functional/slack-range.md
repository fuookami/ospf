# Slack Range

## Current API

### Kotlin

`SlackRangeFunction<V>` receives three linear polynomials `x`, `lb`, and `ub`. Its direct evaluation is the one-sided distance outside the interval:

$$
z_{eval} =
\begin{cases}
lb-x, & x < lb,\\
x-ub, & x > ub,\\
0, & lb \le x \le ub.
\end{cases}
$$

The implementation is [`SlackRange.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/SlackRange.kt#L42-L165). Its primary constructor is:

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

`invoke` has the polynomial overload at `SlackRange.kt:126-136`; `fromLinearIntermediateSymbol` accepts a linear intermediate input and returns a `LinearFunctionSymbolAdapter` (`SlackRange.kt:153-165`). Integer `type` creates `UIntVar` helpers and a continuous type creates `URealVar` helpers (`SlackRange.kt:53-58`). The constructor does not validate `lb <= ub`; callers must provide an ordered interval.

### Rust

Rust exposes [`SlackRangeFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/slack_range.rs):

```rust
SlackRangeFunction::new(
    id: u64,
    name: &str,
    input: Linear<V>,
    lower: V,
    upper: V,
) -> SlackRangeFunction<V>
```

The Rust implementation builds an exact internal `MaxFunction` over `lower - input`, `input - upper`, and zero. Its `result_variable()` therefore represents the total one-sided distance max(lower - input, input - upper, 0). This differs from Kotlin's current `resultPolynomial`, which exposes only the upper-side helper; Rust has no separate public `neg`/`pos` polynomials.

## Derived variables and constraints

The model-side helper expression is

$$
polyX = x + neg - pos,
$$

where `neg` is the lower-side violation and `pos` is the upper-side violation. If `constraint = true`, the implementation registers

$$
polyX \le ub,
\qquad
polyX \ge lb
$$

(`SlackRange.kt:73-79,102-109`). If `constraint = false`, no range constraints are added. The non-negative helper domains allow larger-than-minimal helper values.

There is an important result contract difference: `neg` and `pos` are separately exposed (`SlackRange.kt:66-71`), but `resultPolynomial` contains **only `pos`**, not `neg + pos` (`SlackRange.kt:60-65`). Thus minimizing the function adapter measures upper violation only; it does not penalize lower violation. Use the exposed `neg` and `pos` polynomials explicitly when an objective must minimize total interval violation. `evaluate` still returns the one-sided distance above and is independent of `resultPolynomial` (`SlackRange.kt:81-92`).

## References

- Implementation: [`SlackRange.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/SlackRange.kt)
- Complete example: [`SlackRangeTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/SlackRangeTest.kt)
- Core tests: [`SlackRangeFunctionGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/SlackRangeFunctionGenericEvaluateTest.kt), [`FunctionSymbolPiecewiseGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolPiecewiseGenericRegistrationTest.kt)

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

The generic evaluation test constructs all three inputs as `LinearPolynomial<V>` and supplies a converter (`SlackRangeFunctionGenericEvaluateTest.kt:20-38`). The following is the current package/API pattern; the adapter is needed when the function is stored in a model symbol table or objective:

Rust source: [`slack_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/slack_range.rs) and its inner [`max.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/max.rs).
