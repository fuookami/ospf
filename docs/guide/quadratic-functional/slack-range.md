# Slack Range in a Quadratic Model

`QuadraticSlackRangeFunction` is the exact distance from a quadratic
expression to the closed interval $[lower,upper]$:

$$
s=\max(lower-p(x),\ p(x)-upper,\ 0).
$$

The constructor requires `lower <= upper`.

## Solver mathematical model

For $a=lower-p(x)$ and $b=p(x)-upper$, the implementation registers exact
positive-part encodings for $a$ and $b$ and returns their sum. Equivalently,
with $s\ge0$, binary selectors $z_0,z_1,z_2$, and valid bounds $M_i$:

$$
\begin{aligned}
s&\ge a,&s&\ge b,&s&\ge0,\\
s&\le a+M_0(1-z_0),&s&\le b+M_1(1-z_1),&s&\le M_2(1-z_2),\\
z_0+z_1+z_2&=1.
\end{aligned}
$$

Thus the result is zero inside the interval and is the exact one-sided
violation outside it.

## Kotlin API

Kotlin provides a native quadratic symbol:

```kotlin
val slack = QuadraticSlackRangeFunction(
    input = inputQuadratic,
    lower = Flt64(1.0),
    upper = Flt64(2.0),
    bigM = Flt64(100.0),
    converter = IntoValue.Identity,
    name = "quadratic-slack-range"
)
```

For a linear expression, use `SlackRangeFunction(input, lower, upper,
bigM, converter, name)`. It has the same exact max formulation and exposes
the result polynomial of the internal `MaxFunction`.

## Rust API

```rust
let slack = QuadraticSlackRangeFunction::new(
    23, "quadratic_slack_range", input, 1.0_f64, 2.0_f64,
);
assert_eq!(slack.calculate_value(&tokens, false), Some(1.0));
```

The exact linear range-distance formulation is fed by the original linear
input when it has no quadratic terms. A genuinely quadratic input receives
one signed linear bridge variable and one quadratic bridge equality first.
`with_big_m` can provide an explicit Big-M; otherwise registered token bounds
are used when available. `result_variable()` is continuous.

## Kotlin/Rust example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticSlackRangeFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64

val slack = QuadraticSlackRangeFunction(
    input = inputQuadratic,
    lower = Flt64(1.0),
    upper = Flt64(2.0),
    bigM = Flt64(100.0),
    converter = IntoValue.Identity,
    name = "quadratic-slack-range"
)
check(slack.evaluate(values) == Flt64(1.0))
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::function::QuadraticSlackRangeFunction;

let slack = QuadraticSlackRangeFunction::new(
    23, "quadratic_slack_range", input, 1.0_f64, 2.0_f64,
);
assert_eq!(slack.calculate_value(&tokens, false), Some(1.0));
```

:::

## Tests and references

- Kotlin: [`QuadraticSlackRangeFunctionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticSlackRangeFunctionTest.kt)
- Kotlin implementation: [`QuadraticSlackRange.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticSlackRange.kt)
- Rust implementation: [`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)
- Rust focused helper/row test: [`function_symbol_quadratic_slack_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_slack_range.rs)
- Rust linear range implementation: [`slack_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/slack_range.rs)
