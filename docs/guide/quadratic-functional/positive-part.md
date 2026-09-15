# Quadratic Positive Part

`QuadraticPositivePartFunction` gives both implementations the same unambiguous operation:

$$
y=\max\{p(x),0\}.
$$

This is a positive-part function, not a semi-continuous variable. The old Rust name `QuadraticSemiFunction` has been removed. Semi-continuous domains remain the responsibility of the separate linear `SemiFunction` API.

## Solver mathematical model

### Kotlin

Kotlin uses the identity $y=-\min\{-p(x),0\}$. Let $t=\min\{-p(x),0\}$ and $u_0,u_1\in\{0,1\}$. Its exact selector model is:

$$
\begin{aligned}
t &\le -p(x),\\
t &\le 0,\\
t &\ge -p(x)-M(1-u_0),\\
t &\ge -M(1-u_1),\\
u_0+u_1 &=1,\\
y&=-t.
\end{aligned}
$$

The public quadratic result expression is `-resultVar`; `resultVar` itself is the internal minimum $t$.

### Rust

Rust first bridges the quadratic input as $b=p(x)$ and then applies an exact two-candidate maximum:

$$
\begin{aligned}
b&=p(x),\\
y&\ge b,\\
y&\ge0,\\
y&\le b+M(1-u_0),\\
y&\le M(1-u_1),\\
u_0+u_1&=1.
\end{aligned}
$$

The formulations differ internally but define the same result and direct-evaluation contract.

## API and examples

::: code-group

```kotlin [Kotlin]
val p = QuadraticPolynomial(
    monomials = listOf(QuadraticMonomial.quadratic(Flt64.one, x, x)),
    constant = -Flt64(4.0)
)
val positivePart = QuadraticPositivePartFunction(
    input = p,
    bigM = Flt64(100.0),
    converter = IntoValue.Identity,
    name = "positive_part"
)
// x = 1: max(1^2 - 4, 0) = 0
```

```rust [Rust]
let p = Quadratic::new(
    vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
    -4.0,
);
let positive_part = QuadraticPositivePartFunction::new(
    17,
    "positive_part",
    p,
);
// x = 1: max(1^2 - 4, 0) = 0
```

:::

Kotlin accepts an optional explicit `bigM`; otherwise it derives candidate-specific values from finite polynomial bounds. Rust derives the selector Big-M from registered token bounds when possible and otherwise uses its configured fallback.

## Evaluation and boundaries

Direct evaluation returns zero for a negative input, the input for a positive value, and zero at the origin. Missing input values return `null`/`None`. This arithmetic function has no tolerance-based `Undefined` region.

## Tests and source

- Kotlin source: [`QuadraticPositivePart.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticPositivePart.kt)
- Kotlin independent test: [`QuadraticPositivePartFunctionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticPositivePartFunctionTest.kt)
- Rust source: [`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)
- Rust independent test: [`function_symbol_quadratic_positive_part.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_positive_part.rs)

## Related pages

- [Linear semi-continuous marker](../linear-functional/semi)
- [Quadratic minimum](./quadratic-min)
