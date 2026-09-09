# Quadratic Minimum

`QuadraticMinFunction` computes the minimum of a list of quadratic polynomials and offers an exact selector mode or a lower-envelope relaxation.

## Contract

- Input: `polynomials: List<QuadraticPolynomial<V>>`.
- Output/helper: real `resultVar` named by appending `_min` to `name`.
- Direct evaluation returns the minimum; missing symbols or an empty candidate list result in `null`.
- `exact = true` creates one binary selector per candidate and aims to enforce exact minimum; `exact = false` registers only $y\le p_i$ constraints.
- Generic values require `V : RealNumber<V>, V : Ring<V>, V : NumberField<V>` and an `IntoValue<V>` converter.

> [!WARNING]
> `exact = false` is a lower-envelope relaxation. Without an objective or another constraint pushing y upward, it need not equal the mathematical minimum.

## Definition and mathematical model

For candidates $p_i$ and result y, both modes add

$$
y\le p_i\qquad(i=0,\ldots,n-1).
$$

Exact mode additionally creates $u_i\in\{0,1\}$:

$$
y\ge p_i-M_i(1-u_i),\qquad \sum_i u_i=1.
$$

The direct evaluator always computes $\min_i p_i$, independent of `exact`.

## Implementation, helper variables, and constraints

The implementation creates `name` + `_min` as a real result variable. In exact mode it creates `name` + `_u_` + index binary selectors, infers each Big-M from candidate bounds when possible, and registers the upper/lower selector constraints and exactly-one equality. Relaxed mode omits selector variables and the Big-M lower constraints.

## Current API

### Kotlin

Source: [`QuadraticMin.kt` (`QuadraticMinFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticMin.kt#L40-L390)

```kotlin
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticMinFunction
import fuookami.ospf.kotlin.core.token.AutoTokenTable
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Quadratic
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val y = RealVar("y")
val first = QuadraticPolynomial(
    listOf(QuadraticMonomial.quadratic(Flt64.one, x, y)), Flt64.one
)
val second = QuadraticPolynomial(
    listOf(QuadraticMonomial.linear(Flt64.one, x)), Flt64.two
)
val minimum = QuadraticMinFunction(
    polynomials = listOf(first, second),
    exact = true,
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "quadratic_min"
)
val tokens = AutoTokenTable<Flt64>(Quadratic, false)
tokens.add(listOf(x, y))
val value = minimum.prepare(
    mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0)),
    tokens,
    IntoValue.Identity
)
check(value == Flt64(4.0))
tokens.close()
```

### Rust

Rust exposes [`QuadraticMinFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_min.rs):

```rust
QuadraticMinFunction::new(
    id: u64,
    name: &str,
    inputs: Vec<Quadratic<V>>,
    exact: bool,
) -> QuadraticMinFunction<V>
```

`result_variable` returns the inner `name + "_min"` variable and `with_declared_dependencies` preserves explicit dependency IDs. Each input is bridged by `QuadraticLinearFunction`; `exact = true` creates the inner binary selectors, while `exact = false` keeps only the lower-envelope upper inequalities. `calculate_value` always computes the mathematical minimum. When token bounds are available, `mechanism_constraints_with_tokens` infers Big-M from the original quadratic candidates; otherwise the generic fallback policy is used. Rust has no `bigM` constructor argument on this type.

```rust
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticMinFunction;

let first = Quadratic::new(
    vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)],
    1.0,
);
let second = Quadratic::new(
    vec![QuadraticMonomial::new_linear(1.0, 0)],
    2.0,
);
let minimum = QuadraticMinFunction::new(15, "quadratic_min", vec![first, second], true);
assert!(minimum.result_variable().name().contains("quadratic_min_min"));
```

## Evaluate versus solver

Direct evaluation is always the exact minimum. Exact solver mode needs valid Big-M ranges for all candidates; relaxed mode only guarantees an upper bound on y for every candidate. A non-negative result variable can also conflict with a negative mathematical minimum, so compare the candidate domains with the variable domains before registration.

## Boundaries, tolerance, and Undefined

The input list should be non-empty; an empty list makes the direct minimum null and provides no meaningful solver model. Missing values return `null`. There is no tolerance or three-valued Undefined state. Invalid or insufficient Big-M values can make exact registration fail or weaken it.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticMinFunction
import fuookami.ospf.kotlin.core.token.AutoTokenTable
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Quadratic
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val y = RealVar("y")
val first = QuadraticPolynomial(
    listOf(QuadraticMonomial.quadratic(Flt64.one, x, y)), Flt64.one
)
val second = QuadraticPolynomial(
    listOf(QuadraticMonomial.linear(Flt64.one, x)), Flt64.two
)
val minimum = QuadraticMinFunction(
    polynomials = listOf(first, second),
    exact = true,
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "quadratic_min"
)
val tokens = AutoTokenTable<Flt64>(Quadratic, false)
tokens.add(listOf(x, y))
val value = minimum.prepare(
    mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0)),
    tokens,
    IntoValue.Identity
)
check(value == Flt64(4.0))
tokens.close()
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticMinFunction;
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
let minimum = QuadraticMinFunction::new(
    16,
    "quadratic_min",
    vec![
        Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 1.0),
        Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 2.0),
    ],
    true,
);
assert_eq!(minimum.calculate_value(&tokens, false), Some(4.0));
```

:::

- Core evaluation: [`QuadraticFunctionGenericEvaluationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionGenericEvaluationTest.kt)
- Core registration: [`FunctionSymbolGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolGenericRegistrationTest.kt)
- Example directory (no dedicated quadratic-min file): [quadratic_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/quadratic_function)

- Rust implementation and focused tests: [`quadratic_min.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_min.rs) and [`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)

## Related pages

- [Maximum](../linear-functional/max)
- [Minimum](../linear-functional/min)
- [Quadratic Linear](./quadratic-linear)
