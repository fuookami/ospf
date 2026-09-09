# Minimum

`MinFunction` represents the minimum of one or more linear polynomials:

$$
y=\min(p_1,p_2,\ldots,p_n).
$$

## Contract

- Input: a non-empty `List<LinearPolynomial<V>>` (`n >= 1`).
- Output: `resultVar`, a `URealVar`, exposed as `resultPolynomial`.
- `evaluate` evaluates every input and returns their minimum; a missing symbol value makes it return `null`.
- `V` must implement `RealNumber<V>` and `NumberField<V>`; pass the matching `IntoValue<V>` converter.

## Mathematical definition

The implementation uses binary `selectorVars` $s_i$ and the symmetric exact selector model:

$$
y\le p_i\quad(i=1,\ldots,n),
$$

$$
y-p_i-M_i s_i\ge -M_i,
\qquad \sum_{i=1}^{n}s_i=1.
$$

With the selector for one candidate equal to zero, that candidate is forced to equal the result; the remaining inequalities force the result to be no greater than every candidate.

## Domain and boundaries

The solver result is a `URealVar`, so a negative minimum cannot be represented. `evaluate` can still return a negative value. Use this function only when the feasible model guarantees a non-negative minimum, or choose a signed result formulation. As with `MaxFunction`, inferred Big-M values require finite candidate bounds; otherwise the current fallback is $10^6$, and an explicit `bigM` must cover all candidate gaps.

## Current API

### Kotlin

`MinFunction` is declared in the same source file as `MaxFunction` (there is no separate implementation file): [`Max.kt` (`MinFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Max.kt#L182-L281)

```kotlin
MinFunction(
    polynomials: List<LinearPolynomial<V>>,
    bigM: V? = null,
    converter: IntoValue<V>,
    name: String = "min",
    displayName: String? = null
)
```

The companion factory also provides `fromSymbols` for `LinearIntermediateSymbol<V>` candidates.

### Rust

Rust exposes [`MinFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/max.rs) over `flatten::Linear<V>`:

```rust
MinFunction::new(id: u64, name: &str, polynomials: Vec<Linear<V>>, exact: bool) -> MinFunction<V>
```

`exact = true` creates one binary selector per candidate and registers the exact selector model. `exact = false` keeps only the inequality envelope; unlike Kotlin, Rust has no `bigM` or converter argument on this constructor. The result is exposed by `result_variable()`.

## Auxiliary variables and registration

`helperVariables` contains the non-negative `resultVar` and one binary selector for each candidate. Constraint registration adds `result <= p_i`, a Big-M lower/equality gate for each candidate, and `sum(selectorVars) = 1`.

## `evaluate` versus solver

`evaluate` directly folds the candidate values and does not impose the `URealVar` domain. Solver registration does impose that domain and relies on valid Big-M values. Consequently, direct evaluation of all-negative candidates can succeed while the solver model is infeasible.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.MinFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.eq
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

val x = RealVar("x")
val y = RealVar("y")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val yPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, y)), Flt64.zero)
val min = MinFunction(
    polynomials = listOf(xPoly, yPoly),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "min"
)
val value = min.evaluate(mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0)))
check(value != null && (value eq Flt64.two))
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::MinFunction;

let x = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let y = Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0);
let min = MinFunction::new(1, "min", vec![x, y], true);
assert!(min.exact());
let _result = min.result_variable();
```

:::

Complete example: [`MinTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/MinTest.kt)

Core validation: [`MaxAndMaskingFunctionGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/MaxAndMaskingFunctionGenericEvaluateTest.kt)

Rust source and cross-language regression coverage: [`max.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/max.rs) and [`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs).

## MinMaxFunction and MaxMinFunction

$$
MinMax(p_1,\ldots,p_n)=\max_i p_i,\qquad
MaxMin(p_1,\ldots,p_n)=\min_i p_i.
$$

Despite their names, `MinMaxFunction` computes the maximum by delegating every evaluation, helper-variable, and constraint operation to an inner `MaxFunction`. `MaxMinFunction` computes the minimum by delegating to an inner `MinFunction`. The names describe the optimization interpretation, not a different aggregation algorithm. Both wrappers accept the same `polynomials`, optional `bigM`, `converter`, `name`, and optional `displayName` parameters. Their `fromSymbols` factories accept `List<LinearIntermediateSymbol<V>>` and return a `LinearFunctionSymbolAdapter`; the adapter is only a bridge to the intermediate-symbol API.

Source: [`MinMax.kt` (`MinMaxFunction` and `MaxMinFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/MinMax.kt#L40-L196)

```kotlin
val minMax = MinMaxFunction(
    polynomials = listOf(xPoly, yPoly),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "min_max"
)
val maxMin = MaxMinFunction(
    polynomials = listOf(xPoly, yPoly),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "max_min"
)
```

## Related pages

- [`max`](./max): the corresponding maximum operator.
- [`masking`](./masking): binary selection of one polynomial versus zero.
