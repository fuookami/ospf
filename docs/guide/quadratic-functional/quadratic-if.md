# Quadratic Conditional IF

`QuadraticIfFunction` is the quadratic-input counterpart of the linear conditional IF. The condition is a bounded `QuadraticPolynomial<V>`; the result is a binary variable that selects the relation's true or false branch. The symbol composes the linear `IfFunction` through the shared `QuadraticFunctionSymbol<V>` base, so the condition is first bound to a bridge variable by an exact equality and the linear indicator rows are applied to the bridged affine condition.

For a condition polynomial value $d=p(x)$ and strict boundary $g$, the semantics are the linear ones:

| Relation | True branch | False branch | Undefined gap |
| --- | --- | --- | --- |
| `GT` | $d\ge g$ | $d\le0$ | $0<d<g$ |
| `GE` | $d\ge0$ | $d\le-g$ | $-g<d<0$ |
| `LT` | $d\le-g$ | $d\ge0$ | $-g<d<0$ |
| `LE` | $d\le0$ | $d\ge g$ | $0<d<g$ |

$$
y = \begin{cases}
1, & \text{true branch} \\
0, & \text{false branch} \\
\text{undefined}, & \text{inside the gap}
\end{cases}
$$

## Contract

- Input: `condition: QuadraticPolynomial<V>` (single input).
- A condition containing quadratic monomials is bound to one bridge variable `${name}_input_0` by an exact quadratic equality; an affine condition passes through unchanged with no bridge variable.
- Bridge ranges are tightened to the condition's finite bounds; a bound that is not float-representable is widened only to the next representable solver value (`Math.nextUp`/`nextDown`).
- Registration validates every input has finite, un-widened bounds; widening a captured bound is rejected, tightening is safe.
- `helperVariables` = the bridge variables plus the `IfFunction` helpers (`${name}_if` and `${name}_if_nz`, both binary).
- The result `polynomial` is the unit-coefficient polynomial of `${name}_if`, lifted to a quadratic polynomial.
- Semantics inherit the linear conditional IF: relation-indicator over `GT`/`GE`/`LT`/`LE` (`EQ` and `NE` are rejected), `strictBoundary` defaulting to `NONZERO_TOLERANCE = 1e-10`, `delta` defaulting to `strictBoundary`, and a three-valued undefined gap.
- Generic values require `V : RealNumber<V>, V : NumberField<V>` and an `IntoValue<V>` converter.

## Solver mathematical model

Registration first validates the captured input bounds, then submits one exact quadratic equality for the quadratic condition:

$$
p(x)-\mathrm{bridge}_0=0,
$$

named `${name}_input_0`, with the bridge's range tightened to the condition's finite bounds. The linear `IfFunction` is constructed over the bridged affine condition, and its range-driven indicator rows — for normalized condition $q$, true threshold $T$, false threshold $F$, finite range $L\le q\le U$, binary indicator $a$, and result $y$ = `${name}_if`:

$$
q+(L-T)a\ge L,
\qquad
q+(F-U)a\le F,
\qquad
y-a=0
$$

— are promoted to quadratic constraints on the same model. See [Conditional IF](../linear-functional/if) for the threshold derivation, the fold-to-fixed-value behavior when the declared range proves one branch, and the full row set. Because the bridge equality is quadratic, the composed model is generally a nonconvex MIQCP and requires a solver with nonconvex quadratic constraint support.

## Current API

### Kotlin

