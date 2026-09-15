# Quadratic Masking Range

`QuadraticMaskingRangeFunction` is the binary-gated quadratic expression shared by the Kotlin and Rust implementations. For a quadratic polynomial $p(x)$, binary mask $z$ and signed result $y$:

$$
y = \begin{cases}
p(x), & z=1,\\
0, & z=0.
\end{cases}
$$

The solver model uses one positive finite constant $M$ and the same four rows in both languages:

$$
\begin{aligned}
y-p(x)+Mz &\le M,\\
y-p(x)-Mz &\ge -M,\\
y &\le Mz,\\
y &\ge -Mz.
\end{aligned}
$$

The mask is a binary variable, and $y$ is signed. The fourth row is essential when $p(x)$ can be negative; an unsigned helper variable is not equivalent.

## API

::: code-group

```kotlin [Kotlin]
val z = BinVar("z")
val p = QuadraticPolynomial(
    monomials = listOf(QuadraticMonomial.quadratic(Flt64.one, x, x)),
    constant = Flt64.zero
)
val f = QuadraticMaskingRangeFunction(
    polynomial = p,
    z = z,
    bigM = Flt64(100.0),
    converter = IntoValue.Identity,
    name = "masked_square"
)
```

```rust [Rust]
let z = BinaryVariableItem::create(VariableId::standalone(2), "z");
let p = Quadratic::new(
    vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
    0.0,
);
let f = QuadraticMaskingRangeFunction::with_big_m(
    2, "masked_square", p, z.clone(), 100.0,
);
```

:::

`with_quadratic_bounds` and the old lower/upper-bound contract were removed. They described a different range-variable operation and are no longer part of the public API.

## Evaluation and registration

At evaluation time, `z=0` returns zero without reading the result token; `z=1` evaluates $p(x)$. Registration adds the signed result helper and the four rows above. For a linear input, the rows are emitted as linear constraints; for a genuinely quadratic input, they are emitted as quadratic constraints.

## Tests and source

- Kotlin source: [`QuadraticMaskingRange.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticMaskingRange.kt)
- Rust source: [`quadratic_masking_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_masking_range.rs)
- Kotlin independent tests: [`QuadraticMaskingRangeFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticMaskingRangeFunctionDedicatedTest.kt) and [`QuadraticFunctionGenericEvaluationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionGenericEvaluationTest.kt)
- Rust independent test: [`function_symbol_quadratic_masking_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_masking_range.rs)

## Related pages

- [Linear masking](../linear-functional/masking)
- [Quadratic in-step range](./quadratic-in-step-range)
