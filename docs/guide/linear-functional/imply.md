# Implication

`ImplyFunction` models the implication `A ⇒ B` for two linear expressions interpreted as relations to zero. It uses the shared discrete-condition classifier and three-valued boundary semantics.

> [!WARNING]
> `ImplyFunction` is not a generic Boolean algebra over arbitrary solver symbols. Registration requires finite, ordered condition bounds; the legacy `bigM` parameter is validated when supplied but cannot replace those bounds.

## Contract

- `antecedent` and `consequent` are `LinearPolynomial<V>` values.
- `relation` (default `Comparison.GT`) is applied to each polynomial against zero.
- `evaluate` returns one for true, zero for false, and `null` for an undefined condition or missing/invalid required input.
- `strictBoundary` is the true/false separation gap; `tolerance` is the compatibility parameter used when `strictBoundary` is omitted; `delta` defaults to `strictBoundary`.
- Optional `antecedentBounds` and `consequentBounds` must be finite and ordered for solver registration.

## Definition and mathematical model

Classically,

$$
A\Rightarrow B\equiv\lnot A\lor B.
$$

For each relation, the shared classifier uses a gap $g=\text{strictBoundary}$. For the default `GT` relation, a difference $d$ is true when $d\ge g$, false when $d\le0$, and undefined when $0<d<g$. The analogous matrix is used for `GE`, `LT`, and `LE`.

The implication result is:

| Antecedent | Consequent | Result |
| --- | --- | --- |
| False | not inspected | True |
| True | True | True |
| True | False | False |
| Undefined | not inspected | Undefined |

## Solver mathematical model

Let $a,b\in\{0,1\}$ be the antecedent and consequent relation flags. Each is linked to its normalized condition $q_j\in[L_j,U_j]$ by

$$
q_j+(L_j-T_j)u_j\ge L_j,
\qquad
q_j+(F_j-U_j)u_j\le F_j,
\qquad (u_0,u_1)=(a,b).
$$

Kotlin then passes the implication row

$$
a-b\le0.
$$

Thus a true antecedent forces a true consequent; a false antecedent imposes no consequent truth requirement. Rust `ConditionalImplyFunction` is the corresponding relation-based implementation; the legacy Rust same-named helper has a different contract.

## Current API

### Kotlin

Source: [`Imply.kt` (`ImplyFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Imply.kt#L229-L1148)

```kotlin
ImplyFunction(
    antecedent: LinearPolynomial<V>,
    consequent: LinearPolynomial<V>,
    converter: IntoValue<V>,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    name: String = "imply",
    displayName: String? = null,
    relation: Comparison = Comparison.GT,
    antecedentBounds: ConditionBounds<V>? = null,
    consequentBounds: ConditionBounds<V>? = null,
    delta: V? = null
)
```

### Rust

Source: [`imply.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/imply.rs)

Rust keeps a legacy `ImplyFunction` for two `LinearInequality<V>` objects and an explicit `big_m`; its direct evaluator uses ordinary Boolean inequality checks. The closest one-to-one match for Kotlin's bounded, three-valued implication is `ConditionalImplyFunction`: construct it from two `ConditionalIfFunction` descriptors (or use `from_parts`), including each relation, strict boundary, and finite bounds. A false premise short-circuits to true, while an undefined premise or reached undefined consequence maps to `None`.

```rust
ImplyFunction::new(
    id: u64,
    name: &str,
    premise: LinearInequality<V>,
    consequence: LinearInequality<V>,
    big_m: V,
) -> Self
ConditionalImplyFunction::from_parts(
    id: u64,
    name: &str,
    premise: Linear<V>,
    premise_relation: ConditionRelation,
    premise_strict_boundary: V,
    premise_bounds: ConditionBounds<V>,
    consequence: Linear<V>,
    consequence_relation: ConditionRelation,
    consequence_strict_boundary: V,
    consequence_bounds: ConditionBounds<V>,
) -> Result<Self>
ConditionalImplyFunction::evaluate(
    &self,
    premise_difference: &V,
    consequence_difference: &V,
) -> Result<Option<V>>
```

## Evaluate versus solver

Direct evaluation classifies the antecedent first. A false antecedent returns true without reading the consequent; a true antecedent classifies the consequent; an undefined antecedent returns `null`. Solver registration constructs the shared indicator constraints, performs bound validation and constant folding, and uses the binary indicators as the model result. Thus direct short-circuiting does not bypass registration-time bound errors.

## Boundaries, tolerance, and Undefined

Missing antecedent input is a classification failure. Missing consequent input matters only after a true antecedent. Values in the classifier gap are `TruthValue.Undefined` and therefore become `null` in `evaluate`. Non-finite or sentinel condition bounds, reversed bounds, non-positive `strictBoundary`/`delta`, and unsuitable Big-M values fail registration through the Result API.

## Minimal current example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ConditionBounds
import fuookami.ospf.kotlin.core.symbol.function.ImplyFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val y = RealVar("y")
val antecedent = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, x)), Flt64.zero
)
val consequent = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, y)), Flt64.zero
)
val implication = ImplyFunction(
    antecedent = antecedent,
    consequent = consequent,
    converter = IntoValue.Identity,
    strictBoundary = Flt64(0.1),
    antecedentBounds = ConditionBounds(Flt64(-10.0), Flt64(10.0)),
    consequentBounds = ConditionBounds(Flt64(-10.0), Flt64(10.0)),
    name = "imply"
)
val value = implication.evaluate(
    mapOf<Symbol, Flt64>(x to Flt64.zero, y to Flt64.zero)
)
check(value == Flt64.one)
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::{
    ConditionBounds, ConditionRelation, ConditionalImplyFunction,
};

let function = ConditionalImplyFunction::from_parts(
    1,
    "imply",
    Linear::new(vec![], 1.0),
    ConditionRelation::Greater,
    0.1,
    ConditionBounds {
        lower: -2.0,
        upper: 2.0,
    },
    Linear::new(vec![], 1.0),
    ConditionRelation::Greater,
    0.1,
    ConditionBounds {
        lower: -2.0,
        upper: 2.0,
    },
)
.expect("valid conditional implication");
let value = function
    .evaluate(&1.0, &1.0)
    .expect("classifiable implication");
assert_eq!(value, Some(1.0));
```

:::

## Tests and examples

- Core regression: [`ImplyFunctionRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/ImplyFunctionRegressionTest.kt)
- Example directory (no dedicated implication file): [linear_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function)
- Rust implementation and unit tests: [`imply.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/imply.rs)
- Rust conditional regression: [`conditional_function_solver_regression.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/conditional_function_solver_regression.rs)

## Related pages

- [Conditional IF](./if)
- [Conditional If-Then](./if-then)
- [Sigmoid](./sigmoid)
