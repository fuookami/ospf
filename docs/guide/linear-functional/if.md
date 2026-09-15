# Conditional IF

## Contract

`IfFunction<V>` evaluates a linear condition against zero and exposes a binary result. For a condition difference $d$, the result is `1` on the relation's true branch and `0` on its false branch. The supported relations are `GT`, `GE`, `LT`, and `LE`; `EQ` and `NE` are rejected by the shared classifier.

The condition is a `LinearPolynomial<V>`, not a pre-built Boolean expression. `IfFunction` is generic over `V : RealNumber<V> & NumberField<V>`.

## Definition and truth table

The condition polynomial is interpreted as $d = \mathrm{lhs}-\mathrm{rhs}$, or simply as the supplied difference polynomial. Let $g$ be `strictBoundary`:

| Relation | True branch | False branch | Undefined gap |
| --- | --- | --- | --- |
| `GT` | $d\ge g$ | $d\le0$ | $0<d<g$ |
| `GE` | $d\ge0$ | $d\le-g$ | $-g<d<0$ |
| `LT` | $d\le-g$ | $d\ge0$ | $-g<d<0$ |
| `LE` | $d\le0$ | $d\ge g$ | $0<d<g$ |

The result is:

$$
y = \begin{cases}
1, & \text{true branch} \\
0, & \text{false branch} \\
\text{undefined}, & \text{inside the gap}
\end{cases}
$$

## Boundary, tolerance, and Undefined

`classify` returns `TruthValue.True`, `TruthValue.False`, or `TruthValue.Undefined`. `evaluate()` maps the first two to `1` and `0`, and maps `Undefined`, missing input, or a failed classification to `null`.

`strictBoundary` defaults to the compatibility `tolerance`, which defaults to `NONZERO_TOLERANCE = 1e-10`. `delta` defaults to `strictBoundary` and is used when a discrete condition is normalized for constraints. Both must be finite, representable, and positive.

Registration requires a finite closed `ConditionBounds(lower, upper)`, either explicitly through `conditionBounds`/`bounds` or inferred from `condition.finiteBounds(converter)`. The legacy `bigM` parameter is checked for compatibility but cannot replace these bounds. If the supplied range covers only one branch, the implementation folds the indicator and result to a fixed value.

## Current API

### Kotlin

Source: [`If.kt` (`IfFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/If.kt)

```kotlin
IfFunction(
    condition: LinearPolynomial<V>,
    converter: IntoValue<V>,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    name: String = "if",
    displayName: String? = null,
    relation: Comparison = Comparison.GT,
    bounds: ConditionBounds<V>? = null,
    conditionBounds: ConditionBounds<V>? = null,
    delta: V? = null
)
```

The companion `invoke` has the same condition parameters. `IfFunction.from` accepts a `LinearConstraintInput<V>`, extracts its flattened difference polynomial, preserves its comparison relation, and returns a `LinearFunctionSymbolAdapter<V>`.

### Rust

Source: [`if_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/if_function.rs)

Rust has a same-named helper, but it is not a one-to-one replacement for Kotlin's relation classifier. Rust `IfFunction` is a ternary value selector: it tests whether `condition` is nonzero and returns `then_expr` or `else_expr`; it has no `Comparison`, `strictBoundary`, `ConditionBounds`, or `Undefined` gap. The selector uses an internal `16 * f64::EPSILON` zero test. For Kotlin-style `0/1` relation indicators, use `ConditionalIndicatorFunction::new` with `ConditionRelation`, a positive strict boundary, and finite `ConditionBounds`, then compose its result variable with the desired expression.

```rust
IfFunction::new(
    id: u64,
    name: &str,
    condition: Linear<V>,
    then_expr: Linear<V>,
    else_expr: Linear<V>,
) -> Self
IfFunction::named(
    name: impl AsRef<str>,
    condition: Linear<V>,
    then_expr: Linear<V>,
    else_expr: Linear<V>,
) -> Self
IfFunction::condition_indicator_variable(&self) -> &BinaryVariableItem
IfFunction::result_variable(&self) -> &ContinuousVariableItem
```

## Solver mathematical model

For normalized condition $q$, true threshold $T$, false threshold $F$, finite range $L\le q\le U$, and binary indicator $a$, Kotlin passes

$$
q+(L-T)a\ge L,
\qquad
q+(F-U)a\le F,
\qquad
y-a=0.
$$

Here $y$ is `name_if`; the first two rows imply $a=1\Rightarrow q\ge T$ and $a=0\Rightarrow q\le F$. Rust's same-named ternary selector instead uses a nonzero condition flag $a$ and gates $y=t$ when $a=1$ and $y=e$ when $a=0$ with four Big-M bounds. Rust `ConditionalIndicatorFunction` is the direct counterpart of the three Kotlin rows above.

## `evaluate()` versus the solver model

The direct evaluator classifies one supplied value; it can return `null` for the gap. The solver has to represent the entire declared range, so a value in the gap has no binary branch and can make the model infeasible. The old page formula that uses the maximum input value for every out-of-range case is not the current implementation.

## Minimal current example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ConditionBounds
import fuookami.ospf.kotlin.core.symbol.function.IfFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

fun main() {
    val x = RealVar("x")
    val condition = LinearPolynomial(
        monomials = listOf(LinearMonomial(Flt64.one, x)),
        constant = Flt64(-2.0)
    )
    val function = IfFunction(
        condition = condition,
        converter = IntoValue.Identity,
        relation = Comparison.GT,
        strictBoundary = Flt64(0.1),
        conditionBounds = ConditionBounds(Flt64(-2.0), Flt64(3.0)),
        name = "if"
    )

    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(3.0))) == Flt64.one)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(1.0))) == Flt64.zero)
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::IfFunction;
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::token::VecTokenList;

let function = IfFunction::named(
    "if",
    Linear::new(vec![], 1.0),
    Linear::new(vec![], 7.0),
    Linear::new(vec![], 0.0),
);
let value = <IfFunction as FunctionSymbol>::calculate_value(
    &function,
    &VecTokenList::<f64>::new(),
    false,
);
assert_eq!(value, Some(7.0));
```

:::

## Source and core tests

- [Implementation: `If.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/If.kt)
- [Core conditional regression test: `ConditionalFunctionRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/ConditionalFunctionRegressionTest.kt)
- [Core generic registration test: `FunctionSymbolGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolGenericRegistrationTest.kt)
- [Complete example: `ConditionalFunctionSolveTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/ConditionalFunctionSolveTest.kt)
- [Rust implementation and unit tests: `if_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/if_function.rs)
- [Rust range-driven conditional regression: `conditional_function_solver_regression.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/conditional_function_solver_regression.rs)

## Related pages

- [Interval condition](/guide/linear-functional/if-in)
- [If-Then](/guide/linear-functional/if-then)
- [Binaryzation](/guide/linear-functional/bin)
- [One-of constraint](/guide/linear-functional/one-of)
