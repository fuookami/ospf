# Logical AND

## Contract

`AndFunction<V>` accepts one or more linear polynomials and exposes a binary result. The result is `1` exactly when every input polynomial is nonzero; it is `0` when at least one input is zero. The current API is generic over `V : RealNumber<V> & NumberField<V>` and is a linear function symbol.

This is a nonzero test, not a Boolean-variable-only operation. A polynomial may be continuous or may contain intermediate symbols.

## Definition and truth table

For input polynomials $p_1,\ldots,p_n$, let $a_i$ denote the nonzero indicator:

$$
a_i = \begin{cases}
1, & p_i \ne 0 \\
0, & p_i = 0
\end{cases},
\qquad
y = \begin{cases}
1, & \sum_{i=1}^{n} a_i = n \\
0, & \text{otherwise}
\end{cases}
$$

For two inputs, the truth table is:

| $p_1$ nonzero | $p_2$ nonzero | $y$ |
| --- | --- | --- |
| no | no | 0 |
| no | yes | 0 |
| yes | no | 0 |
| yes | yes | 1 |

The constructor requires at least one input polynomial.

## Boundary, tolerance, and Undefined

`evaluate()` compares each evaluated value with exact zero. A missing polynomial input makes the result `null`; it does not return a separate `Undefined` value.

Solver registration uses the shared nonzero-indicator construction. With tolerance $t$, indicator `0` represents the zero band $\lvert p_i\rvert\le t$. With strict boundary $g$, indicator `1` represents either $p_i\ge g$ or $p_i\le-g$. Values in the gap $t<\lvert p_i\rvert<g$ are not assigned to either branch and can make the model infeasible.

The constants in the current source are `NONZERO_TOLERANCE = 1e-10` and `STRICT_BOUNDARY = NONZERO_TOLERANCE * 16 + 16 * 2^-52`. They are not `1e-6` or `0.5`. `bigM` is inferred from each polynomial's finite range when omitted; the range-based helper falls back to `BIG_M_DEFAULT = 1e6` when no usable range is available.

## Current API

### Kotlin

The primary constructor is:

```kotlin
AndFunction(
    polynomials: List<LinearPolynomial<V>>,
    converter: IntoValue<V>,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    name: String = "and",
    displayName: String? = null
)
```

The companion `invoke` accepts `polynomials`, `converter`, `bigM`, `name`, and `displayName`. The additional `fromLinearPolynomials` factory accepts `List<ToLinearPolynomial<V>>` and returns a `LinearFunctionSymbolAdapter<V>`.

### Rust

Source: [`and.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/and.rs)

Rust accepts flattened inputs and provides `AndFunction::new(id, name, polynomials)`, `AndFunction::named(name, polynomials)`, and `AndFunction::auto(polynomials)`. The public variables are available through `result_variable()`, `indicator_variables()`, and `side_variables()`. Rust has no public tolerance parameter; its evaluator uses an epsilon-level nonzero test, while mechanism Big-M is inferred from token bounds or falls back to the core default.

```rust
AndFunction::new(id: u64, name: &str, polynomials: Vec<Linear<V>>) -> Self
AndFunction::named(name: impl AsRef<str>, polynomials: Vec<Linear<V>>) -> Self
AndFunction::auto(polynomials: Vec<Linear<V>>) -> Self
```

## Solver mathematical model

For a function named `name`, the current implementation creates:

- `name_and`: the binary result;
- `name_and_nz{i}`: one nonzero indicator for each input;
- `name_and_side{i}`: one sign-side helper for each nonzero indicator.

`helperVariables` contains the result, all nonzero indicators, and all side helpers. Registration first adds these variables through `registerAuxiliaryTokens`; `registerConstraints` adds the shared four-inequality nonzero test for every input, then adds:

For each $p_i$, with nonzero flag $a_i$, side flag $s_i$, zero tolerance $t$, and strict boundary $g$, that shared block is

$$
a_i=0\Rightarrow -t\le p_i\le t,
$$

$$
(a_i,s_i)=(1,1)\Rightarrow p_i\ge g,
\qquad
(a_i,s_i)=(1,0)\Rightarrow p_i\le-g.
$$

The implementation expands these implications into four Big-M linear inequalities before appending the AND rows:

$$
\sum_i a_i \ge n y,
\qquad
y \le a_i\quad(1\le i\le n).
$$

The public `resultPolynomial` is the unit-coefficient polynomial of `name_and`. The implementation is in `And.kt` and uses `AbstractLinearMechanismModel` registration.

Rust registers the same nonzero-indicator block followed by the same AND rows; its numerical threshold and Big-M are selected by the Rust mechanism rather than Kotlin constructor arguments.

## `evaluate()` versus the solver model

The direct evaluator uses exact `v == 0`/`v != 0` semantics. The solver model deliberately separates a zero band from a strict nonzero branch, so a value that is numerically nonzero but lies between tolerance and strict boundary is accepted by `evaluate()` but has no solver branch. Choose `tolerance` and `strictBoundary` consistently with the value lattice of the model.

## Minimal current example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.AndFunction
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
    val function = AndFunction(
        polynomials = listOf(xPoly, yPoly),
        converter = IntoValue.Identity,
        name = "and"
    )

    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64.one, y to Flt64(2.0))) == Flt64.one)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64.one, y to Flt64.zero)) == Flt64.zero)
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::AndFunction;
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::token::VecTokenList;

let function = AndFunction::named(
    "and",
    vec![Linear::new(vec![], 1.0), Linear::new(vec![], 2.0)],
);
let value = <AndFunction as FunctionSymbol>::calculate_value(
    &function,
    &VecTokenList::<f64>::new(),
    false,
);
assert_eq!(value, Some(1.0));
```

:::

## Source and core tests

- [Implementation: `And.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/And.kt)
- [Core generic registration test: `FunctionSymbolGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolGenericRegistrationTest.kt)
- [Complete example: `AndTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/AndTest.kt)
- [Rust implementation: `and.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/and.rs)
- [Rust core coverage: `gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs)

## Related pages

- [Logical OR](/guide/linear-functional/or)
- [Logical NOT](/guide/linear-functional/not)
- [Exactly-one result (`XorFunction`)](/guide/linear-functional/xor)
- [Binaryzation](/guide/linear-functional/bin)
