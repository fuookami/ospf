# Masking

`MaskingFunction` models the product of a linear polynomial and a binary mask:

$$
y = p\,z,\qquad z\in\{0,1\}.
$$

The current `ospf-kotlin-core` implementation is the authoritative contract.

## Contract

- Input: `input: LinearPolynomial<V>` and `mask: AbstractVariableItem<*, *>`.
- The mask is intended to be a binary variable (`BinVar`).
- Output: a `RealVar` exposed through `resultPolynomial`.
- `evaluate` returns zero when the mask is absent or equal to zero; otherwise it evaluates and returns `input` (so a non-binary supplied value is treated as “on”).
- `V` must implement `RealNumber<V>` and `NumberField<V>` and be accompanied by an `IntoValue<V>` converter.

## Mathematical definition

For a valid binary mask,

$$
y = \begin{cases}p,&z=1,\\0,&z=0.\end{cases}
$$

If finite bounds $L\le p\le U$ are available, the standard four linear inequalities are

$$
y\le Uz,\quad y\ge Lz,\quad y-p\le -L(1-z),\quad y-p\ge -U(1-z).
$$

The implementation obtains $L,U$ from the input polynomial when possible; otherwise it uses $-M,M$.

## Domain and boundaries

The solver semantics require `mask` to take only 0 or 1. A mask value other than zero is accepted by `evaluate`, but it is not a valid assignment for the binary solver model. If the input has no finite bounds, pass an explicit `bigM`; the default fallback is currently $10^6$. The result is not forced non-negative, so negative input bounds are supported when the Big-M bounds are valid.

## Current API

### Kotlin

Source: [`Masking.kt` (`MaskingFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Masking.kt#L42-L122)

```kotlin
MaskingFunction(
    input: LinearPolynomial<V>,
    mask: AbstractVariableItem<*, *>,
    bigM: V? = null,
    converter: IntoValue<V>,
    name: String,
    displayName: String? = null
)
```

The same source file also contains `MaskingWithPolyMaskFunction` and `MaskingRangeFunction`; those are distinct APIs and should not be substituted for the binary-mask contract described here.

### Rust

Rust provides the binary-variable counterpart [`MaskingFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/masking.rs):

```rust
MaskingFunction::new(
    id: u64,
    name: &str,
    input: Linear<V>,
    mask_var: BinaryVariableItem,
) -> MaskingFunction<V>

MaskingFunction::with_big_m(
    id: u64,
    name: &str,
    input: Linear<V>,
    mask_var: BinaryVariableItem,
    big_m: V,
) -> MaskingFunction<V>
```

Rust requires the mask itself to be a `BinaryVariableItem`, while Kotlin accepts an abstract variable item and relies on the caller's binary contract. The Rust module also has direct counterparts for the two Kotlin variants: [`MaskingWithPolyMaskFunction::new`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/masking.rs) / `with_big_m` take a `Linear<V>` mask expression and create a binary bridge, while [`MaskingRangeFunction::new`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/masking.rs) takes a linear mask plus `lower` and `upper`. `result_variable()`, `mask_variable()`/`mask_bridge_variable()`, and `big_m()` expose the Rust state; there is no Kotlin converter or `displayName` argument.

## Solver mathematical model

For binary mask $z$, signed result $y$, and finite $L\le p\le U$, the exact rows are

$$
y\le Uz,\qquad y\ge Lz,
$$

$$
y-p\le-L(1-z),\qquad y-p\ge-U(1-z).
$$

Kotlin and Rust use this four-row product linearization. Kotlin relies on the caller to supply a binary mask; Rust's type requires it. Missing input bounds are replaced by $[-M,M]$.

## `evaluate` versus solver

`evaluate` looks up `mask` directly: missing mask and exact zero both return zero, while every other value evaluates the input. The solver registers `resultVar` and assumes the mask variable is binary, so it enforces the two intended cases only. This difference matters if a caller uses incomplete maps or non-binary mask values for evaluation.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.MaskingFunction
import fuookami.ospf.kotlin.core.variable.BinVar
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.eq
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

val x = RealVar("x")
val mask = BinVar("mask")
val input = LinearPolynomial(
    monomials = listOf(LinearMonomial(Flt64.one, x)),
    constant = Flt64.one
)
val masking = MaskingFunction(
    input = input,
    mask = mask,
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "masking"
)
val value = masking.evaluate(mapOf<Symbol, Flt64>(x to Flt64(5.0), mask to Flt64.one))
check(value != null && (value eq Flt64(6.0)))
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::MaskingFunction;
use ospf_rust_core::variable::{BinaryVariableItem, VariableId};

let mask = BinaryVariableItem::create(VariableId::standalone(2), "mask");
let input = Linear::new(vec![LinearMonomial::new(1.0, 0)], 1.0);
let masking = MaskingFunction::with_big_m(1, "masking", input, mask, 10.0_f64);
assert_eq!(masking.big_m(), &10.0);
let _result = masking.result_variable();
```

:::

Complete example: [`MaskingTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/MaskingTest.kt)

Core validation: [`MaxAndMaskingFunctionGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/MaxAndMaskingFunctionGenericEvaluateTest.kt) and [`MaskingRangeFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/MaskingRangeFunctionDedicatedTest.kt). Rust's dedicated range coverage is [`function_symbol_masking_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_masking_range.rs).

Rust source and parity coverage: [`masking.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/masking.rs) and [`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs).

## Polynomial mask: `MaskingWithPolyMaskFunction`

This variant accepts a linear mask expression instead of a variable:

$$
m = maskPoly,\qquad y = input\cdot m,\qquad m\in\{0,1\}.
$$

It creates `maskVar` (a `BinVar`) and `resultVar` (a `RealVar`), registers `maskPoly = maskVar`, then applies the same four Big-M constraints as `MaskingFunction`. Direct `evaluate` returns zero for a missing or zero mask expression and otherwise evaluates `input`; the solver relies on the binary equality to enforce the intended domain.

Source: [`Masking.kt` (`MaskingWithPolyMaskFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Masking.kt#L227-L430)

```kotlin
val mask = BinVar("mask")
val input = LinearPolynomial(
    monomials = listOf(LinearMonomial(Flt64.one, x)),
    constant = Flt64.one
)
val maskPoly = LinearPolynomial(
    monomials = listOf(LinearMonomial(Flt64.one, mask)),
    constant = Flt64.zero
)
val polyMask = MaskingWithPolyMaskFunction(
    input = input,
    maskPoly = maskPoly,
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "poly_mask"
)
val value = polyMask.evaluate(
    mapOf<Symbol, Flt64>(x to Flt64(5.0), mask to Flt64.one)
)
check(value != null && (value eq Flt64(6.0)))
```

There is no dedicated current example/test for this variant; use the source and shared masking test, plus the actual [`linear_function` example directory](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function).

## Masked range: `MaskingRangeFunction`

$$
lower\cdot m\le y\le upper\cdot m.
$$

The constructor requires `lower <= upper`, creates a signed `RealVar`/continuous result, and registers only those two inequalities; the mask expression is expected to be binary, but this class neither creates nor enforces a binary mask. Negative lower bounds are valid. With binary `m`, `m=0` gives `y=0` and `m=1` gives `lower\le y\le upper`.

Source: [`Masking.kt` (`MaskingRangeFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Masking.kt#L432-L567)

```kotlin
val mask = BinVar("range_mask")
val maskPoly = LinearPolynomial(
    monomials = listOf(LinearMonomial(Flt64.one, mask)),
    constant = Flt64.zero
)
val rangeMask = MaskingRangeFunction(
    mask = maskPoly,
    lower = Flt64(2.0),
    upper = Flt64(5.0),
    converter = IntoValue.Identity,
    name = "range_mask"
)
val value = rangeMask.evaluate(
    mapOf<Symbol, Flt64>(mask to Flt64.one, rangeMask.resultVar to Flt64(4.0))
)
check(value != null && (value eq Flt64(4.0)))
```

Direct evaluation returns `null`/`None` when either the mask or `resultVar` is missing; a zero mask returns zero, and a nonzero mask clamps to the scaled interval (swapping endpoints when the mask is negative). The registered rows use the signed lower bound directly, so `lower < 0` is supported for the intended binary mask contract. Rust's `zero_if_none = true` evaluation option can still substitute zero for missing polynomial tokens, as with the rest of its function symbols. The dedicated tests cover signed bounds, missing values, and both registered rows.

## Related pages

- [`abs`](./abs): split a value into positive and negative parts.
- [`max`](./max) and [`min`](./min): aggregate candidate polynomials.
- [`slack`](./slack) and [`slack-range`](./slack-range): other bounded linear transformations.
