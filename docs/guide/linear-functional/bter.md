# Balance ternaryzation

## Contract

`BalanceTernaryzationFunction<V>` maps a linear polynomial to the three values $-1$, $0$, and $1$. The current function is a thresholded sign function; it is not a binary pair $(y'_p,y'_n)$ and it does not use a discrete-versus-continuous mode.

## Definition and piecewise value

For input value $x$ and `epsilon = \varepsilon`:

$$
y = \operatorname{BTer}(x) = \begin{cases}
1, & x > \varepsilon \\
0, & -\varepsilon \le x \le \varepsilon \\
-1, & x < -\varepsilon
\end{cases}
$$

The equalities at $x=\varepsilon$ and $x=-\varepsilon$ belong to the zero branch in the direct evaluator. The default $\varepsilon$ is `Flt64(1e-6)`.

## Boundary, tolerance, and Undefined

This function has no `tolerance` or `Undefined` result. `evaluate()` returns `null` only when the input polynomial cannot be evaluated. The threshold is the public `epsilon` parameter, and the comparisons are strict on the two nonzero branches.

The solver representation is built by `UnivariateLinearPiecewiseFunction`. It inserts a transition precision of `Flt64(1e-10)` around $-\varepsilon$ and $+\varepsilon$, so the solver model is a piecewise-linear approximation with narrow ramp segments. It is not safe to infer exact solver endpoint behavior from the direct `evaluate()` branch without checking the generated piecewise segments.

## Current API

### Kotlin

```kotlin
BalanceTernaryzationFunction(
    x: LinearPolynomial<V>,
    epsilon: Flt64 = Flt64(1e-6),
    extract: Boolean = true,
    converter: IntoValue<V>,
    name: String = "bter",
    displayName: String? = null,
    fallbackLower: Flt64 = Flt64(-1e6),
    fallbackUpper: Flt64 = Flt64(1e6)
)
```

`extract` is retained for compatibility and is currently unused; the implementation always creates the piecewise helper. `fallbackLower` and `fallbackUpper` supply missing breakpoint endpoints, but the nested piecewise registration still needs a provable finite input range for automatic Big-M inference.

### Rust

Source: [`balance_ternaryzation.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/balance_ternaryzation.rs)

Rust has a same-named helper, but it is not a one-to-one API for Kotlin's thresholded input function: it accepts no input `Linear<V>`, `epsilon`, or fallback bounds. `BalanceTernaryzationFunction::new(id, name)` creates a continuous result together with positive and negative binary variables; its value is `positive - negative`.

```rust
BalanceTernaryzationFunction::new(id: u64, name: &str) -> Self
BalanceTernaryzationFunction::result_variable(&self) -> &ContinuousVariableItem
BalanceTernaryzationFunction::positive_variable(&self) -> &BinaryVariableItem
BalanceTernaryzationFunction::negative_variable(&self) -> &BinaryVariableItem
```

There is currently no direct Rust API with Kotlin's `Linear<V> + epsilon` threshold signature. To model that contract, compose binary/conditional functions around an input polynomial and then feed the positive/negative indicators into this helper, or use the helper only when those indicators already exist.

## Auxiliary variables and registration model

The function creates an internal `UnivariateLinearPiecewiseFunction` named ``name`_impl`. Its result is exposed through `result`; `helperVariables` delegates to the nested piecewise function. The nested function creates one real result variable and one binary selector per segment.

`registerAuxiliaryTokens` and `registerConstraints` delegate to that nested function. The nested registration requires exactly one active segment, bounds each selector segment, and links the result to the segment's slope/intercept with linear Big-M inequalities.

## `evaluate()` versus the solver model

Direct evaluation follows the three-branch formula above. The solver uses the generated ramps: near each threshold it may produce an interpolated value instead of exactly $-1$, $0$, or $1$, and at the closed breakpoint the first matching piecewise segment determines the direct nested value. Treat this as a current implementation warning, not as an additional documented contract.

## Minimal current example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.BalanceTernaryzationFunction
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
    val function = BalanceTernaryzationFunction(
        x = xPoly,
        epsilon = Flt64(1e-6),
        converter = IntoValue.Identity,
        name = "bter"
    )

    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(2.0))) == Flt64.one)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero)) == Flt64.zero)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(-2.0))) == Flt64(-1.0))
}
```

```rust [Rust]
use ospf_rust_core::symbol::function::BalanceTernaryzationFunction;
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};

let function = BalanceTernaryzationFunction::new(1, "bter");
let positive = function.positive_variable().clone();
let positive_token = Token::from_generic(positive.clone(), positive.index());
positive_token.set_result(1.0);
let negative = function.negative_variable().clone();
let negative_token = Token::from_generic(negative.clone(), negative.index());
negative_token.set_result(0.0);
let mut tokens = VecTokenList::new();
tokens.add_token(positive_token);
tokens.add_token(negative_token);
let value = <BalanceTernaryzationFunction as FunctionSymbol>::calculate_value(
    &function,
    &tokens,
    false,
);
assert_eq!(value, Some(1.0));
```

:::

## Source and core tests

- [Implementation: `BalanceTernaryzation.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/BalanceTernaryzation.kt)
- [Core generic evaluation test: `FunctionSymbolDiscreteGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolDiscreteGenericEvaluateTest.kt)
- [Complete example: `BTerTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/BTerTest.kt)
- [Rust implementation: `balance_ternaryzation.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/balance_ternaryzation.rs)
- [Rust core coverage: `p0_evaluation_tests.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/p0_evaluation_tests.rs)

## Related pages

- [Binaryzation](/guide/linear-functional/bin)
- [Unit linear piecewise functions](/guide/linear-functional/ulp)
- [Conditional IF](/guide/linear-functional/if)
