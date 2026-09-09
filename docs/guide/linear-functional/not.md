# Logical NOT

## Contract

`NotFunction<V>` accepts one linear polynomial and exposes a binary result. The result is `1` exactly when the polynomial is zero, and `0` when it is nonzero. The API is generic over `V : RealNumber<V> & NumberField<V>`.

The operation is the inverse of the current nonzero indicator. It is not a Boolean negation that assumes a pre-existing binary input variable.

## Definition and truth table

For a linear polynomial (p), let (a) be its nonzero indicator:

$$
a = \begin{cases}
1, & p \ne 0 \\
0, & p = 0
\end{cases},
\qquad
y = 1-a = \begin{cases}
1, & p = 0 \\
0, & p \ne 0
\end{cases}
$$

| (p) | (y=\operatorname{Not}(p)) |
| --- | --- |
| zero | 1 |
| nonzero | 0 |

## Boundary, tolerance, and Undefined

`evaluate()` compares the evaluated polynomial with exact zero and returns `null` if the input is missing. It has no `Undefined` return branch.

The solver's nonzero indicator uses tolerance (t) for the zero band and strict boundary (g) for the nonzero branch: `indicatorVar = 0` represents $\lvert p\rvert\le t$, while `indicatorVar = 1` requires $p\ge g$ or $p\le-g$. The interval (t<\lvert p\rvert<g) is unclassified and may make the model infeasible. The result is linked by (y+a=1).

The current defaults are `NONZERO_TOLERANCE = 1e-10` and `STRICT_BOUNDARY = NONZERO_TOLERANCE * 16 + 16 * 2^-52`. Omitted `bigM` is inferred from the polynomial's finite range, with `BIG_M_DEFAULT = 1e6` as the fallback.

## Current API

### Kotlin

```kotlin
NotFunction(
    polynomial: LinearPolynomial<V>,
    converter: IntoValue<V>,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    name: String = "not",
    displayName: String? = null
)
```

The companion `invoke` accepts `polynomial`, `converter`, `bigM`, `name`, and `displayName`; use the primary constructor to set `tolerance` or `strictBoundary` explicitly.

### Rust

Rust exposes [`NotFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/and.rs) in the same module as `OrFunction` and `XorFunction`:

```rust
NotFunction::new(id: u64, name: &str, polynomial: Linear<V>) -> NotFunction<V>
```

The result, nonzero indicator, and side helper are available through `result_variable()`, `indicator_variable()`, and `side_variable()`. Rust uses the shared nonzero-indicator defaults and does not expose Kotlin's per-instance `tolerance` or `strictBoundary` parameters; `evaluate` still treats an exact zero as true for NOT.

## Auxiliary variables and registration model

For `name`, the implementation creates:

- `name_not_nz`: the nonzero indicator (a);
- `name_not_side`: the sign-side helper used by the nonzero test;
- `name_not`: the binary result (y).

All three are in `helperVariables`. `registerAuxiliaryTokens` adds them. `registerConstraints` adds the shared four nonzero-indicator inequalities and the equality:

$$
y+a=1.
$$

The public `resultPolynomial` is the unit-coefficient polynomial of `name_not`; constraints are registered on `AbstractLinearMechanismModel`.

## `evaluate()` versus the solver model

The evaluator treats every exact nonzero value as false for NOT, even when its magnitude is below `strictBoundary`. The solver has a zero band, a strict nonzero band, and an unclassified gap. Do not use a value from the gap as a solver input without changing the boundaries or the model's value lattice.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.NotFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

fun main() {
    val x = RealVar("x")
    val xPoly = LinearPolynomial(
        monomials = listOf(LinearMonomial(Flt64.one, x)),
        constant = Flt64.zero
    )
    val function = NotFunction(
        polynomial = xPoly,
        converter = IntoValue.Identity,
        name = "not"
    )

    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero)) == Flt64.one)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(3.0))) == Flt64.zero)
}
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::NotFunction;

let input = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let not = NotFunction::new(1, "not", input);
let _result = not.result_variable();
```

:::

Source and core tests:

- [Implementation: `And.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/And.kt)
- [Core generic registration test: `FunctionSymbolGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolGenericRegistrationTest.kt)
- [Complete example: `NotTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/NotTest.kt)

Rust source and regression coverage: [`and.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/and.rs) and [`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs).

## Related pages

- [Logical AND](/guide/linear-functional/and)
- [Logical OR](/guide/linear-functional/or)
- [Binaryzation](/guide/linear-functional/bin)
- [Exactly-one result (`XorFunction`)](/guide/linear-functional/xor)
