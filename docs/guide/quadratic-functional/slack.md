# Slack in a Quadratic Model

`QuadraticSlackFunction` is the exact absolute-distance function

$$
s=|L(x)-R(x)|.
$$

Both language implementations evaluate the original quadratic expressions
and register an exact absolute-value formulation, so correctness does not
depend on minimizing the result. Rust creates a linear bridge only for a
genuinely quadratic input; purely linear inputs remain direct expressions.

## Solver mathematical model

Let $l=L(x)$, $r=R(x)$, $d=l-r$, $s\ge0$ be the result, $z\in\{0,1\}$ a
side selector, and $M$ a valid bound on $|d|$. The rows sent to the solver are

$$
\begin{aligned}
s-d&\ge0,\\
s+d&\ge0,\\
s-d+Mz&\le M,\\
s+d-Mz&\le0.
\end{aligned}
$$

They force $s=|d|$. An explicit `bigM` must bound the whole difference.

## Kotlin API

Kotlin provides a native quadratic symbol; no linear adapter is required:

```kotlin
val slack = QuadraticSlackFunction(
    left = leftQuadratic,
    right = rightQuadratic,
    bigM = Flt64(100.0),
    converter = IntoValue.Identity,
    name = "quadratic-slack"
)
```

The implementation is the sum of two exact positive-part functions for
$L-R$ and $R-L$; `evaluate` returns the absolute difference and registration
registers both delegates.

## Rust API

```rust
let slack = QuadraticSlackFunction::with_big_m(
    21, "quadratic_slack", left, right, 100.0_f64,
);
assert_eq!(slack.calculate_value(&tokens, false), Some(1.0));
```

`new` uses the default Big-M policy and `with_target` uses a constant
right-hand side. `with_big_m` overrides the policy. The result variable is
continuous; the mechanism registers the four absolute-value rows and adds a
quadratic bridge row only for each genuinely quadratic input.

## Kotlin/Rust example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticSlackFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64

val slack = QuadraticSlackFunction(
    left = leftQuadratic,
    right = rightQuadratic,
    bigM = Flt64(100.0),
    converter = IntoValue.Identity,
    name = "quadratic-slack"
)
check(slack.evaluate(values) == Flt64(1.0))
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::function::QuadraticSlackFunction;

let slack = QuadraticSlackFunction::with_big_m(21, "quadratic_slack", left, right, 100.0_f64);
assert_eq!(slack.calculate_value(&tokens, false), Some(1.0));
```

:::

## Tests and references

- Kotlin: [`QuadraticSlackFunctionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticSlackFunctionTest.kt)
- Kotlin implementation: [`QuadraticSlack.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticSlack.kt)
- Rust implementation: [`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)
- Rust test: [`function_symbol_quadratic_slack.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_slack.rs)
