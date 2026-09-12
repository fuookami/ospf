# Floor

`FloorFunction` represents the floor of a linear polynomial:

$$
y=\lfloor p\rfloor.
$$

## Contract

- Input: `x: LinearPolynomial<V>`.
- Output: an `IntVar` (`resultVar`) exposed as `resultPolynomial`.
- `evaluate` returns `null` when the input is not evaluable; otherwise it returns `floor(p)`.
- There is no divisor `d`: this API is `floor(p)`, not `floor(p / d)`.
- `bigM` is retained as an unused compatibility parameter; this encoding does not use Big-M.

## Mathematical definition

For a finite real input,

$$
\lfloor p\rfloor=k\quad\Longleftrightarrow\quad k\le p<k+1.
$$

The solver uses `epsilon = NONZERO_TOLERANCE` to represent the strict upper edge as

$$
p\ge k,\qquad p\le k+1-\varepsilon,
$$

and registers `resultVar = k`.

## Domain and boundaries

The mathematical function accepts any finite real value, including negative values. The strict upper inequality is tolerance-based, so values within the epsilon band below an integer boundary can be affected in a solver model; ordinary values away from that band are unaffected. Both `kVar` and `resultVar` are integer variables.

## Current API

### Kotlin

Source: [`Floor.kt` (constructor, evaluation, and constraints)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Floor.kt#L40-L120)

```kotlin
FloorFunction(
    x: LinearPolynomial<V>,
    converter: IntoValue<V>,
    bigM: V? = null,
    name: String,
    displayName: String? = null
)
```

### Rust

Source: [`floor.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/floor.rs)

Rust accepts a flattened `Linear<V>` and provides `FloorFunction::new(id, name, input)`, `FloorFunction::named(name, input)`, and `FloorFunction::auto(input)`. `input_polynomial()`, `result_variable()`, and `integer_variable()` expose the input and helper variables. The result variable is a `ContinuousVariableItem` linked to the auxiliary integer variable; Rust has no caller-supplied `big_m` or tolerance argument, and its mechanism uses the fixed `ROUNDING_EPSILON = 1e-8` boundary.

```rust
FloorFunction::new(id: u64, name: &str, input: Linear<V>) -> Self
FloorFunction::named(name: impl AsRef<str>, input: Linear<V>) -> Self
FloorFunction::auto(input: Linear<V>) -> Self
```

## Solver mathematical model

With integer helper $k$ and result $y$, the rows passed to the solver are

$$
p-k\ge0,\qquad p-k\le1-\varepsilon,\qquad y-k=0.
$$

There is no Big-M row. Kotlin makes both $k$ and $y$ integer; Rust links its continuous result to integer $k$ by the last equality.

## `evaluate` versus solver

`evaluate` converts the input through `IntoValue` and calls the numeric type's `floor`. Solver registration uses an integer variable plus the epsilon-adjusted inequalities. The difference is limited to the numerical treatment of a strict boundary; `bigM` does not affect this function.

## Minimal current example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.FloorFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.eq
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

val x = RealVar("x")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val floor = FloorFunction(
    x = xPoly,
    converter = IntoValue.Identity,
    name = "floor"
)
val value = floor.evaluate(mapOf<Symbol, Flt64>(x to Flt64(1.8)))
check(value != null && (value eq Flt64.one))
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::FloorFunction;
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::token::VecTokenList;

let function = FloorFunction::named("floor", Linear::new(vec![], 1.8));
let value = <FloorFunction as FunctionSymbol>::calculate_value(
    &function,
    &VecTokenList::<f64>::new(),
    false,
);
assert_eq!(value, Some(1.0));
```

:::

Complete example: [`FloorTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/FloorTest.kt)

Core validation: [`FunctionSymbolDiscreteGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolDiscreteGenericEvaluateTest.kt)

Rust implementation and unit tests: [`floor.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/floor.rs)

## Related pages

- [`ceiling`](./ceiling): the upper-integer counterpart.
- [`rounding`](./rounding): nearest-integer encoding, with a distinct half-integer rule.
- [`mod`](./mod): uses floor of a scaled value, but has an explicit positive divisor.
