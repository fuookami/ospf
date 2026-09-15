# Maximum

`MaxFunction` represents the maximum of one or more linear polynomials:

$$
y=\max(p_1,p_2,\ldots,p_n).
$$

## Contract

- Input: a non-empty `List<LinearPolynomial<V>>` (`n >= 1`).
- Output: `resultVar`, a `URealVar`, exposed as `resultPolynomial`.
- `evaluate` evaluates every input and returns their maximum; a missing symbol value makes it return `null`.
- `V` must implement `RealNumber<V>` and `NumberField<V>`; pass the matching `IntoValue<V>` converter.

## Mathematical definition

The exact selector model uses binary `selectorVars` $s_i$:

$$
y\ge p_i\quad(i=1,\ldots,n),
$$

$$
y-p_i+M_i s_i\le M_i,
\qquad \sum_{i=1}^{n}s_i=1.
$$

With the selector for one candidate equal to one, that candidate is forced to equal the result; the remaining inequalities force the result to be at least every candidate.

## Domain and boundaries

The current result variable is `URealVar`. Therefore a solver model cannot represent a negative maximum, even though `evaluate` can return a negative value. Ensure at least one candidate is known non-negative over the feasible domain, or use a different formulation if negative results are required. With no explicit `bigM`, each candidate's finite bounds are used when available; otherwise the fallback Big-M is currently $10^6$. An explicit value must be large enough for every candidate gap.

## Current API

### Kotlin

Source: [`Max.kt` (`MaxFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Max.kt#L43-L142)

```kotlin
MaxFunction(
    polynomials: List<LinearPolynomial<V>>,
    bigM: V? = null,
    converter: IntoValue<V>,
    name: String = "max",
    displayName: String? = null
)
```

The companion factory also provides `fromSymbols` for a list of `LinearIntermediateSymbol<V>`. The ordinary constructor is the clearest choice when the candidates are already `LinearPolynomial<V>` values.

### Rust

Rust exposes [`MaxFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/max.rs) over flattened `Linear<V>` expressions:

```rust
MaxFunction::new(
    id: u64,
    name: &str,
    polynomials: Vec<Linear<V>>,
    exact: bool,
) -> MaxFunction<V>
```

`exact = true` creates one binary selector per candidate and registers an exactly-one selector model. With `exact = false`, Rust registers only the lower bounds `result >= p_i`; an objective or another upper bound is then needed to make the result equal the maximum. `result_variable()`, `polynomials()`, and `exact()` expose the state. Unlike Kotlin's `URealVar` result, Rust's result is a continuous variable; callers must provide suitable bounds when the model requires them. Rust's [`MinMaxFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/min_max.rs) and [`MaxMinFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/min_max.rs) are the corresponding wrapper symbols without the `exact` flag.

## Solver mathematical model

With result $y$, candidates $p_i$, and selectors $s_i\in\{0,1\}$, Kotlin and Rust `exact = true` pass

$$
y-p_i\ge0\quad(1\le i\le n),
$$

$$
y-p_i+M_i s_i\le M_i\quad(1\le i\le n),
\qquad
\sum_{i=1}^{n}s_i=1.
$$

Rust `exact = false` registers only $y-p_i\ge0$; equality with the maximum then depends on minimization or another upper bound. Kotlin always registers the exact selector form.

## `evaluate` versus solver

`evaluate` is a direct fold over all candidate values and has no Big-M or variable-domain side effects. Solver registration adds the selector model and the non-negative result domain; an undersized Big-M or a negative true maximum can make the solver model infeasible despite a valid direct evaluation.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.MaxFunction
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
val max = MaxFunction(
    polynomials = listOf(xPoly, yPoly),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "max"
)
val value = max.evaluate(mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0)))
check(value != null && (value eq Flt64(5.0)))
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::MaxFunction;

let first = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let second = Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0);
let max = MaxFunction::new(1, "max", vec![first, second], true);
assert!(max.exact());
let _result = max.result_variable();
```

:::

Complete example: [`MaxTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/MaxTest.kt)

Core validation: [`MaxAndMaskingFunctionGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/MaxAndMaskingFunctionGenericEvaluateTest.kt)

Rust source and parity coverage: [`max.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/max.rs) and [`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs).

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

- [`min`](./min): the corresponding minimum operator.
- [`masking`](./masking): binary selection of one polynomial versus zero.