Source: [`QuadraticIf.kt` (`QuadraticIfFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticIf.kt)

```kotlin
QuadraticIfFunction(
    condition: QuadraticPolynomial<V>,
    relation: Comparison = Comparison.GT,
    strictBoundary: V? = null,
    delta: V? = null,
    converter: IntoValue<V>,
    name: String = "quadratic_if",
    displayName: String? = null
)
```

The class extends `QuadraticFunctionSymbol<V>` and delegates to `IfFunction(condition = inputs[0], ...)` after the condition has been bound.

### Rust

Rust now provides a same-named wrapper in `quadratic_function.rs`: [`QuadraticIfFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs). It bridges the quadratic condition to a linear expression — every genuinely-quadratic condition gets one bridge variable pinned by an exact quadratic equality, while affine conditions pass through unchanged — and wraps the same building block described below, preserving the three-valued gap semantics and the explicit-bounds Big-M policy.

```rust
QuadraticIfFunction::new(
    id: u64,
    name: &str,
    condition: Quadratic<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    condition_bounds: ConditionBounds<V>,
) -> Result<QuadraticIfFunction<V>>
QuadraticIfFunction::with_declared_dependencies(
    self,
    dependency_ids: Vec<u64>,
) -> Self
```

The bridge is named `{name}_bridge`; `result_variable()` returns the inner indicator's binary result variable, and `relation()`, `strict_boundary()`, and `condition_bounds()` expose the stored configuration. Construction preflight-validates the bounds, the boundary, and the condition's finiteness. The mechanism Big-M comes only from the explicit `condition_bounds` — token bounds are never read — and direct evaluation classifies the original quadratic condition three-valued: true branch `1`, false branch `0`, `None` inside the gap (collapsed to `0` when `zero_if_none` is set).

Internally, the wrapper bridges the quadratic condition to a linear expression first (with `QuadraticLinearFunction`, which registers $p(x)-\mathrm{bridge}=0$), then creates a range-driven relation indicator over the bridged linear condition:

Source: [`conditional_indicator.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/conditional_indicator.rs)

```rust
ConditionalIndicatorFunction::new(
    id: u64,
    name: &str,
    condition: Linear<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    bounds: ConditionBounds<V>,
) -> Result<ConditionalIndicatorFunction<V>>
ConditionalIndicatorFunction::named(
    name: impl AsRef<str>,
    condition: Linear<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    bounds: ConditionBounds<V>,
) -> Result<Self>
```

`ConditionRelation` offers `Greater`, `GreaterEqual`, `Less`, and `LessEqual`; construction preflight-validates the bounds, boundary, and polynomial finiteness.

## Evaluate versus solver

The direct evaluator resolves the original inputs, writes the computed input values into the bridge slots, and delegates to the linear `IfFunction`'s evaluate. Gap and boundary-gap semantics therefore match the linear page exactly: a value on the true branch maps to `1`, a value on the false branch maps to `0`, and a value inside the gap maps to `null`. The Rust `QuadraticIfFunction` skips the bridge bookkeeping and evaluates the original quadratic condition directly, classifying it with the same three-valued semantics (`None` inside the gap, `0` when `zero_if_none` is set).

Note the evaluation entry point: there is no single-map `evaluate(values)` on these classes. The overload is `evaluate(values, tokenTable, converter, zeroIfNone)`, for example `f.evaluate(mapOf(x to Flt64(3.0)), null, IntoValue.Identity, false)`. `prepare(values, tokenTable, converter)` delegates to the same path with `zeroIfNone = false`.

The solver must represent the entire declared condition range, so a value inside the gap has no binary branch and can make the model infeasible. The evaluator classifies one supplied value and can return `null`; it does not need bounds, while registration does.

## Minimal example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticIfFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial

val x = RealVar("x").also {
    it.range.geq(Flt64(-2.0))
    it.range.leq(Flt64(2.0))
}
val condition = QuadraticPolynomial(
    monomials = listOf(QuadraticMonomial.quadratic(Flt64.one, x, x)),
    constant = Flt64(-1.0)
)
val function = QuadraticIfFunction(
    condition = condition,
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

check(value(mapOf(x to Flt64.zero)) == Flt64.zero)   // x^2 - 1 = -1, false branch
check(value(mapOf(x to Flt64(2.0))) == Flt64.one)    // x^2 - 1 = 3, true branch
check(value(mapOf(x to Flt64(1.1))) == null)         // x^2 - 1 = 0.21, inside (0, 0.5)
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::{ConditionBounds, ConditionRelation, QuadraticIfFunction};
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

// Condition x^2 with x in [0, 2] => condition range [0, 4], gap 0.5
let condition = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
let qif = QuadraticIfFunction::new(
    1,
    "qif",
    condition,
    ConditionRelation::Greater,
    0.5,
    ConditionBounds { lower: 0.0, upper: 4.0 },
)
.expect("valid quadratic condition");

let tokens_for = |value: f64| {
    let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
    let mut tokens = VecTokenList::<f64>::new();
    let tx = Token::from_generic(x, 0);
    tx.set_result(value);
    tokens.add_token(tx);
    tokens
};

assert_eq!(
    <QuadraticIfFunction as FunctionSymbol>::calculate_value(&qif, &tokens_for(2.0), false),
    Some(1.0) // x^2 = 4 >= 0.5, true branch
);
assert_eq!(
    <QuadraticIfFunction as FunctionSymbol>::calculate_value(&qif, &tokens_for(0.0), false),
    Some(0.0) // x^2 = 0 <= 0, false branch
);
assert_eq!(
    <QuadraticIfFunction as FunctionSymbol>::calculate_value(&qif, &tokens_for(0.5), false),
    None // x^2 = 0.25, inside (0, 0.5)
);
```

:::

## Tests and references

- Kotlin implementation: [`QuadraticIf.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticIf.kt)
- Kotlin composition, gap-semantics, and mechanism-model test (covers `QuadraticIfFunction`, `QuadraticIfInFunction`, and `QuadraticIfThenFunction`): [`QuadraticFunctionCompositionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionCompositionTest.kt)
- Function-symbol README documenting the quadratic composition contract: [`function/README.md`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/README.md)
- Rust building block: [`conditional_indicator.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/conditional_indicator.rs)
- Rust conditional regression test (covers `ConditionalIndicatorFunction` and `ConditionalIfFunction` over linear expressions): [`conditional_function_solver_regression.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/conditional_function_solver_regression.rs)
- Rust wrapper implementation with in-file regression tests (`quadratic_if_classifies_three_valued_condition` and `quadratic_if_registers_indicator_rows_over_the_bridge_column`): [`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)
- Rust dedicated contract test: [`function_symbol_quadratic_if.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_if.rs); end-to-end solver coverage: [`gurobi_quadratic_model_integration.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_quadratic_model_integration.rs) (`gurobi_solves_quadratic_if_with_non_linear_input`).

## Related pages

- [Conditional IF](../linear-functional/if)
- [Quadratic conditional interval](./quadratic-if-in)
- [Quadratic If-Then](./quadratic-if-then)
- [Quadratic Linear](./quadratic-linear)
