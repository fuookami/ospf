# Binaryzation

## Contract

`BinaryzationFunction<V>` maps one linear polynomial $p$ to a binary result. Its current contract is positive-part binarization:

$$
y = \operatorname{Bin}(p) = \begin{cases}
1, & p > 0 \\
0, & p \le 0
\end{cases}
$$

The input need not be a binary variable; it may be any `LinearPolynomial<V>` with `V : RealNumber<V> & NumberField<V>`.

## Definition and truth table

| $p$ | $y$ |
| --- | --- |
| $p>0$ | 1 |
| $p=0$ | 0 |
| $p<0$ | 0 |

The result is a binary variable, not a numeric copy of $p$.

## Boundary, tolerance, and Undefined

`evaluate()` uses a strict comparison `p > 0`; a missing polynomial value returns `null`.

The solver registration uses the fixed `NONZERO_TOLERANCE = 1e-10` as $\varepsilon$. If $a$ is the result variable and $M$ is the selected Big-M value, the essential indicator constraints are:

$$
p - M a \le 0,
\qquad
p - M' a \ge \varepsilon-M'.
$$

Consequently, $a=0$ requires $p\le0$, while $a=1$ requires $p\ge\varepsilon$. The open interval $0<p<\varepsilon$ is a solver gap: `evaluate()` returns `1`, but the linearized model has no valid branch. There is no public `tolerance` parameter on `BinaryzationFunction`.

When `bigM` is omitted, the implementation derives it from the polynomial's finite range and falls back to `BIG_M_DEFAULT = 1e6` if necessary.

## Current API

### Kotlin

```kotlin
BinaryzationFunction(
    polynomial: LinearPolynomial<V>,
    converter: IntoValue<V>,
    bigM: V? = null,
    name: String = "bin",
    displayName: String? = null
)
```

The companion `invoke` has the same arguments. `converter` is required; old scalar constructors are not part of the current API.

### Rust

Source: [`binaryzation.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/binaryzation.rs)

Rust exposes `BinaryzationFunction::new(id, name, input, threshold, big_m, method)` plus named/automatic factories. The convenience constructors are `with_big_m`/`named_big_m`/`auto_big_m` (strict `input > threshold`, with threshold zero for the Kotlin-compatible positive test) and `with_threshold`/`named_threshold`/`auto_threshold` (inclusive `input >= threshold`). `BinaryzationMethod` is one of `BigM`, `Threshold`, `Indicator`, or `SOS1`; the last two currently use the mechanism-layer Big-M equivalent.

```rust
BinaryzationFunction::new(
    id: u64,
    name: &str,
    input: Linear<V>,
    threshold: V,
    big_m: V,
    method: BinaryzationMethod,
) -> Self
BinaryzationFunction::named_big_m(name: impl AsRef<str>, input: Linear<V>, big_m: V) -> Self
BinaryzationFunction::named_threshold(name: impl AsRef<str>, input: Linear<V>, threshold: V) -> Self
```

## Auxiliary variables and registration model

For `name`, the only helper is `name_bin`, which is also `resultVar`. `helperVariables` contains this binary variable, and `resultPolynomial` is its unit-coefficient polynomial.

`registerAuxiliaryTokens` adds the result variable. `registerConstraints` calls the shared positive-indicator builder with the chosen Big-M and the fixed tolerance, then adds its linear inequalities to `AbstractLinearMechanismModel`.

## `evaluate()` versus the solver model

The evaluator separates only $p > 0$ from $p \le 0$; it does not model the numerical gap used by the solver. If the model can produce values in $(0, \text{NONZERO\_TOLERANCE})$, decide whether to change the input lattice or add a separate documented policy before relying on solver/evaluator agreement.

## Minimal current example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.BinaryzationFunction
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
    val function = BinaryzationFunction(
        polynomial = xPoly,
        converter = IntoValue.Identity,
        name = "bin"
    )

    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(2.0))) == Flt64.one)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero)) == Flt64.zero)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(-1.0))) == Flt64.zero)
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::BinaryzationFunction;
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::token::VecTokenList;

let input = Linear::new(vec![], 2.0);
let function = BinaryzationFunction::named_big_m("bin", input, 10.0);
let value = <BinaryzationFunction as FunctionSymbol>::calculate_value(
    &function,
    &VecTokenList::<f64>::new(),
    false,
);
assert_eq!(value, Some(1.0));
```

:::

## Source and core tests

- [Implementation: `Binaryzation.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Binaryzation.kt)
- [Core regression test: `FunctionSymbolRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolRegressionTest.kt)
- [Core result-polynomial contract test: `LegacyResultPolynomialContractTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/LegacyResultPolynomialContractTest.kt)
- [Complete example: `BinTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/BinTest.kt)
- [Rust implementation: `binaryzation.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/binaryzation.rs)
- [Rust core coverage: `p0_evaluation_tests.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/p0_evaluation_tests.rs)

## Related pages

- [Logical AND](/guide/linear-functional/and)
- [Logical OR](/guide/linear-functional/or)
- [Logical NOT](/guide/linear-functional/not)
- [Balance ternaryzation](/guide/linear-functional/bter)
