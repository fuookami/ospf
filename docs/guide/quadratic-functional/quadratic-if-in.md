# Quadratic Conditional interval

`QuadraticIfInFunction` is the quadratic-input counterpart of the linear conditional interval. The input is a bounded `QuadraticPolynomial<V>`; the result is a binary variable that is `1` when the input value lies in the closed interval $[\mathrm{lower},\mathrm{upper}]$ and `0` when it is safely outside. The symbol composes the linear `IfInFunction` through the shared `QuadraticFunctionSymbol<V>` base, so the input is first bound to a bridge variable by an exact equality and the linear interval rows are applied to the bridged affine input.

Define the two differences against the input polynomial value $x=p(t)$:

$$
d_\mathrm{lower}=p(t)-\mathrm{lower},
\qquad
d_\mathrm{upper}=\mathrm{upper}-p(t).
$$

Both differences are classified with the shared `GE` relation. For strict boundary $g$:

| Position of $p(t)$ | Lower side | Upper side | Result |
| --- | --- | --- | --- |
| $p(t)\le\mathrm{lower}-g$ | False | True or Undefined | 0 |
| $\mathrm{lower}-g<p(t)<\mathrm{lower}$ | Undefined | True | Undefined |
| $\mathrm{lower}\le p(t)\le\mathrm{upper}$ | True | True | 1 |
| $\mathrm{upper}<p(t)<\mathrm{upper}+g$ | True | Undefined | Undefined |
| $p(t)\ge\mathrm{upper}+g$ | True or Undefined | False | 0 |

At either endpoint, both closed-interval comparisons are true.

## Contract

- Input: `input: QuadraticPolynomial<V>` plus the scalar endpoints `lower: V` and `upper: V` (`lower <= upper` is required).
- An input containing quadratic monomials is bound to one bridge variable `${name}_input_0` by an exact quadratic equality; an affine input passes through unchanged with no bridge variable.
- Bridge ranges are tightened to the input's finite bounds; a bound that is not float-representable is widened only to the next representable solver value (`Math.nextUp`/`nextDown`).
- Registration validates every input has finite, un-widened bounds; widening a captured bound is rejected, tightening is safe.
- `helperVariables` = the bridge variables plus the `IfInFunction` helpers (`${name}_ifin`, `${name}_ge`, and `${name}_le`, all binary).
- The result `polynomial` is the unit-coefficient polynomial of `${name}_ifin`, lifted to a quadratic polynomial.
- Semantics inherit the linear conditional interval: closed-interval membership through two `GE` indicators, `strictBoundary` defaulting to `NONZERO_TOLERANCE = 1e-10`, `delta` defaulting to `strictBoundary`, and the three-valued gap behavior above.
- Generic values require `V : RealNumber<V>, V : NumberField<V>` and an `IntoValue<V>` converter.

## Solver mathematical model

Registration first validates the captured input bounds, then submits one exact quadratic equality for the quadratic input:

$$
p(t)-\mathrm{bridge}_0=0,
$$

named `${name}_input_0`, with the bridge's range tightened to the input's finite bounds. The linear `IfInFunction` is constructed over the bridged affine input, forming $q_l=\mathrm{bridge}_0-\mathrm{lower}$ and $q_u=\mathrm{upper}-\mathrm{bridge}_0$. For each side $j\in\{l,u\}$, with finite $L_j\le q_j\le U_j$, true threshold $T_j$, false threshold $F_j$, and binary $a_j$, it submits the two indicator rows

$$
q_j+(L_j-T_j)a_j\ge L_j,
\qquad
q_j+(F_j-U_j)a_j\le F_j
$$

and the AND result rows for $y$ = `${name}_ifin`:

$$
y\ge a_l+a_u-1,
\qquad
y\le a_l,
\qquad
y\le a_u.
$$

All rows are promoted to quadratic constraints on the same model. See [Conditional interval](../linear-functional/if-in) for the threshold derivation, the fold behavior when a side is decidable from the declared range, and the full row set. Because the bridge equality is quadratic, the composed model is generally a nonconvex MIQCP and requires a solver with nonconvex quadratic constraint support.

