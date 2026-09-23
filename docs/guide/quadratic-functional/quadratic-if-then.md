# Quadratic If-Then

`QuadraticIfThenFunction` is the quadratic-input counterpart of the linear If-Then. The condition and the gated value are both bounded `QuadraticPolynomial<V>`; when the condition holds, the result equals the then polynomial, and when it fails, the result is zero. The symbol composes the linear `IfThenFunction` through the shared `QuadraticFunctionSymbol<V>` base, so both inputs are first bound to bridge variables by exact equalities and the linear conditional-value rows are applied to the bridged affine inputs.

For a condition polynomial value $d=p(x)$, then polynomial $q=r(x)$, and strict boundary $g$, the condition semantics are the linear ones:

| Relation | True branch | False branch | Undefined gap |
| --- | --- | --- | --- |
| `GT` | $d\ge g$ | $d\le0$ | $0<d<g$ |
| `GE` | $d\ge0$ | $d\le-g$ | $-g<d<0$ |
| `LT` | $d\le-g$ | $d\ge0$ | $-g<d<0$ |
| `LE` | $d\le0$ | $d\ge g$ | $0<d<g$ |

The gated result is:

$$
y = \begin{cases}
q, & \text{true branch} \\
0, & \text{false branch} \\
\text{undefined}, & \text{inside the gap}
\end{cases}
$$

## Contract

- Inputs: `condition: QuadraticPolynomial<V>` and `thenPoly: QuadraticPolynomial<V>` (two inputs; both get their own bridge variables when they contain quadratic monomials).
- Each quadratic input is bound to a bridge variable `${name}_input_$index` by an exact quadratic equality; affine inputs pass through unchanged with no bridge variable.
- Bridge ranges are tightened to each input's finite bounds; a bound that is not float-representable is widened only to the next representable solver value (`Math.nextUp`/`nextDown`).
- Registration validates every input has finite, un-widened bounds; widening a captured bound is rejected, tightening is safe.
- `helperVariables` = the bridge variables plus the `IfThenFunction` helpers (`${name}_ind`, a binary condition indicator, and `${name}_y`, a real result variable ranged over the then bounds united with zero).
- The result `polynomial` is the unit-coefficient polynomial of `${name}_y`, lifted to a quadratic polynomial.
- Semantics inherit the linear If-Then: conditional value with a zero false branch, relation-indicator over `GT`/`GE`/`LT`/`LE`, `strictBoundary` defaulting to `NONZERO_TOLERANCE = 1e-10`, `delta` defaulting to `strictBoundary`, and a three-valued undefined gap.
- Generic values require `V : RealNumber<V>, V : NumberField<V>` and an `IntoValue<V>` converter.

## Solver mathematical model

Registration first validates the captured input bounds, then submits one exact quadratic equality per quadratic input:

$$
p(x)-\mathrm{bridge}_0=0,
\qquad
r(x)-\mathrm{bridge}_1=0,
$$

named `${name}_input_0` and `${name}_input_1`, with each bridge's range tightened to the corresponding input's finite bounds. The linear `IfThenFunction` is constructed over the bridged affine condition $c=\mathrm{bridge}_0$ and then polynomial $q=\mathrm{bridge}_1$. For normalized condition $c\in[L_c,U_c]$, true threshold $T$, false threshold $F$, and binary indicator $i$ = `${name}_ind`, it submits the two condition rows

$$
c+(L_c-T)i\ge L_c,
\qquad
c+(F-U_c)i\le F
$$

and, for then bounds $L\le q\le U$ and result $y$ = `${name}_y`, the four gating rows

$$
y\le U\,i,
\qquad
y\ge L\,i,
\qquad
y-q\le-L(1-i),
\qquad
y-q\ge-U(1-i).
$$

All rows are promoted to quadratic constraints on the same model. Together they enforce $i=0\Rightarrow y=0$ and $i=1\Rightarrow y=q$; when the declared condition range proves one branch, the indicator and result fold to fixed values. See [If-Then](../linear-functional/if-then) for the full derivation. Because the bridge equalities are quadratic, the composed model is generally a nonconvex MIQCP and requires a solver with nonconvex quadratic constraint support.

