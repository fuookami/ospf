# Quadratic Inequality Indicator

`QuadraticInequalityFunction` composes the linear inequality indicator over a bounded quadratic polynomial. For input polynomial $p(x)$, scalar right-hand side $r$, and comparison sign:

$$
y=\mathbf{1}[p(x)\ \text{sign}\ r],\qquad y\in\{0,1\}.
$$

The genuinely quadratic left-hand side is bound to a bridge variable by the shared base class `QuadraticFunctionSymbol<V>`; the linear `InequalityFunction` then applies its Big-M indicator encoding to the bridge.

## Contract

- Inputs: `lhs: QuadraticPolynomial<V>`, scalar `rhs: V`, and a `Comparison` sign; `LE`, `LT`, `GE`, `GT`, `EQ`, and `NE` are supported.
- The scalar `rhs` never gets a bridge; only the quadratic `lhs` registers the bridge `${name}_input_0` plus one exact quadratic equality.
- `createFunction` returns `InequalityFunction` with the same `rhs`, `sign`, `bigM`, `tolerance`, and `strictBoundary`, so the helper variables are the bridge plus `${name}_flag` (and `${name}_side` for `EQ`/`NE`).
- Direct evaluation keeps the linear page's gap semantics: `tolerance` (default $10^{-6}$) is the gap for `LE`/`GE` and the zero band for `EQ`/`NE`; `strictBoundary` (default $0.5$) is the minimum true-branch difference for `LT`/`GT` and the outside-band boundary for `EQ`/`NE`. Evaluation inside a gap returns `null`.
- Generic values require `V : RealNumber<V>, V : NumberField<V>` and an `IntoValue<V>` converter.
- The formulation may be nonconvex MIQCP (a quadratic input equals a variable through an EQ row); it requires a solver supporting nonconvex quadratic constraints.

## Definition and mathematical model

Let $d=p(x)-r$ and normalize the requested relation to $q\ge T$ for the true branch and $q\le F$ for the false branch — the same normalization as the linear inequality indicator (`GT`/`LE` use $q=d$, `LT`/`GE` use $q=-d$). For result $y\in\{0,1\}$ the implementation emits two Big-M rows

$$
q-M_1y\le F,\qquad q-M_2y\ge T-M_2,
$$

with the gap-relaxed multipliers $M_1=M,\ M_2=M+g$ for `GT`/`LT` and $M_1=M+g,\ M_2=M$ for `LE`/`GE`, where $g$ is `tolerance` for `LE`/`GE` and `strictBoundary` for `LT`/`GT`. Hence $y=1\Rightarrow q\ge T$ and $y=0\Rightarrow q\le F$; the open interval $(F,T)$ is intentionally infeasible. `EQ` and `NE` instead use the shared four-row zero/nonzero Big-M encoding with a side binary: the `EQ` flag equals the complement of the nonzero flag, while the `NE` flag is the nonzero flag itself.

## Solver mathematical model

### Kotlin

Let the input be $p(x)$ with finite range $[L,U]$. The base class submits exactly one quadratic equality

$$
p(x)-b=0,\qquad b\in[\tilde L,\tilde U],
$$

where $b$ is the bridge `${name}_input_0` and the range is the captured input range, widened only when float representability requires it. The linear `InequalityFunction` then applies the normalization above to $d=b-rhs$ and submits the two Big-M rows (or the four-row zero-band encoding for `EQ`/`NE`). The Big-M is inferred from the finite lhs-rhs range when `bigM` is omitted.

### Rust

Rust composes the bridge `QuadraticLinearFunction` (result column `{name}_bridge_lin_y`) with an inner `InequalityFunction` built on that column and the caller's `right`, `kind`, and `big_m`. The mechanism path emits the bridge's quadratic equality plus the inner indicator rows; when token bounds are available, the inner Big-M is re-inferred from the original quadratic input's bounds, never below the policy minimum. `EQ` and `NE` allocate the side binary inside the inner function. Direct evaluation compares the evaluated input against `right` with the fixed tolerance $\varepsilon=16\varepsilon_{f64}$ per kind.

