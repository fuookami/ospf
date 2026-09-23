# Quadratic Abs

`QuadraticAbsFunction` composes the linear absolute-value function over a bounded quadratic polynomial. For an input polynomial $p(x)$:

$$
y=|p(x)|.
$$

A genuinely quadratic input is first bound to a bridge variable by the shared base class `QuadraticFunctionSymbol<V>`; the linear `AbsFunction` then applies its positive/negative decomposition with side-specific Big-M to that bridge.

## Contract

- Input: `polynomial: QuadraticPolynomial<V>` (Kotlin) or `input: Quadratic<V>` (Rust).
- If the input contains quadratic terms, the base class creates one bounded bridge `RealVar` named `${name}_input_0` and registers one exact quadratic equality `${name}_input_0`: input = bridge. The bridge range is tightened to the input's finite bounds, widened only when float representability requires it.
- Registration validates that every input has finite, un-widened bounds; widening a captured bound is rejected, tightening remains safe.
- An affine (linear/constant) input passes through with no bridge variable.
- `createFunction` returns `AbsFunction`, so the helper variables are the bridge plus the linear abs helpers `${name}_abs`, `${name}_abs_pos`, `${name}_abs_neg`, and the binary `${name}_abs_sign` (four helpers, no bridge, for a purely affine input).
- The result `polynomial` is the absolute-value result variable lifted to a quadratic polynomial with linear monomials only.
- Generic values require `V : RealNumber<V>, V : NumberField<V>` and an `IntoValue<V>` converter.
- The formulation may be nonconvex MIQCP (a quadratic input equals a variable through an EQ row); it requires a solver supporting nonconvex quadratic constraints.

## Solver mathematical model

### Kotlin

Let the input be $p(x)$ with finite range $[L,U]$. The base class submits exactly one quadratic equality

$$
p(x)-b=0,\qquad b\in[\tilde L,\tilde U],
$$

where $b$ is the bridge `${name}_input_0` and $[\tilde L,\tilde U]$ is the captured input range, widened by one ULP only when the bounds are not float-representable. The linear `AbsFunction` then decomposes the bridge value into non-negative parts:

$$
b=b^+-b^-,\qquad y=b^++b^-,
$$

with the branch rows $b^+\le M^+s$ and $b^-\le M^-(1-s)$ for the binary selector $s$. The side Big-Ms are inferred per side from the input's finite bounds ($M^+$ from the upper bound, $M^-$ from the negated lower bound) and fall back to the library default when the range is unknown.

### Rust

Rust composes the same two stages: the `QuadraticLinearFunction` bridge emits exactly one quadratic equality $q-b=0$ (row `{name}_bridge_quad_eq`), after which the four branch rows of the linear `AbsFunction` apply to the bridge column:

$$
y-b\ge 0,\qquad y+b\ge 0,\qquad y-b+M^+s\le M^+,\qquad y+b-M^-s\le 0.
$$

The asymmetric branch pair is resolved in a fixed order: an explicit `with_big_m`/`with_branch_big_m` value wins and is validated (non-finite or non-positive values fail constraint generation), then token-bound inference covers $\max(0,-2L)$ and $\max(0,2U)$, and the policy fallback is the last resort. A purely linear input created through `from_linear` degenerates: no bridge column and no quadratic constraint are emitted. When the input domain is finite, the result column is tightened to $0\le y\le\max(|L|,|U|)$; bounds are only tightened, never widened.

## Current API

### Kotlin

Source: [`QuadraticAbs.kt` (`QuadraticAbsFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticAbs.kt), composed on the base class [`QuadraticFunctionSymbol.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionSymbol.kt).

```kotlin
QuadraticAbsFunction(
    polynomial: QuadraticPolynomial<V>,
    bigM: V? = null,
    converter: IntoValue<V>,
    name: String = "quadratic_abs",
    displayName: String? = null
)
```

The optional `bigM` is forwarded to the linear `AbsFunction`; when omitted, the side-specific Big-M pair is derived from the input's finite bounds with the library fallback.

### Rust

Rust's [`QuadraticAbsFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs) bridges the quadratic input and applies the linear abs branch rows:

```rust
QuadraticAbsFunction::new(id: u64, name: &str, input: Quadratic<V>) -> Self
QuadraticAbsFunction::with_big_m(id: u64, name: &str, input: Quadratic<V>, big_m: V) -> Self
QuadraticAbsFunction::with_branch_big_m(id: u64, name: &str, input: Quadratic<V>, big_m: AbsBranchBigM) -> Self
QuadraticAbsFunction::from_linear(id: u64, name: &str, input: Linear<V>) -> Self
```

`with_big_m` uses one value for both branch rows, while `with_branch_big_m` accepts the asymmetric pair `AbsBranchBigM { positive_branch, negative_branch }`. `result_variable()` exposes the non-negative result column, `side_variable()` the binary branch selector, and `bridge_variable()` the bridge column (registered only when `has_quadratic_input()` holds). `big_m()` returns the explicit pair when one was configured.

## Evaluate versus solver

Direct evaluation resolves the original inputs including quadratic monomials, feeds the bridge values, and delegates to the linear `AbsFunction`, so both paths describe the same $|p(x)|$; missing symbols return `null`. The solver path additionally depends on valid branch Big-M coverage: an undersized explicit Big-M can exclude the true value even though `evaluate` still succeeds. There is no tolerance or three-valued undefined state.

## Minimal example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticAbsFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial

val x = RealVar("x").also {
    it.range.geq(Flt64(-2.0))
    it.range.leq(Flt64(2.0))
}
val polynomial = QuadraticPolynomial(
    monomials = listOf(QuadraticMonomial.quadratic(Flt64.one, x, x)),
    constant = Flt64(-1.0)
)
val abs = QuadraticAbsFunction(
    polynomial = polynomial,
    converter = IntoValue.Identity,
    name = "quadratic_abs"
)
val value = abs.evaluate(
    values = mapOf<Symbol, Flt64>(x to Flt64(2.0)),
    tokenTable = null,
    converter = IntoValue.Identity,
    zeroIfNone = false
)
check(value == Flt64(3.0))
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticAbsFunction;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
let mut tokens = VecTokenList::<f64>::new();
let tx = Token::from_generic(x, 0);
tx.set_result(2.0);
tokens.add_token(tx);
let input = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], -1.0);
let abs = QuadraticAbsFunction::new(1, "quadratic_abs", input);
assert_eq!(abs.calculate_value(&tokens, false), Some(3.0));
```

:::

## Tests and references

- Kotlin composition, registration, and evaluation coverage: [`QuadraticFunctionCompositionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionCompositionTest.kt)
- Rust composition acceptance coverage (bridge equality, branch rows, and Big-M resolution): [`quadratic_composition_acceptance.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/quadratic_composition_acceptance.rs); there is no dedicated `function_symbol_quadratic_abs.rs` file.

## Related pages

- [Absolute value](../linear-functional/abs): the underlying linear decomposition model.
- [Quadratic Linear](./quadratic-linear): the shared quadratic bridge.
- [Quadratic Product](./product): explicit quadratic monomial products.
