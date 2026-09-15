# Xor (Exactly One)

`XorFunction` returns one if and only if exactly one input expression is
nonzero. For two inputs this is ordinary XOR; for three or more inputs it is
an exactly-one predicate, not odd parity.

## Mathematical definition

For nonzero indicators $a_i\in\{0,1\}$ and result $y\in\{0,1\}$,

$$
y=1\iff\sum_i a_i=1.
$$

Both Kotlin and Rust use the same exact linear encoding:

$$
\begin{aligned}
y&\le\sum_i a_i,\\
y&\ge a_i-\sum_{j\ne i}a_j &&\forall i,\\
y+a_i+a_j&\le2 &&\forall i<j.
\end{aligned}
$$

The first row forces zero when every input is zero. The second family forces
one when exactly one indicator is active. The pair rows force zero as soon as
two or more indicators are active.

## Nonzero indicators

Each input is connected to an indicator and a sign-side helper by the shared
nonzero Big-M formulation. Both implementations use a default zero-band
tolerance of `1e-10` and a strict nonzero boundary of about `1.6e-9`.
Kotlin exposes `bigM`, `tolerance`, and `strictBoundary` on the direct
constructor; Rust exposes the matching `with_big_m`, `with_tolerance`,
`with_strict_boundary`, and `with_parameters` builders. Values with
`abs(input) <= tolerance` are zero. The open transition interval
`tolerance < abs(input) < strictBoundary` is undefined (`null`/`None`) because
the solver deliberately rejects it; values at or beyond the strict boundary
are nonzero.

## Kotlin/Rust example

::: code-group

```kotlin [Kotlin]
val exactlyOne = XorFunction(
    polynomials = listOf(a, b, c),
    converter = IntoValue.Identity,
    name = "exactly-one"
)
check(exactlyOne.evaluate(valuesWithOnlyA) == Flt64.one)
check(exactlyOne.evaluate(valuesWithAAndB) == Flt64.zero)
```

```rust [Rust]
let exactly_one = XorFunction::new(1, "exactly_one", vec![a, b, c]);
assert_eq!(exactly_one.calculate_value(&only_a_tokens, false), Some(1.0));
assert_eq!(exactly_one.calculate_value(&a_and_b_tokens, false), Some(0.0));
```

:::

To configure all numeric policy values in Rust at once:

```rust
let configured = XorFunction::new(2, "configured_xor", vec![a, b])
    .with_parameters(Some(100.0), 1e-8, 1e-6);
```

## Tests and references

- Kotlin: [`XorFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/XorFunctionDedicatedTest.kt)
- Kotlin implementation: [`And.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/And.kt)
- Rust: [`function_symbol_xor.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_xor.rs)
- Rust implementation: [`and.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/and.rs)
