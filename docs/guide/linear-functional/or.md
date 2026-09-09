# Logical OR

## Contract

`OrFunction<V>` accepts one or more linear polynomials and exposes a binary result. The result is `1` when at least one input polynomial is nonzero, and `0` only when every input is zero. The API is generic over `V : RealNumber<V> & NumberField<V>`.

This operation tests nonzero values; it does not require the input polynomials themselves to be binary variables.

## Definition and truth table

For input polynomials $p_1,\ldots,p_n$, let $a_i$ be the nonzero indicators:

$$
a_i = \begin{cases}
1, & p_i \ne 0 \\
0, & p_i = 0
\end{cases},
\qquad
y = \begin{cases}
1, & \sum_{i=1}^{n} a_i \ge 1 \\
0, & \sum_{i=1}^{n} a_i = 0
\end{cases}
$$

For two inputs:

| (p_1) nonzero | (p_2) nonzero | (y) |
| --- | --- | --- |
| no | no | 0 |
| no | yes | 1 |
| yes | no | 1 |
| yes | yes | 1 |

The constructor requires at least one input polynomial.

## Boundary, tolerance, and Undefined

`evaluate()` uses exact `v != 0` for the first nonzero input and returns `null` if an input cannot be evaluated. It does not expose an `Undefined` value.

The solver's shared nonzero indicators use two numerical bands. Indicator `0` denotes $\lvert p_i\rvert\le t$, where `tolerance` is (t). Indicator `1` requires one of $p_i\ge g$ or $p_i\le-g$, where `strictBoundary` is (g). The open gap (t<\lvert p_i\rvert<g) has no indicator assignment and can make registration or solving infeasible.

The source constants are `NONZERO_TOLERANCE = 1e-10` and `STRICT_BOUNDARY = NONZERO_TOLERANCE * 16 + 16 * 2^-52`. The default `bigM` is inferred per polynomial from finite bounds and otherwise falls back to `BIG_M_DEFAULT = 1e6`.

## Current API

### Kotlin

```kotlin
OrFunction(
    polynomials: List<LinearPolynomial<V>>,
    converter: IntoValue<V>,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    name: String = "or",
    displayName: String? = null
)
```

The companion `invoke` accepts `polynomials`, `converter`, `bigM`, `name`, and `displayName`. Unlike the primary constructor, that convenience overload does not expose `tolerance` or `strictBoundary`.

### Rust

Rust exposes [`OrFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/and.rs):

```rust
OrFunction::new(id: u64, name: &str, polynomials: Vec<Linear<V>>) -> OrFunction<V>
```

`named` and `auto` are also available. `result_variable()`, `indicator_variables()`, and `side_variables()` expose the generated binary variables. The Rust constructor has no Kotlin-style converter, `bigM`, tolerance, or strict-boundary arguments; it uses the shared nonzero-indicator policy and infers Big-M from registered bounds when possible.

## Auxiliary variables and registration model

For `name`, the implementation creates `name_or` as the result, `name_or_nz{i}` as one nonzero indicator per input, and `name_or_side{i}` as one sign-side helper per input. All are returned by `helperVariables`.

`registerAuxiliaryTokens` adds these variables. `registerConstraints` adds the four shared nonzero-indicator inequalities for each polynomial, followed by:

$$
\sum_i a_i \ge y,
\qquad
y \ge a_i\quad(1\le i\le n).
$$

The public `resultPolynomial` is the unit-coefficient polynomial of `name_or`. The symbol registers against `AbstractLinearMechanismModel`.

## `evaluate()` versus the solver model

The evaluator treats every exact nonzero value as true, including a value smaller than `strictBoundary`. The solver encoding intentionally leaves the tolerance-to-boundary gap unclassified. For a robust model, choose a strict boundary that is separated from the values your variables can actually attain.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.OrFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

fun main() {
    val x = RealVar("x")
    val y = RealVar("y")
    val xPoly = LinearPolynomial(
        monomials = listOf(LinearMonomial(Flt64.one, x)),
        constant = Flt64.zero
    )
    val yPoly = LinearPolynomial(
        monomials = listOf(LinearMonomial(Flt64.one, y)),
        constant = Flt64.zero
    )
    val function = OrFunction(
        polynomials = listOf(xPoly, yPoly),
        converter = IntoValue.Identity,
        name = "or"
    )

    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero, y to Flt64.zero)) == Flt64.zero)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero, y to Flt64(3.0))) == Flt64.one)
}
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::OrFunction;

let x = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let y = Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0);
let or = OrFunction::new(1, "or", vec![x, y]);
assert_eq!(or.indicator_variables().len(), 2);
let _result = or.result_variable();
```

:::

Source and core tests:

- [Implementation: `And.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/And.kt)
- [Core generic registration test: `FunctionSymbolGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolGenericRegistrationTest.kt)
- [Complete example: `OrTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/OrTest.kt)

Rust source and parity coverage: [`and.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/and.rs) and [`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs).

## Related pages

- [Logical AND](/guide/linear-functional/and)
- [Logical NOT](/guide/linear-functional/not)
- [Exactly-one result (`XorFunction`)](/guide/linear-functional/xor)
- [One-of constraint](/guide/linear-functional/one-of)
