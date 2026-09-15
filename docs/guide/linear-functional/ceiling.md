# Ceiling

`CeilingFunction` represents the ceiling of a linear polynomial:

$$
y=\lceil p\rceil.
$$

## Contract

- Input: `x: LinearPolynomial<V>`.
- Output: an `IntVar` (`resultVar`) exposed as `resultPolynomial`.
- `evaluate` returns `null` when the input is not evaluable; otherwise it returns `ceil(p)`.
- There is no divisor `d`: this API is `ceil(p)`, not `ceil(p / d)`.
- `bigM` is retained as an unused compatibility parameter; this encoding does not use Big-M.

## Mathematical definition

For a finite real input,

$$
\lceil p\rceil=k\quad\Longleftrightarrow\quad k-1<p\le k.
$$

The solver uses `epsilon = NONZERO_TOLERANCE` to represent the strict lower edge as

$$
p\le k,\qquad p\ge k-1+\varepsilon,
$$

and registers `resultVar = k`.

## Domain and boundaries

The mathematical function accepts any finite real value, including negative values. The solver's strict inequality is tolerance-based, so values within the epsilon band below an integer can be treated differently from exact mathematical `ceil`; ordinary values away from that band are unaffected. Both the helper quotient `kVar` and `resultVar` are integer variables.

## Current API

### Kotlin

Source: [`Ceiling.kt` (constructor, evaluation, and constraints)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Ceiling.kt#L39-L119)

```kotlin
CeilingFunction(
    x: LinearPolynomial<V>,
    converter: IntoValue<V>,
    bigM: V? = null,
    name: String,
    displayName: String? = null
)
```

### Rust

Source: [`ceiling.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/ceiling.rs)

Rust accepts a flattened `Linear<V>` and provides `CeilingFunction::new(id, name, input)`, `CeilingFunction::named(name, input)`, and `CeilingFunction::auto(input)`. `input_polynomial()`, `result_variable()`, and `integer_variable()` expose the input and helper variables. The result variable is a `ContinuousVariableItem` linked to the auxiliary integer variable; Rust has no caller-supplied `big_m` or tolerance argument, and its mechanism uses the fixed `ROUNDING_EPSILON = 1e-8` boundary.

```rust
CeilingFunction::new(id: u64, name: &str, input: Linear<V>) -> Self
CeilingFunction::named(name: impl AsRef<str>, input: Linear<V>) -> Self
CeilingFunction::auto(input: Linear<V>) -> Self
```

## Solver mathematical model

With integer helper $k$ and result $y$, both implementations pass the following rows (with their own fixed $\varepsilon$) to the solver:

$$
p-k\le0,\qquad p-k\ge-1+\varepsilon,\qquad y-k=0.
$$

There is no Big-M row. Kotlin makes both $k$ and $y$ integer; Rust exposes a continuous $y$ linked to integer $k$, so the equality makes the solved value integral.

## `evaluate` versus solver

`evaluate` converts the input through `IntoValue` and calls the numeric type's `ceil`. Solver registration uses an integer variable plus the epsilon-adjusted inequalities. Thus the only boundary difference is the finite numerical tolerance used to encode a strict inequality; `bigM` does not affect this function.

## Minimal current example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.CeilingFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.eq
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

val x = RealVar("x")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val ceil = CeilingFunction(
    x = xPoly,
    converter = IntoValue.Identity,
    name = "ceil"
)
val value = ceil.evaluate(mapOf<Symbol, Flt64>(x to Flt64(1.2)))
check(value != null && (value eq Flt64.two))
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::CeilingFunction;
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::token::VecTokenList;

let function = CeilingFunction::named("ceil", Linear::new(vec![], 1.2));
let value = <CeilingFunction as FunctionSymbol>::calculate_value(
    &function,
    &VecTokenList::<f64>::new(),
    false,
);
assert_eq!(value, Some(2.0));
```

:::

Complete example: [`CeilingTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/CeilingTest.kt)

Core validation: [`FunctionSymbolDiscreteGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolDiscreteGenericEvaluateTest.kt)

Rust implementation and unit tests: [`ceiling.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/ceiling.rs)

## Related pages

- [`floor`](./floor): the lower-integer counterpart.
- [`rounding`](./rounding): nearest-integer encoding, with a distinct half-integer rule.
- [`mod`](./mod): uses floor of a scaled value, but has an explicit positive divisor.
