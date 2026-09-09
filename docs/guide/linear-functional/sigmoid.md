# Sigmoid

`SigmoidFunction` is a binary step/condition indicator, despite its name. It is not the continuous logistic function $1/(1+e^{-x})$.

> [!WARNING]
> The current implementation uses the shared three-valued discrete-condition classifier and returns `null` in its boundary gap; it does not approximate a smooth sigmoid curve.

## Contract

- Input: condition `LinearPolynomial<V>` interpreted with `relation` (default `Comparison.GT`) against zero.
- Output: `resultPolynomial` containing the binary indicator whose name appends `_sig_ind` to `name`.
- True/false values are one/zero; an undefined gap or missing input evaluates to `null`.
- `strictBoundary` and `delta` define the branch separation and discrete conversion; `tolerance` is used when strictBoundary is omitted.
- Solver registration requires finite, ordered condition bounds. Legacy `bigM` cannot replace those bounds.

## Definition and mathematical model

For the default `GT` relation and gap $g=\text{strictBoundary}$,

$$
y=\begin{cases}
1,&condition\ge g,\\
0,&condition\le0,\\
\text{undefined},&0<condition<g.
\end{cases}
$$

The corresponding reversed inequalities are used for `LT`, and the analogous matrix is used for `GE` and `LE`.

## Implementation, helper variables, and constraints

The function creates one binary indicator whose name appends `_sig_ind` to `name`, validates/normalizes the condition through the shared discrete-condition utilities, and then emits relation-indicator constraints. If the normalized condition is constant, registration folds the indicator to one or zero. Otherwise the finite condition range supplies the two branch bounds; no logistic nonlinear terms are added.

## Current API

### Kotlin

Source: [`Sigmoid.kt` (`SigmoidFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Sigmoid.kt#L45-L300)

The public factory and constructor expose the same condition, boundary, relation, bound, and naming parameters; use `conditionBounds` (or the `bounds` alias) for the finite solver range.

```kotlin
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ConditionBounds
import fuookami.ospf.kotlin.core.symbol.function.SigmoidFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val condition = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, x)), Flt64.zero
)
val sigmoid = SigmoidFunction(
    condition = condition,
    converter = IntoValue.Identity,
    strictBoundary = Flt64(0.1),
    conditionBounds = ConditionBounds(Flt64(-10.0), Flt64(10.0)),
    name = "sigmoid"
)
val value = sigmoid.evaluate(mapOf<Symbol, Flt64>(x to Flt64.one))
check(value == Flt64.one)
```

### Rust

The Kotlin page's binary relation-step semantics map to Rust [`SigmoidStepFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/sigmoid.rs), not to Rust's same-named continuous PWL symbol:

```rust
SigmoidStepFunction::from_parts(
    condition: Linear<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    bounds: ConditionBounds<V>,
) -> Result<SigmoidStepFunction<V>>

SigmoidFunction::new(
    id: u64,
    name: &str,
    input: Linear<V>,
) -> SigmoidFunction<V>
```

`SigmoidStepFunction` is the closest Rust API: it classifies a relation with `True`/`False`/`Undefined` and exposes a binary `result_variable()`. It is also available through `SigmoidFunction::step`/`relation` and the aliases `SigmoidRelationFunction` and `ConditionalSigmoidFunction`. Rust's `SigmoidFunction::new` instead builds a sampled continuous logistic PWL function; its direct evaluator is (1/(1+e^{-x})), so it is not a one-to-one replacement for the Kotlin step indicator.

## Evaluate versus solver

Direct `evaluate` calls `classify` and maps True/False/Undefined to one/zero/`null`. Solver registration runs the precheck, normalizes the condition, folds constants, registers the indicator, and adds the shared relation constraints. A false direct branch does not bypass solver-time range validation.

## Boundaries, tolerance, and Undefined

Missing condition values or non-finite values fail classification and produce `null` from `evaluate`. The gap is relation-dependent; for default GT it is $(0,g)$. Non-positive or non-finite `strictBoundary`/`delta`, reversed/sentinel bounds, and invalid Big-M values return a failed Result during registration.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ConditionBounds
import fuookami.ospf.kotlin.core.symbol.function.SigmoidFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val condition = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, x)), Flt64.zero
)
val sigmoid = SigmoidFunction(
    condition = condition,
    converter = IntoValue.Identity,
    strictBoundary = Flt64(0.1),
    conditionBounds = ConditionBounds(Flt64(-10.0), Flt64(10.0)),
    name = "sigmoid"
)
val value = sigmoid.evaluate(mapOf<Symbol, Flt64>(x to Flt64.one))
check(value == Flt64.one)
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{
    ConditionBounds, ConditionRelation, SigmoidFunction, SigmoidStepFunction,
};

let condition = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let step = SigmoidStepFunction::from_parts(
    condition.clone(),
    ConditionRelation::Greater,
    0.1_f64,
    ConditionBounds { lower: -10.0, upper: 10.0 },
)
.unwrap();
assert_eq!(step.evaluate(&1.0).unwrap(), Some(1.0));

let smooth = SigmoidFunction::new(2, "sigmoid", condition);
let _smooth_result = smooth.result_variable();
```

:::

- Core regression: [`ConditionalFunctionRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/ConditionalFunctionRegressionTest.kt)
- Core registration test: [`FunctionSymbolConditionalGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolConditionalGenericRegistrationTest.kt)
- Example directory (no dedicated sigmoid file): [linear_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function)

Rust sources: [`sigmoid.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/sigmoid.rs), [`conditional.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/conditional.rs), and [`conditional_function_solver_regression.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/conditional_function_solver_regression.rs).

## Related pages

- [Imply](./imply)
- [Conditional IF](./if)
- [Conditional If-Then](./if-then)
