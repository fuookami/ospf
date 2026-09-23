# Quadratic Maximum

`QuadraticMaxFunction` computes the maximum of a list of quadratic polynomials:

$$
y=\max(p_1,p_2,\ldots,p_n).
$$

It shares the composition architecture of `QuadraticMinFunction`: every input containing quadratic terms is bound to a bridge variable by one exact quadratic equality, and the linear `MaxFunction` selector model is applied to the bound inputs.

## Contract

- Input: `polynomials: List<QuadraticPolynomial<V>>`; registration rejects an empty list, and every candidate needs inferable finite bounds.
- Output/helper: each genuinely quadratic input gets a bridge `RealVar` named `${name}_input_$index`; the result is the inner function's real variable `${name}_max`, exposed through `polynomial`.
- Direct evaluation returns the maximum; missing symbols or an empty candidate list result in `null`.
- Kotlin always registers the exact selector form; Rust exposes an `exact` flag, and `exact = false` registers only the $y\ge p_i$ rows.
- Generic values require `V : RealNumber<V>, V : NumberField<V>` and an `IntoValue<V>` converter.

> [!WARNING]
> The registered formulation may be nonconvex MIQCP — each quadratic candidate is pinned by a variable-equality row — and requires a solver supporting nonconvex quadratic constraints. Rust's `exact = false` is an upper-envelope relaxation: without minimization or another constraint pushing y downward, it need not equal the mathematical maximum.

## Solver mathematical model

### Kotlin

Every input containing quadratic terms (or a quadratic intermediate symbol) is bound first: a signed continuous bridge $b_i\in\mathbb R$ named `${name}_input_$i` is created and pinned by one exact quadratic equality

$$
p_i(x)-b_i=0.
$$

The bridge range is tightened to the input's finite bounds, widening them to the next representable double when a captured bound is not float-representable. Affine inputs pass through without a bridge. Registration validates finite, un-widened bounds: widening a bound after it has been captured is rejected, while tightening remains safe.

The linear maximum model is then applied to the bound inputs; see [Maximum](/guide/linear-functional/max) for the full row discussion. With result $y$ — the inner function's `${name}_max` variable — and selectors $s_i\in\{0,1\}$:

$$
y\ge p_i,\qquad
y-p_i+M_i s_i\le M_i,
\qquad
\sum_i s_i=1.
$$

An explicit `bigM` takes precedence; otherwise candidate bounds are inferred, falling back to each candidate's default Big-M. When every candidate has finite bounds, the result range is tightened to the candidate bounds, so a negative maximum is representable.

### Rust

Rust creates a `QuadraticLinearFunction` bridge named `{name}_bridge{i}` for every candidate and applies the inner `MaxFunction` to the bound inputs. Purely linear candidates remain expression-only — the bridge registers a helper token only for genuine quadratic inputs — while quadratic candidates contribute one bridge equality plus the maximum rows:

$$
y\ge b_i,
$$

and, when `exact = true`:

$$
y-b_i+M_i s_i\le M_i,
\qquad
s_i\in\{0,1\},
\qquad
\sum_i s_i=1.
$$

With `exact = false`, only the $y\ge b_i$ rows are registered. When token bounds are available, `mechanism_constraints_with_tokens` infers Big-M from the original quadratic candidates; otherwise the inner function's generic fallback policy is used.

## Current API

### Kotlin

Source: [`QuadraticMax.kt` (`QuadraticMaxFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticMax.kt)

```kotlin
QuadraticMaxFunction(
    polynomials: List<QuadraticPolynomial<V>>,
    bigM: V? = null,
    converter: IntoValue<V>,
    name: String = "quadratic_max",
    displayName: String? = null
)
```

The constructor has no `exact` flag; Kotlin always registers the exact selector model. `createFunction` delegates to `MaxFunction` with the same `name`, `bigM`, `converter`, and `displayName`. There is no single-map `evaluate(values)` overload; call `evaluate(values, tokenTable, converter, zeroIfNone = false)`:

```kotlin
val value = maximum.evaluate(
    mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0)),
    null,
    IntoValue.Identity,
    zeroIfNone = false
)
```

### Rust

Rust exposes [`QuadraticMaxFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs) in `quadratic_function.rs`:

