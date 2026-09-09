# Conditional interval

## Contract

`IfInFunction<V>` returns a binary interval-membership result for one linear polynomial and two scalar endpoints. The intended interval is closed: the result is `1` when `lower <= x <= upper`, and `0` when the value is safely outside the interval.

The implementation is generic over `V : RealNumber<V> & NumberField<V>`. It requires `lower <= upper` and a finite range for `x` when constraints are registered.

## Definition and three-valued interval semantics

Define the two differences:

$$
d_\mathrm{lower}=x-\mathrm{lower},
\qquad
d_\mathrm{upper}=\mathrm{upper}-x.
$$

Both differences are classified with the shared `GE` relation. For strict boundary $g$:

| Position of $x$ | Lower side | Upper side | Result |
| --- | --- | --- | --- |
| $x\le\mathrm{lower}-g$ | False | True or Undefined | 0 |
| $\mathrm{lower}-g<x<\mathrm{lower}$ | Undefined | True | Undefined |
| $\mathrm{lower}\le x\le\mathrm{upper}$ | True | True | 1 |
| $\mathrm{upper}<x<\mathrm{upper}+g$ | True | Undefined | Undefined |
| $x\ge\mathrm{upper}+g$ | True or Undefined | False | 0 |

At either endpoint, both closed-interval comparisons are true. If `lower == upper`, the single endpoint is still a true point.

## Boundary, tolerance, and Undefined

`classify(values)` returns `TruthValue.False` if either side is false, `TruthValue.True` if both sides are true, and `TruthValue.Undefined` otherwise. `evaluate()` maps true/false to `1`/`0` and maps undefined, missing input, or failed validation to `null`.

`strictBoundary` defaults to compatibility `tolerance`, whose default is `NONZERO_TOLERANCE = 1e-10`. `delta` defaults to `strictBoundary`. These parameters describe the discrete condition gap; they do not turn the open outside bands into true interval members.

## Current API

### Kotlin

Source: [`IfIn.kt` (`IfInFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/IfIn.kt)

```kotlin
IfInFunction(
    x: LinearPolynomial<V>,
    lower: V,
    upper: V,
    converter: IntoValue<V>,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    name: String = "ifin",
    displayName: String? = null,
    bounds: ConditionBounds<V>? = null,
    conditionBounds: ConditionBounds<V>? = null,
    delta: V? = null
)
```

The companion `invoke` exposes the same arguments. `bounds` and `conditionBounds` are aliases for the finite range of `x`; `bigM` is retained for compatibility and cannot replace that range.

### Rust

Source: [`if_in.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/if_in.rs)

Rust's same-named `IfInFunction` is not the Kotlin closed-interval API: it tests approximate membership in a discrete `values` set and takes an explicit `big_m`. The closest interval composition is `ConditionalIfFunction` for each side, combined by `IfInRangeFunction`; call `IfInRangeFunction::registerable(id, name)` to obtain a `RegisterableIfInRangeFunction` with registered indicators and an AND result. The range validator requires `GreaterEqual` side relations, opposite-signed one-variable conditions, ordered endpoints, and finite bounds.

```rust
IfInFunction::new(
    id: u64,
    name: &str,
    input: Linear<V>,
    values: Vec<V>,
    big_m: V,
) -> Self
ConditionalIfFunction::new(
    condition: Linear<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    bounds: ConditionBounds<V>,
) -> Result<Self>
IfInRangeFunction::new(
    lower: ConditionalIfFunction<V>,
    upper: ConditionalIfFunction<V>,
) -> Result<Self>
IfInRangeFunction::registerable(
    self,
    id: u64,
    name: impl AsRef<str>,
) -> Result<RegisterableIfInRangeFunction<V>>
```

## Auxiliary variables and registration model

For `name`, the implementation creates `name_ifin` as the result, `name_ge` for `x - lower >= 0`, and `name_le` for `upper - x >= 0`. All three are binary variables in `helperVariables`; `resultPolynomial` is the unit-coefficient polynomial of the result.

`registerAuxiliaryTokens` validates the endpoints, condition polynomial, finite x bounds, `strictBoundary`, and `delta`, then adds the helper variables. `registerConstraints` normalizes both `GE` conditions, folds fixed branches when the range proves them, and otherwise links the result to the conjunction of the two side indicators.

## `evaluate()` versus the solver model

The direct evaluator uses the exact evaluated `x` together with the three-valued gap. The solver uses range-driven indicator constraints; if the declared x range intersects an undefined outside band, the resulting model may be infeasible. An explicit finite `ConditionBounds` is part of the solver contract even though direct evaluation only needs values.

## Minimal current example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ConditionBounds
import fuookami.ospf.kotlin.core.symbol.function.IfInFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

fun main() {
    val x = RealVar("x")
    val xPoly = LinearPolynomial(
        monomials = listOf(LinearMonomial(Flt64.one, x)),
        constant = Flt64.zero
    )
    val function = IfInFunction(
        x = xPoly,
        lower = Flt64.zero,
        upper = Flt64(2.0),
        converter = IntoValue.Identity,
        strictBoundary = Flt64(0.1),
        conditionBounds = ConditionBounds(Flt64(-1.0), Flt64(3.0)),
        name = "ifin"
    )

    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64.one)) == Flt64.one)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(-1.0))) == Flt64.zero)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(-0.05))) == null)
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{
    ConditionBounds, ConditionRelation, ConditionalIfFunction, IfInRangeFunction,
};

let bounds = ConditionBounds {
    lower: -2.0,
    upper: 2.0,
};
let lower = ConditionalIfFunction::new(
    Linear::new(vec![LinearMonomial::new(1.0, 0)], 1.0),
    ConditionRelation::GreaterEqual,
    0.1,
    bounds.clone(),
)
.expect("valid lower interval condition");
let upper = ConditionalIfFunction::new(
    Linear::new(vec![LinearMonomial::new(-1.0, 0)], 1.0),
    ConditionRelation::GreaterEqual,
    0.1,
    bounds,
)
.expect("valid upper interval condition");
let range = IfInRangeFunction::new(lower, upper).expect("valid closed interval");
let value = range.evaluate(&0.0, &0.0).expect("classifiable interval");
assert_eq!(value, Some(1.0));
```

:::

## Source and core tests

- [Implementation: `IfIn.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/IfIn.kt)
- [Core conditional registration test: `FunctionSymbolConditionalGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolConditionalGenericRegistrationTest.kt)
- [Core conditional regression test: `ConditionalFunctionRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/ConditionalFunctionRegressionTest.kt)
- [Complete example: `ConditionalFunctionSolveRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/ConditionalFunctionSolveRegressionTest.kt)
- [Rust implementation and unit tests: `if_in.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/if_in.rs)
- [Rust conditional regression: `conditional_function_solver_regression.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/conditional_function_solver_regression.rs)
- [Rust registration atomicity and interval validation: `registration_atomicity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/registration_atomicity.rs)

## Related pages

- [Conditional IF](/guide/linear-functional/if)
- [If-Then](/guide/linear-functional/if-then)
- [One-of constraint](/guide/linear-functional/one-of)
