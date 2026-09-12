# One-of constraint

## Contract

`OneOfFunction<V>` accepts one or more linear polynomials. Its direct evaluator returns `1` exactly when one input is nonzero and `0` otherwise. During solver registration it is stronger than a free Boolean indicator: it imposes that exactly one input is nonzero and fixes the result to `1`.

This function selects no branch value and does not implement the old branch/payload API. It counts nonzero input polynomials.

## Definition and truth table

For inputs $p_1,\ldots,p_n$, let:

$$
a_i = \begin{cases}
1, & p_i \ne 0 \\
0, & p_i = 0
\end{cases},
\qquad
y = \begin{cases}
1, & \sum_{i=1}^{n} a_i = 1 \\
0, & \sum_{i=1}^{n} a_i \ne 1
\end{cases}
$$

The registered model additionally requires:

$$
\sum_{i=1}^{n} a_i = 1,
\qquad
y=1.
$$

For two inputs, the evaluator table is:

| $p_1$ nonzero | $p_2$ nonzero | $y$ |
| --- | --- | --- |
| no | no | 0 |
| no | yes | 1 |
| yes | no | 1 |
| yes | yes | 0 |

The constructor requires at least one input polynomial.

## Boundary, tolerance, and Undefined

`evaluate()` uses exact `v != 0`; a missing input value returns `null`. It does not return `Undefined`.

Each solver nonzero indicator uses tolerance $t$ as its zero band $\lvert p_i\rvert\le t$ and strict boundary $g$ as its nonzero band $p_i\ge g$ or $p_i\le-g$. The gap $t<\lvert p_i\rvert<g$ has no valid indicator assignment and can make the model infeasible. The current defaults are `NONZERO_TOLERANCE = 1e-10` and `STRICT_BOUNDARY = NONZERO_TOLERANCE * 16 + 16 * 2^-52`; omitted `bigM` is inferred from finite input bounds and otherwise falls back to `BIG_M_DEFAULT = 1e6`.

## Current API

### Kotlin

```kotlin
OneOfFunction(
    polynomials: List<LinearPolynomial<V>>,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    converter: IntoValue<V>,
    name: String = "oneof",
    displayName: String? = null
)
```

The companion `invoke` accepts `polynomials`, `bigM`, `converter`, `name`, and `displayName`; use the primary constructor to set `tolerance` or `strictBoundary`.

### Rust

Rust's [`OneOfFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/one_of.rs) has a different contract from Kotlin's exactly-one nonzero test:

```rust
OneOfFunction::new(id: u64, name: &str, polynomials: Vec<Linear<V>>) -> OneOfFunction<V>
```

It creates `selection_variables()` and returns the selected weighted sum in a continuous `result_variable()`. The selectors are model variables; Rust does not inspect whether each candidate polynomial is zero. There is therefore no one-to-one Rust API for Kotlin's `OneOfFunction` truth table; use `XorFunction` or `SatisfiedAmountFunction` when counting nonzero/binary indicators is the intended meaning.

## Solver mathematical model

For `name`, the implementation creates `name_oneof` as the result, `name_oneof_nz{i}` as one nonzero indicator per input, and `name_oneof_side{i}` as one sign-side helper per input. All are in `helperVariables`; `resultPolynomial` is the unit-coefficient polynomial of `name_oneof`.

For every input, the shared four-row Big-M block represents

$$
a_i=0\Rightarrow -t\le p_i\le t,
$$

$$
(a_i,s_i)=(1,1)\Rightarrow p_i\ge g,
\qquad
(a_i,s_i)=(1,0)\Rightarrow p_i\le-g.
$$

The implementation expands those implications and then passes

$$
\sum_i a_i=1,\qquad y=1.
$$

Rust's same-named selection function instead passes $\sum_i z_i=1$ and $y=\sum_i z_i p_i$; it does not create nonzero flags.

## `evaluate()` versus the solver model

Before registration, `evaluate()` is a total exactly-one indicator (or `null` for missing input). After registration, any assignment with zero or multiple solver nonzero indicators is infeasible rather than merely producing result `0`. This distinction is intentional in the current implementation and should be stated wherever the function is used as a model constraint.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.OneOfFunction
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
    val function = OneOfFunction(
        polynomials = listOf(xPoly, yPoly),
        converter = IntoValue.Identity,
        name = "oneof"
    )

    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64.one, y to Flt64.zero)) == Flt64.one)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64.one, y to Flt64(2.0))) == Flt64.zero)
}
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::OneOfFunction;

let x = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let y = Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0);
let one_of = OneOfFunction::new(1, "one_of", vec![x, y]);
assert_eq!(one_of.selection_variables().len(), 2);
let _result = one_of.result_variable();
```

:::

Source and core tests:

- [Implementation: `OneOf.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/OneOf.kt)
- [Core conditional registration test: `FunctionSymbolConditionalGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolConditionalGenericRegistrationTest.kt)
- [Core conditional regression test: `ConditionalFunctionRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/ConditionalFunctionRegressionTest.kt)
- [Complete example: `OneOfTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/OneOfTest.kt)

Rust source: [`one_of.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/one_of.rs).

## Related pages

- [Logical AND](/guide/linear-functional/and)
- [Logical OR](/guide/linear-functional/or)
- [Exactly-one result (`XorFunction`)](/guide/linear-functional/xor)
- [Conditional IF](/guide/linear-functional/if)
