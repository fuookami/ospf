# Modulo

`ModFunction` represents the non-negative remainder of a linear polynomial divided by a positive constant:

$$
y = p\bmod d.
$$

## Contract

- Input: `x: LinearPolynomial<V>` and a constant divisor `d: V`.
- Construction requires `d > 0`; zero and negative divisors throw an argument exception.
- The quotient helper is an `IntVar`; the remainder and result helpers are `URealVar`.
- `evaluate` returns `null` when `x` is not evaluable and otherwise computes the floor-based remainder.
- `bigM` remains a constructor parameter for compatibility but is not used by the current modulo constraints.

## Mathematical definition

The implementation uses

$$
q=\left\lfloor\frac{p}{d}\right\rfloor,\qquad y=p-dq,
$$

with

$$
0\le y<d.
$$

The registered upper bound is `y <= d - NONZERO_TOLERANCE` (converted to `V`), so the solver encodes a strict remainder upper edge with a small numerical margin.

## Domain and boundaries

`d` must be strictly positive; the API does not implement a signed-divisor variant. For negative `p`, the floor definition still gives a non-negative remainder (for example, $-0.2\bmod 0.7=0.5$). The solver uses `IntVar q`, `URealVar r`, and `URealVar result`; the remainder's strict upper bound is approximate because it uses the library tolerance. The input may be any finite evaluable linear polynomial.

## Current API

### Kotlin

Source: [`Mod.kt` (constructor, validation, evaluation, and constraints)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Mod.kt#L39-L118)

```kotlin
ModFunction(
    x: LinearPolynomial<V>,
    d: V,
    bigM: V? = null,
    converter: IntoValue<V>,
    name: String = "mod",
    displayName: String? = null
)
```

### Rust

Rust exposes [`ModFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/mod_function.rs):

```rust
ModFunction::new(id: u64, name: &str, input: Linear<V>, divisor: V) -> ModFunction<V>
```

The Rust symbol creates a continuous remainder (`result_variable()`) and an integer quotient (`quotient_variable()`). It rejects a non-finite or zero divisor during mechanism injection, but unlike Kotlin it does not require the divisor to be positive; negative divisors use the alternate signed bounds in the Rust implementation. There is no Rust `bigM` parameter.

## Solver mathematical model

With quotient $q\in\mathbb Z$, non-negative remainder $r\ge0$, and result $y\ge0$, Kotlin passes

$$
r-p+dq=0,\qquad r\le d-\varepsilon,\qquad y-r=0.
$$

Rust uses the same quotient equality and signed divisor-dependent remainder bounds; for $d>0$ they reduce to $0\le r\le d-\varepsilon$. No Big-M row is used.

## `evaluate` versus solver

`evaluate` converts the input and divisor through `IntoValue`, applies `floor`, and computes the remainder directly. The solver uses an integer quotient and a tolerance-adjusted strict upper bound. At ordinary finite values the results agree; values near the upper remainder boundary can be affected by the solver's `NONZERO_TOLERANCE`.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ModFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.eq
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

val x = RealVar("x")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val mod = ModFunction(
    x = xPoly,
    d = Flt64.two,
    converter = IntoValue.Identity,
    name = "mod"
)
val value = mod.evaluate(mapOf<Symbol, Flt64>(x to Flt64(5.0)))
check(value != null && (value eq Flt64.one))
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::ModFunction;

let input = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let modulo = ModFunction::new(1, "mod", input, 2.0_f64);
assert_eq!(modulo.divisor(), &2.0);
let _remainder = modulo.result_variable();
```

:::

Complete example: [`ModTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/ModTest.kt)

Core validation: [`FunctionSymbolDiscreteGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolDiscreteGenericEvaluateTest.kt)

Rust source and parity coverage: [`mod_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/mod_function.rs) and [`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs).

## Related pages

- [`floor`](./floor): the quotient operation used in the definition.
- [`ceiling`](./ceiling) and [`rounding`](./rounding): other discrete transformations.
- [`ulp`](./ulp): piecewise linear interpolation when a continuous approximation is preferred.
