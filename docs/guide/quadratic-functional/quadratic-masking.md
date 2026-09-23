# Quadratic Masking

`QuadraticMaskingFunction` gates a bounded quadratic polynomial by a binary mask. For input polynomial $p(x)$ and binary mask $z$:

$$
y = \begin{cases}p(x), & z=1,\\ 0, & z=0,\end{cases}\qquad z\in\{0,1\}.
$$

Only the genuinely quadratic input is bound to a bridge variable by the shared base class `QuadraticFunctionSymbol<V>`; the mask is passed to the base class as a second, purely linear input and therefore gets no bridge of its own. The linear `MaskingFunction` then applies its four product rows to the bridge. Keeping the mask as a separate binary factor avoids the cubic or quartic expansion a naive product of the quadratic input with the mask would require.

## Contract

- Inputs: `input: QuadraticPolynomial<V>` and `mask: BinVar`.
- The mask is passed as a second (linear) input, so the base class gives it no bridge variable; only the quadratic input registers the bridge `${name}_input_0` plus one exact quadratic equality.
- `createFunction` returns `MaskingFunction`, so the solver-side result is the signed `RealVar` `${name}_masking` and the four product rows require the linear side's Big-M (inferred from the input's finite bounds by default, falling back to the library default).
- Direct evaluation returns zero when the mask is absent or zero and otherwise the input value, so a negative $p(x)$ stays negative when the mask is on.
- Generic values require `V : RealNumber<V>, V : NumberField<V>` and an `IntoValue<V>` converter.
- The formulation may be nonconvex MIQCP (a quadratic input equals a variable through an EQ row); it requires a solver supporting nonconvex quadratic constraints.

## Solver mathematical model

### Kotlin

Let the input be $p(x)$ with finite range $[L,U]$. The base class submits exactly one quadratic equality

$$
p(x)-b=0,\qquad b\in[\tilde L,\tilde U],
$$

where $b$ is the bridge `${name}_input_0`. The linear `MaskingFunction` then registers the signed result variable `${name}_masking` and the four product rows on $(b,z)$:

$$
y\le Uz,\qquad y\ge Lz,\qquad y-b\le -L(1-z),\qquad y-b\ge -U(1-z).
$$

Missing input bounds are replaced by $\pm M$ from the linear side's Big-M.

### Rust

Rust composes the bridge `QuadraticLinearFunction` (result column `{name}_bridge_lin_y`) with an inner `MaskingFunction` built on that column and the caller's `BinaryVariableItem`. The mechanism path emits the bridge's quadratic equality plus the same four product rows; when token bounds are available, the inner Big-M is re-inferred from the original quadratic input's bounds, never below the policy minimum, and the constructor-supplied value applies otherwise. The mask must be a `BinaryVariableItem`; `result_variable()` exposes the signed result column and `mask_variable()` the mask.

## Current API

### Kotlin

Source: [`QuadraticMasking.kt` (`QuadraticMaskingFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticMasking.kt), composed on the base class [`QuadraticFunctionSymbol.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionSymbol.kt).

```kotlin
QuadraticMaskingFunction(
    input: QuadraticPolynomial<V>,
    mask: BinVar,
    bigM: V? = null,
    converter: IntoValue<V>,
    name: String = "quadratic_masking",
    displayName: String? = null
)
```

The optional `bigM` is forwarded to the linear `MaskingFunction`; when omitted, it is inferred from the input's finite bounds with the library fallback.

### Rust

Rust's [`QuadraticMaskingFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs) takes a flattened `Quadratic<V>` and a binary mask variable:

```rust
QuadraticMaskingFunction::new(id: u64, name: &str, input: Quadratic<V>, mask_var: BinaryVariableItem) -> Self
QuadraticMaskingFunction::with_big_m(id: u64, name: &str, input: Quadratic<V>, mask_var: BinaryVariableItem, big_m: V) -> Self
```

`new` lets the inner masking function start from the default Big-M, while `with_big_m` fixes it explicitly. `result_variable()` returns the signed continuous result column and `mask_variable()` the binary mask.

## Evaluate versus solver

Direct evaluation resolves the original quadratic input and the mask from the supplied values: an absent or zero mask returns zero and a nonzero mask returns $p(x)$, so negative values pass through when the mask is on. The solver assumes the mask variable is binary, so only the two intended cases $y=b$ and $y=0$ are feasible, and the bridge equality keeps $b=p(x)$ exact. A non-binary mask value is accepted by `evaluate` but is not a valid assignment for the solver model.

## Minimal example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticMaskingFunction
import fuookami.ospf.kotlin.core.variable.BinVar
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial

val x = RealVar("x").also {
    it.range.geq(Flt64(-2.0))
    it.range.leq(Flt64(2.0))
}
val mask = BinVar("mask")
val input = QuadraticPolynomial(
    monomials = listOf(QuadraticMonomial.quadratic(Flt64.one, x, x)),
    constant = Flt64(-1.0)
)
val masking = QuadraticMaskingFunction(
    input = input,
    mask = mask,
    converter = IntoValue.Identity,
    name = "quadratic_masking"
)
val masked = masking.evaluate(
    values = mapOf<Symbol, Flt64>(x to Flt64(2.0), mask to Flt64.one),
    tokenTable = null,
    converter = IntoValue.Identity,
    zeroIfNone = false
)
check(masked == Flt64(3.0))
val gatedOff = masking.evaluate(
    values = mapOf<Symbol, Flt64>(x to Flt64(2.0), mask to Flt64.zero),
    tokenTable = null,
    converter = IntoValue.Identity,
    zeroIfNone = false
)
check(gatedOff == Flt64.zero)
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticMaskingFunction;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId};

let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
let mask = BinaryVariableItem::create(VariableId::standalone(2), "mask");
let mut tokens = VecTokenList::<f64>::new();
let tx = Token::from_generic(x, 0);
tx.set_result(2.0);
tokens.add_token(tx);
let tm = Token::from_generic(mask.clone(), 2);
tm.set_result(1.0);
tokens.add_token(tm);
let input = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
let masking = QuadraticMaskingFunction::with_big_m(1, "quadratic_masking", input, mask, 10.0);
assert_eq!(masking.calculate_value(&tokens, false), Some(4.0));
```

:::

## Tests and references

- Kotlin composition, registration, and evaluation coverage: [`QuadraticFunctionCompositionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionCompositionTest.kt), including the dedicated row-feasibility case `maskingConstraintsRejectIncorrectResultAndIncorrectBridge`.
- Rust dedicated contract test: [`function_symbol_quadratic_masking.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_masking.rs); end-to-end solver coverage: [`gurobi_quadratic_model_integration.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_quadratic_model_integration.rs) (`gurobi_solves_quadratic_masking_with_non_linear_input`).

## Related pages

- [Linear masking](../linear-functional/masking): the underlying four-row product linearization.
- [Quadratic Masking Range](./quadratic-masking-range): the binary-gated range variant.
- [Quadratic Linear](./quadratic-linear): the shared quadratic bridge.