## Current API

### Kotlin

Source: [`QuadraticInequality.kt` (`QuadraticInequalityFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticInequality.kt), composed on the base class [`QuadraticFunctionSymbol.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionSymbol.kt).

```kotlin
QuadraticInequalityFunction(
    lhs: QuadraticPolynomial<V>,
    rhs: V,
    sign: Comparison,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    converter: IntoValue<V>,
    name: String = "quadratic_inequality",
    displayName: String? = null
)
```

All linear-side parameters (`bigM`, `tolerance`, `strictBoundary`) are forwarded to the composed `InequalityFunction`.

### Rust

Rust's [`QuadraticInequalityFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs) takes a flattened `Quadratic<V>`, a scalar right-hand value, an explicit `InequalityKind`, and Big-M:

```rust
QuadraticInequalityFunction::new(
    id: u64,
    name: &str,
    input: Quadratic<V>,
    right: V,
    kind: InequalityKind,
    big_m: V,
) -> Self

QuadraticInequalityFunction::less_equal(id: u64, name: &str, input: Quadratic<V>, right: V, big_m: V) -> Self
QuadraticInequalityFunction::greater_equal(id: u64, name: &str, input: Quadratic<V>, right: V, big_m: V) -> Self
```

`InequalityKind` contains `LessEqual`, `GreaterEqual`, `Less`, `Greater`, `Equal`, and `NotEqual`; `result_variable()` returns the binary indicator. Rust has no `tolerance`/`strictBoundary` constructor parameters: its direct evaluation uses the fixed epsilon described above, and its mechanism uses the supplied or inferred Big-M.

## Evaluate versus solver

Direct evaluation classifies with the relation's gap (`tolerance` for `LE`/`GE`, `strictBoundary` for `LT`/`GT`, and the distance band for `EQ`/`NE`) and returns `null` inside the gap. The solver registration uses the Big-M rows with the same thresholds; the gap is infeasible there rather than undefined. `NE` is fully supported on both paths. Rust's direct evaluation replaces the configurable gaps with the fixed $\varepsilon=16\varepsilon_{f64}$ comparison.

## Minimal example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticInequalityFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial

val x = RealVar("x").also {
    it.range.geq(Flt64(-2.0))
    it.range.leq(Flt64(2.0))
}
val lhs = QuadraticPolynomial(
    monomials = listOf(QuadraticMonomial.quadratic(Flt64.one, x, x)),
    constant = Flt64.zero
)
val inequality = QuadraticInequalityFunction(
    lhs = lhs,
    rhs = Flt64.one,
    sign = Comparison.LE,
    converter = IntoValue.Identity,
    name = "quadratic_inequality"
)
val satisfied = inequality.evaluate(
    values = mapOf<Symbol, Flt64>(x to Flt64.one),
    tokenTable = null,
    converter = IntoValue.Identity,
    zeroIfNone = false
)
check(satisfied == Flt64.one)
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticInequalityFunction;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
let mut tokens = VecTokenList::<f64>::new();
let tx = Token::from_generic(x, 0);
tx.set_result(2.0);
tokens.add_token(tx);
let input = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
let inequality = QuadraticInequalityFunction::less_equal(1, "quadratic_inequality", input, 5.0, 10.0);
assert_eq!(inequality.calculate_value(&tokens, false), Some(1.0));
```

:::

## Tests and references

- Kotlin composition, registration, and evaluation coverage: [`QuadraticFunctionCompositionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionCompositionTest.kt)
- Rust dedicated contract test: [`function_symbol_quadratic_inequality.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_inequality.rs); end-to-end solver coverage: [`gurobi_quadratic_model_integration.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_quadratic_model_integration.rs) (`gurobi_solves_quadratic_inequality_with_non_linear_input`).

## Related pages

- [Inequality Indicator](../linear-functional/inequality): the underlying linear indicator semantics.
- [Quadratic Linear](./quadratic-linear): the shared quadratic bridge.
- [Quadratic In-Step Range](./quadratic-in-step-range): interval membership over a quadratic input.
