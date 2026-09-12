# Conditional If-Then

## Contract

`IfThenFunction<V>` gates one linear polynomial with a linear condition. When the condition is true, the result equals `thenPoly`; when it is false, the result is zero. A value in the condition gap is `Undefined` and evaluates to `null`.

The condition and the then expression are both `LinearPolynomial<V>`. The function is generic over `V : RealNumber<V> & NumberField<V>`; it does not accept a quadratic polynomial directly.

## Definition and three-valued condition

Let $d$ be the supplied condition difference and $q$ be `thenPoly`. Let $g$ be `strictBoundary`:

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

## Boundary, tolerance, and Undefined

`classify(values)` uses the shared `TruthValue` classifier. `evaluate()` returns `thenPoly.evaluateWith(values)` on `True`, the converter's zero on `False`, and `null` on `Undefined`, missing condition input, or failed evaluation.

`strictBoundary` defaults to `tolerance`, whose default is `NONZERO_TOLERANCE = 1e-10`. `delta` defaults to `strictBoundary`. Only `GT`, `GE`, `LT`, and `LE` are supported.

Constraint registration needs finite closed bounds for both the condition and `thenPoly`. They can be supplied as `conditionBounds`/`bounds` and `thenBounds`, or inferred from the corresponding polynomials. The legacy `bigM` argument cannot replace either range.

## Current API

### Kotlin

Source: [`IfThen.kt` (`IfThenFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/IfThen.kt)

```kotlin
IfThenFunction(
    condition: LinearPolynomial<V>,
    thenPoly: LinearPolynomial<V>,
    converter: IntoValue<V>,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    name: String = "ifthen",
    displayName: String? = null,
    relation: Comparison = Comparison.GT,
    conditionBounds: ConditionBounds<V>? = null,
    thenBounds: ConditionBounds<V>? = null,
    bounds: ConditionBounds<V>? = null,
    delta: V? = null
)
```

The companion `invoke` accepts the same parameters. `IfThenFunction.from` accepts a `LinearConstraintInput<V>`; it extracts the flattened condition and can use a constant-one `thenPoly` by default, returning a `LinearFunctionSymbolAdapter<V>`.

### Rust

Source: [`if_then.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/if_then.rs)

Rust keeps two distinct APIs. `ConditionalThenFunction` is the closest match to Kotlin's conditional value gate: it accepts a `ConditionalIfFunction`, a `Linear<V>` then expression, and explicit finite then bounds. The legacy `IfThenFunction` instead models implication between two `LinearInequality<V>` objects and returns a binary implication result; it is not a one-to-one replacement for Kotlin's `thenPoly` output. Undefined conditions remain `None` in `ConditionalThenFunction`; they are not silently treated as false.

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
IfThenFunction::new(
    id: u64,
    name: &str,
    premise: LinearInequality<V>,
    consequence: LinearInequality<V>,
    big_m: V,
) -> Self
```

## Solver mathematical model

For `name`, the implementation creates `name_ind` as a binary condition indicator and `name_y` as a real result variable. Both are in `helperVariables`; `resultPolynomial` is the unit-coefficient polynomial of `name_y`.

After adding the helper variables, `registerConstraints` normalizes the relation over the condition range. It adds two range-driven indicator inequalities, then gates `thenPoly` with the indicator. For then bounds $L\le q\le U$, the four gating inequalities are:

For normalized condition $c\in[L_c,U_c]$, true threshold $T$, and false threshold $F$, the two condition rows are

$$
c+(L_c-T)i\ge L_c,
\qquad
c+(F-U_c)i\le F.
$$

The four result rows are

$$
y\le U\,i,
\qquad
y\ge L\,i,
\qquad
y-q\le-L(1-i),
\qquad
y-q\ge-U(1-i).
$$

Together these rows enforce $i=0\Rightarrow y=0$ and $i=1\Rightarrow y=q$. If the condition range proves one branch, Kotlin folds the indicator and result instead. Rust's conditional counterpart uses the same indicator-plus-gating construction.

## `evaluate()` versus the solver model

The evaluator can return a then value, zero, or `null`. The solver model requires both finite ranges and has no assignment for a condition value in the gap; such a value can make the model infeasible. An undefined condition is not silently treated as false.

## Minimal current example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ConditionBounds
import fuookami.ospf.kotlin.core.symbol.function.IfThenFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

fun main() {
    val x = RealVar("x")
    val condition = LinearPolynomial(
        monomials = listOf(LinearMonomial(Flt64.one, x)),
        constant = Flt64(-2.0)
    )
    val thenPoly = LinearPolynomial<Flt64>(emptyList(), Flt64(5.0))
    val function = IfThenFunction(
        condition = condition,
        thenPoly = thenPoly,
        converter = IntoValue.Identity,
        relation = Comparison.GT,
        strictBoundary = Flt64(0.1),
        conditionBounds = ConditionBounds(Flt64(-2.0), Flt64(3.0)),
        thenBounds = ConditionBounds(Flt64(5.0), Flt64(5.0)),
        name = "ifthen"
    )

    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(3.0))) == Flt64(5.0))
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(1.0))) == Flt64.zero)
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::{
    ConditionBounds, ConditionRelation, ConditionalThenFunction,
};

let function = ConditionalThenFunction::from_parts_with_bounds(
    Linear::new(vec![], 1.0),
    ConditionRelation::GreaterEqual,
    0.1,
    ConditionBounds {
        lower: -1.0,
        upper: 2.0,
    },
    Linear::new(vec![], 5.0),
    ConditionBounds {
        lower: 5.0,
        upper: 5.0,
    },
)
.expect("valid conditional-then function");
let value = function.evaluate(&1.0, &5.0).expect("classifiable condition");
assert_eq!(value, Some(5.0));
```

:::

## Source and core tests

- [Implementation: `IfThen.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/IfThen.kt)
- [Core conditional registration test: `FunctionSymbolConditionalGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolConditionalGenericRegistrationTest.kt)
- [Core conditional regression test: `ConditionalFunctionRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/ConditionalFunctionRegressionTest.kt)
- [Core constraint-input factory test: `FunctionSymbolConstraintInputFactoryTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolConstraintInputFactoryTest.kt)
- [Complete example: `ConditionalFunctionSolveRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/ConditionalFunctionSolveRegressionTest.kt)
- [Rust implementation and unit tests: `if_then.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/if_then.rs)
- [Rust conditional regression: `conditional_function_solver_regression.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/conditional_function_solver_regression.rs)
- [Rust inequality-implication parity: `gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs)

## Related pages

- [Conditional IF](/guide/linear-functional/if)
- [Conditional interval](/guide/linear-functional/if-in)
- [One-of constraint](/guide/linear-functional/one-of)
