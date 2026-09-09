# Rounding

`RoundingFunction` maps a linear polynomial to an integer using the numeric implementation's `round` operation:

$$
y=\operatorname{round}(p).
$$

The direct-evaluation and solver contracts are intentionally documented separately because their half-integer behavior is not identical.

## Contract

- Input: `x: LinearPolynomial<V>`.
- Output: an `IntVar` (`resultVar`) exposed as `resultPolynomial`.
- `evaluate` returns `null` when the input is not evaluable; otherwise it delegates to `converter.fromValue(x).round()`.
- `bigM` controls the fractional-indicator gate and is normalized to at least one; it must be large enough for that gate, although the default is one.

## Mathematical definition

The solver decomposes the input as

$$
k=\lfloor p\rfloor,\qquad b=p-k,\qquad 0\le b<1,
$$

then chooses binary $r$ with

$$
b\ge 0.5r,\qquad b\le 0.5-\varepsilon+Mr,
$$

and registers

$$
y=k+r.
$$

Consequently, the solver chooses `r = 0` below 0.5 and `r = 1` at or above 0.5: ties go toward positive infinity.

## Domain and boundaries

All finite real inputs are accepted by direct evaluation. Solver strictness around integer boundaries uses `NONZERO_TOLERANCE`, and the fractional gate uses the normalized Big-M. The important semantic boundary is a half-integer:

| Path | Half-integer rule | Example |
| --- | --- | --- |
| Solver registration | `b >= 0.5` rounds up (`k + 1`) | `2.5 -> 3`, `-1.5 -> -1` |
| `Flt32` / `Flt64` evaluation | delegates to `kotlin.math.round`, ties-to-even | `2.5 -> 2`, `-1.5 -> -2` |
| `FltX` evaluation | `BigDecimal` `HALF_UP` | `2.5 -> 3`, `-1.5 -> -2` |

Do not use a half-integer as a cross-check between `evaluate` and a solver result without accounting for this mismatch.

## Current API

### Kotlin

Source: [`Rounding.kt` (constructor, evaluation, and constraints)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Rounding.kt#L37-L147)

The numeric behavior used by `evaluate` is implemented in [`Floating.kt` (`Flt64.round`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-math/src/main/fuookami/ospf/kotlin/math/algebra/number/Floating.kt#L1264-L1277) and [`FltX.round`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-math/src/main/fuookami/ospf/kotlin/math/algebra/number/Floating.kt#L1977-L1980).

```kotlin
RoundingFunction(
    x: LinearPolynomial<V>,
    bigM: V? = null,
    converter: IntoValue<V>,
    name: String,
    displayName: String? = null
)
```

### Rust

Rust exposes [`RoundingFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/rounding.rs) with an explicit rounding kind:

```rust
RoundingFunction::new(id: u64, name: &str, input: Linear<V>, kind: RoundingKind) -> RoundingFunction<V>
RoundingFunction::round(id: u64, name: &str, input: Linear<V>) -> RoundingFunction<V>
```

`RoundingKind` is `Floor`, `Ceil`, `Round`, or `Trunc`; `named_round` and the analogous helpers are also provided. Rust keeps an integer helper but exposes a continuous `result_variable()`, and it has no Kotlin `bigM`/converter parameter. Its round boundary is implemented by the Rust function's own `ROUNDING_EPSILON` rules, so do not assume half-integer tie behavior is identical across languages.

## Auxiliary variables and registration

`helperVariables` registers `kVar` (`IntVar`), `rVar` (`BinVar`), `bVar` (`URealVar`), and `resultVar` (`IntVar`). Registration adds the floor decomposition, `b = x-k`, the fractional upper bound, the two indicator inequalities, and `result = k+r`.

## `evaluate` versus solver

`evaluate` calls the converter's `round`; it does not create helpers or use Big-M. Solver registration always uses floor plus the `b >= 0.5` gate. The paths agree away from half-integers, but can return different values exactly at ties as shown above.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.RoundingFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.eq
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

val x = RealVar("x")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val round = RoundingFunction(
    x = xPoly,
    converter = IntoValue.Identity,
    name = "round"
)
val value = round.evaluate(mapOf<Symbol, Flt64>(x to Flt64(1.2)))
check(value != null && (value eq Flt64.one))
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::RoundingFunction;

let input = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let round = RoundingFunction::round(1, "round", input);
let _result = round.result_variable();
assert!(round.sign_variable().is_some());
```

:::

Complete example: [`RoundTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/RoundTest.kt)

Core validation: [`FunctionSymbolDiscreteGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolDiscreteGenericEvaluateTest.kt) and [`FunctionSymbolRoundingGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolRoundingGenericRegistrationTest.kt)

Rust source and parity coverage: [`rounding.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/rounding.rs) and [`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs).

## Related pages

- [`floor`](./floor): the solver's integer decomposition step.
- [`ceiling`](./ceiling): upper-integer transformation without a divisor.
- [`mod`](./mod): floor-based remainder with `d > 0`.