```rust
QuadraticMaxFunction::new(
    id: u64,
    name: &str,
    inputs: Vec<Quadratic<V>>,
    exact: bool,
) -> QuadraticMaxFunction<V>
```

`result_variable()` returns the inner `name + "_max"` continuous variable and `with_declared_dependencies` preserves explicit dependency IDs. Every candidate is wrapped by a `QuadraticLinearFunction` bridge named `name + "_bridge" + i`; `exact = true` creates the inner binary selectors, while `exact = false` keeps only the lower-envelope rows. `calculate_value` always computes the mathematical maximum. When token bounds are available, `mechanism_constraints_with_tokens` infers Big-M from the original quadratic candidates; otherwise the generic fallback policy is used. Rust has no `bigM` constructor argument on this type.

## Evaluate versus solver

Direct evaluation resolves the original input symbols, feeds the bridge variables, and delegates to the linear `MaxFunction`; it always computes the exact maximum, independent of `exact`, and returns `null` when a symbol is missing or the candidate list is empty. `prepare(values, tokenTable, converter)` forwards to the same evaluation with `zeroIfNone = false`.

Solver registration adds the bridge equalities and the selector model, tightens bridge and result ranges to the candidate bounds, and relies on valid Big-M values; an undersized `bigM` can make the solver model infeasible despite a valid direct evaluation. The public `polynomial` is the linear counterpart's affine result lifted to a quadratic polynomial, and `helperVariables` are the bridges plus the inner function's result and selector variables.

## Minimal example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticMaxFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial

