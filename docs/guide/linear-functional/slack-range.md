# Slack Range

`SlackRangeFunction` computes the exact distance of a linear expression from
the closed interval $[lower,upper]$:

$$
s=\max(lower-x,\ x-upper,\ 0).
$$

The constructor validates `lower <= upper` and accepts scalar bounds:

```kotlin
SlackRangeFunction(
    input: LinearPolynomial<V>,
    lower: V,
    upper: V,
    bigM: V? = null,
    converter: IntoValue<V>,
    name: String,
    displayName: String? = null
)
```

`fromLinearIntermediateSymbol` creates the adapter form when the input is a
linear intermediate symbol. The result polynomial is the internal exact
`MaxFunction` result, so it cannot be inflated by leaving it out of the
objective.

## Solver mathematical model

For candidates $p_0=lower-x$, $p_1=x-upper$, and $p_2=0$, with result $s$,
binary selectors $z_i$, and valid bounds $M_i$, the implementation registers

$$
\begin{aligned}
s-p_i&\ge0,\\
s-p_i+M_i z_i&\le M_i\quad(i=0,1,2),\\
z_0+z_1+z_2&=1.
\end{aligned}
$$

This is the exact maximum formulation: $s=0$ inside the interval, and
$s=lower-x$ or $s=x-upper$ outside it.

## Rust parity

Rust exposes the same scalar-bound API:

```rust
let slack = SlackRangeFunction::new(
    1, "slack_range", input, -2.0_f64, 2.0_f64,
);
assert_eq!(slack.lower_bound(), &-2.0);
assert_eq!(slack.upper_bound(), &2.0);
```

Its `MaxFunction` encoding and direct evaluation implement the same formula.

## Kotlin/Rust example

::: code-group

```kotlin [Kotlin]
val slack = SlackRangeFunction(
    input = input,
    lower = Flt64(-2.0),
    upper = Flt64(2.0),
    converter = IntoValue.Identity,
    name = "slack-range"
)
check(slack.evaluate(values) == Flt64(0.0))
```

```rust [Rust]
let slack = SlackRangeFunction::new(1, "slack_range", input, -2.0_f64, 2.0_f64);
assert_eq!(slack.calculate_value(&tokens, false), Some(0.0));
```

:::

## Tests and references

- Kotlin: [`SlackRangeFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/SlackRangeFunctionDedicatedTest.kt)
- Kotlin implementation: [`SlackRange.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/SlackRange.kt)
- Rust implementation: [`slack_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/slack_range.rs)
- Rust focused test: [`function_symbol_slack_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_slack_range.rs)

The focused tests assert all four helpers and the seven exact-Max rows with an
explicit Big-M, while also covering all three distance regions and reversed bounds.

Both implementations validate finite ordered bounds. Rust also provides
SlackRangeFunction::with_big_m for an explicit positive finite Big-M; the
default constructor delegates to the shared fallback or token-derived policy.