## Current API

### Kotlin

Source: [`QuadraticIfThen.kt` (`QuadraticIfThenFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticIfThen.kt)

```kotlin
QuadraticIfThenFunction(
    condition: QuadraticPolynomial<V>,
    thenPoly: QuadraticPolynomial<V>,
    relation: Comparison = Comparison.GT,
    strictBoundary: V? = null,
    delta: V? = null,
    converter: IntoValue<V>,
    name: String = "quadratic_ifthen",
    displayName: String? = null
)
```

The class extends `QuadraticFunctionSymbol<V>` and delegates to `IfThenFunction(condition = inputs[0], thenPoly = inputs[1], ...)` after both inputs have been bound.

### Rust

Rust now provides a same-named wrapper in `quadratic_function.rs`: [`QuadraticIfThenFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs). It bridges both quadratic inputs to linear expressions — one bridge variable pinned by an exact quadratic equality per genuinely-quadratic input, pass-through for affine inputs — and wraps the same building block described below, preserving the three-valued gap semantics and the explicit-bounds Big-M policy.

```rust
QuadraticIfThenFunction::new(
    id: u64,
    name: &str,
    condition: Quadratic<V>,
    then_poly: Quadratic<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    condition_bounds: ConditionBounds<V>,
    then_bounds: ConditionBounds<V>,
) -> Result<QuadraticIfThenFunction<V>>
QuadraticIfThenFunction::with_declared_dependencies(
    self,
    dependency_ids: Vec<u64>,
) -> Self
```

Two bridges are created: `{name}_bridge_condition` and `{name}_bridge_then`. The wrapper wraps `ConditionalThenFunction::from_parts_with_bounds`, so the result equals the then value when the condition holds, `0` on the false branch, and `None` inside the gap. Both declared bounds are validated at construction; `result_variable()` returns the inner continuous result variable, and `relation()`, `strict_boundary()`, `condition_bounds()`, and `then_bounds()` expose the stored configuration. Big-M comes only from the explicit `condition_bounds` — token bounds are never read.

Internally, the wrapper bridges the quadratic condition and then polynomial to linear expressions first (with `QuadraticLinearFunction`, which registers $p(x)-\mathrm{bridge}=0$), then creates the conditional-value gate over the bridged linear expressions:

Source: [`if_then.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/if_then.rs)

```rust
ConditionalThenFunction::from_parts_with_bounds(
    condition: Linear<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    condition_bounds: ConditionBounds<V>,
    then_poly: Linear<V>,
    then_bounds: ConditionBounds<V>,
) -> Result<ConditionalThenFunction<V>>
ConditionalThenFunction::named(
    name: impl AsRef<str>,
    condition: ConditionalIfFunction<V>,
    then_poly: Linear<V>,
    then_bounds: ConditionBounds<V>,
) -> Result<Self>
```

Undefined conditions remain `None` in `ConditionalThenFunction`; they are not silently treated as false. The legacy Rust `IfThenFunction` instead models implication between two inequalities and returns a binary result; it is not a replacement for the conditional-value gate.

## Evaluate versus solver

The direct evaluator resolves the original inputs, writes the computed values into the bridge slots, and delegates to the linear `IfThenFunction`'s evaluate. Gap and boundary-gap semantics therefore match the linear page exactly: a true condition maps to the then value, a false condition maps to zero, and a condition inside the gap maps to `null`. The Rust `QuadraticIfThenFunction` skips the bridge bookkeeping and evaluates the original quadratic condition and then expression directly with the same three-valued semantics (`None` inside the gap, `0` when `zero_if_none` is set).

Note the evaluation entry point: there is no single-map `evaluate(values)` on these classes. The overload is `evaluate(values, tokenTable, converter, zeroIfNone)`, for example `f.evaluate(mapOf(x to Flt64(2.0)), null, IntoValue.Identity, false)`. `prepare(values, tokenTable, converter)` delegates to the same path with `zeroIfNone = false`.

The solver model requires finite ranges for both inputs and has no assignment for a condition value inside the gap; such a value can make the model infeasible. An undefined condition is not silently treated as false.

## Minimal example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticIfThenFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial

val x = RealVar("x").also {
    it.range.geq(Flt64(-2.0))
    it.range.leq(Flt64(2.0))
}
val square = QuadraticPolynomial(
    monomials = listOf(QuadraticMonomial.quadratic(Flt64.one, x, x)),
    constant = Flt64(-1.0)
)
val function = QuadraticIfThenFunction(
    condition = square,
    thenPoly = square,
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

check(value(mapOf(x to Flt64.zero)) == Flt64.zero)   // condition -1 <= 0, false branch
check(value(mapOf(x to Flt64(2.0))) == Flt64(3.0))   // condition 3 >= 0.5, then value 3
check(value(mapOf(x to Flt64(1.1))) == null)         // condition 0.21, inside (0, 0.5)
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::{
    ConditionBounds, ConditionRelation, QuadraticIfThenFunction,
};
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

// Condition x^2 - 1 (GT, gap 0.5), then = 2x^2; x in [0, 2]
let condition = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], -1.0);
let then_poly = Quadratic::new(vec![QuadraticMonomial::new_quadratic(2.0, 0, 0)], 0.0);
let qifthen = QuadraticIfThenFunction::new(
    1,
    "qifthen",
    condition,
    then_poly,
    ConditionRelation::Greater,
    0.5,
    ConditionBounds { lower: -1.0, upper: 3.0 },
    ConditionBounds { lower: 0.0, upper: 8.0 },
)
.expect("valid quadratic if-then");

let tokens_for = |value: f64| {
    let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
    let mut tokens = VecTokenList::<f64>::new();
    let tx = Token::from_generic(x, 0);
    tx.set_result(value);
    tokens.add_token(tx);
    tokens
};

assert_eq!(
    <QuadraticIfThenFunction as FunctionSymbol>::calculate_value(&qifthen, &tokens_for(2.0), false),
    Some(8.0) // condition 3 >= 0.5, then value 2 * 4
);
assert_eq!(
    <QuadraticIfThenFunction as FunctionSymbol>::calculate_value(&qifthen, &tokens_for(1.0), false),
    Some(0.0) // condition 0 <= 0, zero false branch
);
assert_eq!(
    <QuadraticIfThenFunction as FunctionSymbol>::calculate_value(&qifthen, &tokens_for(1.1), false),
    None // condition 0.21, inside (0, 0.5)
);
```

:::

## Tests and references

- Kotlin implementation: [`QuadraticIfThen.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticIfThen.kt)
- Kotlin composition, gap-semantics, and mechanism-model test (covers `QuadraticIfFunction`, `QuadraticIfInFunction`, and `QuadraticIfThenFunction`): [`QuadraticFunctionCompositionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionCompositionTest.kt)
- Function-symbol README documenting the quadratic composition contract: [`function/README.md`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/README.md)
- Rust building block: [`if_then.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/if_then.rs)
- Rust conditional regression test (covers `ConditionalIfFunction`, `ConditionalIndicatorFunction`, and `ConditionalThenFunction` over linear expressions): [`conditional_function_solver_regression.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/conditional_function_solver_regression.rs)
- Rust wrapper implementation with in-file regression tests (`quadratic_if_then_gates_then_value_by_condition` and `quadratic_if_then_registers_rows_over_both_bridge_columns`): [`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)
- Rust dedicated contract test: [`function_symbol_quadratic_if_then.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_if_then.rs); end-to-end solver coverage: [`gurobi_quadratic_model_integration.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_quadratic_model_integration.rs) (`gurobi_solves_quadratic_if_then_with_non_linear_input`).

## Related pages

- [If-Then](../linear-functional/if-then)
- [Quadratic Conditional IF](./quadratic-if)
- [Quadratic conditional interval](./quadratic-if-in)
- [Quadratic Linear](./quadratic-linear)