val x = RealVar("x")
val y = RealVar("y")
val first = QuadraticPolynomial(
    listOf(QuadraticMonomial.quadratic(Flt64.one, x, y)), Flt64.one
)
val second = QuadraticPolynomial(
    listOf(QuadraticMonomial.linear(Flt64.one, x)), Flt64.two
)
val maximum = QuadraticMaxFunction(
    polynomials = listOf(first, second),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "quadratic_max"
)
val value = maximum.evaluate(
    mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0)),
    null,
    IntoValue.Identity,
    zeroIfNone = false
)
check(value == Flt64(11.0))
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticMaxFunction;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
let y = ContinuousVariableItem::create(VariableId::standalone(1), "y");
let mut tokens = VecTokenList::<f64>::new();
let tx = Token::from_generic(x, 0);
tx.set_result(2.0);
tokens.add_token(tx);
let ty = Token::from_generic(y, 1);
ty.set_result(5.0);
tokens.add_token(ty);
let maximum = QuadraticMaxFunction::new(
    17,
    "quadratic_max",
    vec![
        Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 1.0),
        Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 2.0),
    ],
    true,
);
assert_eq!(maximum.calculate_value(&tokens, false), Some(11.0));
```

:::

## QuadraticMaxMinFunction and QuadraticMinMaxFunction

$$
MaxMin(p_1,\ldots,p_n)=\min_i p_i,\qquad
MinMax(p_1,\ldots,p_n)=\max_i p_i.
$$

Despite their names, `QuadraticMinMaxFunction` computes the maximum — the exact extremum for min-max objectives — and `QuadraticMaxMinFunction` computes the minimum — the exact extremum for max-min objectives, useful when maximizing the worst candidate. The names describe the optimization interpretation, not a different aggregation algorithm. Both extend `QuadraticFunctionSymbol` and delegate every bridge, helper-variable, and constraint operation to their linear counterparts: `QuadraticMaxMinFunction` wraps an inner `MaxMinFunction` (itself a wrapper over `MinFunction`), and `QuadraticMinMaxFunction` wraps an inner `MinMaxFunction` (itself a wrapper over `MaxFunction`). Both wrappers accept the same `polynomials`, optional `bigM`, `converter`, `name`, and optional `displayName` parameters, with the default names `quadratic_maxmin` and `quadratic_minmax`. Both extrema are exact and independent of the objective direction.

Source: [`QuadraticMaxMin.kt` (`QuadraticMaxMinFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticMaxMin.kt) and [`QuadraticMinMax.kt` (`QuadraticMinMaxFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticMinMax.kt)

```kotlin
val maxMin = QuadraticMaxMinFunction(
    polynomials = listOf(first, second),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "quadratic_maxmin"
)
val minMax = QuadraticMinMaxFunction(
    polynomials = listOf(first, second),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "quadratic_minmax"
)
```

Rust now provides the same two wrappers in [`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs):

```rust
QuadraticMaxMinFunction::new(
    id: u64,
    name: &str,
    inputs: Vec<Quadratic<V>>,
) -> QuadraticMaxMinFunction<V>
QuadraticMaxMinFunction::with_declared_dependencies(
    self,
    dependency_ids: Vec<u64>,
) -> Self
QuadraticMinMaxFunction::new(
    id: u64,
    name: &str,
    inputs: Vec<Quadratic<V>>,
) -> QuadraticMinMaxFunction<V>
QuadraticMinMaxFunction::with_declared_dependencies(
    self,
    dependency_ids: Vec<u64>,
) -> Self
```

Unlike the Kotlin constructors, the Rust ones take no `bigM`, `converter`, or `displayName` arguments. Each quadratic candidate gets its own `QuadraticLinearFunction` bridge named `{name}_bridge{i}` — the exact quadratic equality row — while pure-linear candidates stay expression-only. The linear side reuses the exact delegation target of the linear shell: `QuadraticMaxMinFunction` wraps the exact `MinFunction` and registers rows named `{name}_min_*`, and `QuadraticMinMaxFunction` wraps the exact `MaxFunction` and registers rows named `{name}_max_*`. Big-M is inferred from the original candidate token bounds, floored at the minimum Big-M; without usable token bounds the inner function's generic fallback policy is used. Direct evaluation folds `min`/`max` over the original quadratic candidates. As with the Kotlin delegation chains, both extrema are exact and independent of the objective direction.

The linear [Minimum](/guide/linear-functional/min) page's `MinMaxFunction and MaxMinFunction` section explains the same naming pitfall on the linear side. The bridge composition — one bounded real variable and one exact quadratic equality per quadratic input — is also what keeps every candidate quadratic instead of expanding it into cubic or quartic terms.

## Tests and references

- Kotlin composition, evaluation, and registration coverage for all three symbols: [`QuadraticFunctionCompositionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionCompositionTest.kt). The generic evaluation and registration suites cover the Min counterpart but not the Max family.
- Rust implementation with in-file regression tests (`quadratic_min_max_calculate_value`, `quadratic_max_infers_big_m_from_original_candidate_bounds`, and `quadratic_max_min_and_min_max_calculate_value`): [`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)
- Rust dedicated contract test: [`function_symbol_quadratic_max.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_max.rs)
- Rust dedicated contract tests for the MaxMin/MinMax wrappers: [`function_symbol_quadratic_max_min.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_max_min.rs) and [`function_symbol_quadratic_min_max.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_min_max.rs)
- Rust end-to-end solver coverage for the MaxMin/MinMax wrappers: [`gurobi_quadratic_model_integration.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_quadratic_model_integration.rs) (`gurobi_solves_quadratic_max_min_with_non_linear_input`, `gurobi_solves_quadratic_min_max_with_non_linear_input`)

## Related pages

- [Quadratic Minimum](./quadratic-min)
- [Maximum](/guide/linear-functional/max)
- [Minimum](/guide/linear-functional/min)