## Current API

### Kotlin

Source: [`QuadraticIfIn.kt` (`QuadraticIfInFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticIfIn.kt)

```kotlin
QuadraticIfInFunction(
    input: QuadraticPolynomial<V>,
    lower: V,
    upper: V,
    strictBoundary: V? = null,
    delta: V? = null,
    converter: IntoValue<V>,
    name: String = "quadratic_ifin",
    displayName: String? = null
)
```

The class extends `QuadraticFunctionSymbol<V>` and delegates to `IfInFunction(x = inputs[0], lower, upper, ...)` after the input has been bound.

### Rust

Rust now provides a same-named wrapper in `quadratic_function.rs`: [`QuadraticIfInFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs). It bridges the quadratic input to a linear expression — one bridge variable pinned by an exact quadratic equality for a genuinely-quadratic input, pass-through for affine inputs — and wraps the same building blocks described below, preserving their three-valued gap semantics and their explicit-bounds Big-M policy.

```rust
QuadraticIfInFunction::new(
    id: u64,
    name: &str,
    input: Quadratic<V>,
    lower: V,
    upper: V,
    strict_boundary: V,
    input_bounds: ConditionBounds<V>,
) -> Result<QuadraticIfInFunction<V>>
QuadraticIfInFunction::with_declared_dependencies(
    self,
    dependency_ids: Vec<u64>,
) -> Self
```

The bridge is named `{name}_bridge`, and the wrapper wraps a `RegisterableIfInRangeFunction` whose two sides are $x-\mathrm{lower}\ge0$ and $\mathrm{upper}-x\ge0$ (both `GreaterEqual`), so the result is `1` iff $\mathrm{lower}\le x\le\mathrm{upper}$. The interval ordering (`lower <= upper`) and the single-variable side-condition checks happen at construction. `result_variable()` returns the inner binary result variable, and `lower()`, `upper()`, `strict_boundary()`, and `input_bounds()` expose the stored configuration; Big-M comes only from the explicit `input_bounds` — token bounds are never read.

Internally, the wrapper bridges the quadratic input to a linear expression first (with `QuadraticLinearFunction`, which registers $p(t)-\mathrm{bridge}=0$), then describes the closed interval with two `ConditionalIfFunction` sides and registers it as one range-driven indicator pair plus an AND result:

Source: [`if_in.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/if_in.rs), [`conditional.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/conditional.rs)

```rust
ConditionalIfFunction::new(
    condition: Linear<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    bounds: ConditionBounds<V>,
) -> Result<ConditionalIfFunction<V>>
IfInRangeFunction::new(
    lower: ConditionalIfFunction<V>,
    upper: ConditionalIfFunction<V>,
) -> Result<IfInRangeFunction<V>>
IfInRangeFunction::registerable(
    self,
    id: u64,
    name: impl AsRef<str>,
) -> Result<RegisterableIfInRangeFunction<V>>
```

The interval validator requires `GreaterEqual` side relations, opposite-signed one-variable conditions, ordered endpoints, and finite bounds. `RegisterableIfInRangeFunction` creates one `ConditionalIndicatorFunction` per side and combines them with three linear AND rows.

## Evaluate versus solver

The direct evaluator resolves the original input, writes the computed value into the bridge slot, and delegates to the linear `IfInFunction`'s classify/evaluate. The three-valued interval semantics therefore match the linear page exactly: both sides true maps to `1`, either side false maps to `0`, and a value inside a boundary band maps to `null`. The Rust `QuadraticIfInFunction` skips the bridge bookkeeping and classifies the two interval differences of the original quadratic input directly with the same three-valued semantics (`None` inside a boundary band, `0` when `zero_if_none` is set).

Note the evaluation entry point: there is no single-map `evaluate(values)` on these classes. The overload is `evaluate(values, tokenTable, converter, zeroIfNone)`, for example `f.evaluate(mapOf(t to Flt64(1.0)), null, IntoValue.Identity, false)`. `prepare(values, tokenTable, converter)` delegates to the same path with `zeroIfNone = false`.

The solver uses range-driven indicator constraints; if the declared input range intersects an undefined boundary band, the resulting model may be infeasible. The evaluator classifies one supplied value and can return `null`; it does not need bounds, while registration does.

## Minimal example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticIfInFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial

val t = RealVar("t").also {
    it.range.geq(Flt64(-2.0))
    it.range.leq(Flt64(2.0))
}
val input = QuadraticPolynomial(
    monomials = listOf(QuadraticMonomial.quadratic(Flt64.one, t, t)),
    constant = Flt64(-1.0)
)
val function = QuadraticIfInFunction(
    input = input,
    lower = Flt64.zero,
    upper = Flt64(2.0),
    strictBoundary = Flt64(0.5),
    converter = IntoValue.Identity
)

fun value(values: Map<Symbol, Flt64>): Flt64? =
    function.evaluate(
        values = values,
        tokenTable = null,
        converter = IntoValue.Identity,
        zeroIfNone = false
    )

check(value(mapOf(t to Flt64.one)) == Flt64.one)    // t^2 - 1 = 0, inside [0, 2]
check(value(mapOf(t to Flt64(2.0))) == Flt64.zero)  // t^2 - 1 = 3, outside (>= 2.5)
check(value(mapOf(t to Flt64(0.9))) == null)        // t^2 - 1 = -0.19, boundary band
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::{ConditionBounds, QuadraticIfInFunction};
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

// Input x^2 with x in [0, 2] => input range [0, 4], closed interval [1, 4]
let input = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
let qifin = QuadraticIfInFunction::new(
    1,
    "qifin",
    input,
    1.0,
    4.0,
    0.5,
    ConditionBounds { lower: 0.0, upper: 4.0 },
)
.expect("valid quadratic input");

let tokens_for = |value: f64| {
    let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
    let mut tokens = VecTokenList::<f64>::new();
    let tx = Token::from_generic(x, 0);
    tx.set_result(value);
    tokens.add_token(tx);
    tokens
};

assert_eq!(
    <QuadraticIfInFunction as FunctionSymbol>::calculate_value(&qifin, &tokens_for(1.5), false),
    Some(1.0) // x^2 = 2.25, inside [1, 4]
);
assert_eq!(
    <QuadraticIfInFunction as FunctionSymbol>::calculate_value(&qifin, &tokens_for(2.0), false),
    Some(1.0) // x^2 = 4, the closed upper endpoint
);
assert_eq!(
    <QuadraticIfInFunction as FunctionSymbol>::calculate_value(&qifin, &tokens_for(0.5), false),
    Some(0.0) // x^2 = 0.25 < 1, outside
);
```

:::

## Tests and references

- Kotlin implementation: [`QuadraticIfIn.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticIfIn.kt)
- Kotlin composition, gap-semantics, and mechanism-model test (covers `QuadraticIfFunction`, `QuadraticIfInFunction`, and `QuadraticIfThenFunction`): [`QuadraticFunctionCompositionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionCompositionTest.kt)
- Function-symbol README documenting the quadratic composition contract: [`function/README.md`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/README.md)
- Rust building blocks: [`if_in.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/if_in.rs) and [`conditional.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/conditional.rs)
- Rust conditional regression test (covers `ConditionalIfFunction` and `ConditionalIndicatorFunction`, the per-side building blocks of the interval composition): [`conditional_function_solver_regression.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/conditional_function_solver_regression.rs)
- Rust wrapper implementation with in-file regression tests (`quadratic_if_in_classifies_closed_interval` and `quadratic_if_in_registers_interval_rows_over_the_bridge_column`): [`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)
- Rust dedicated contract test: [`function_symbol_quadratic_if_in.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_if_in.rs); end-to-end solver coverage: [`gurobi_quadratic_model_integration.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_quadratic_model_integration.rs) (`gurobi_solves_quadratic_if_in_with_non_linear_input`).

## Related pages

- [Conditional interval](../linear-functional/if-in)
- [Quadratic Conditional IF](./quadratic-if)
- [Quadratic If-Then](./quadratic-if-then)
- [Quadratic Linear](./quadratic-linear)
