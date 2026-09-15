# Quadratic In-Step Range

`QuadraticInStepRangeFunction` is a closed-interval gate, not a floor or step-rounding operation. For a quadratic polynomial $p(x)$, bounds $L\le U$, and exterior tolerance $\varepsilon>0$:

$$
g_\varepsilon(p)=\begin{cases}
p(x), & L\le p(x)\le U,\\
0, & p(x)\le L-\varepsilon\ \text{or}\ p(x)\ge U+\varepsilon,\\
\text{undefined}, & \text{otherwise}.
\end{cases}
$$

The undefined bands make the continuous-domain boundary contract explicit: strict complements cannot be encoded exactly by finitely many non-strict inequalities. Both implementations create three mutually exclusive binary indicators $z_{in}$, $z_{low}$, and $z_{high}$ plus a signed result $y$. With a positive Big-M constant $M$, the solver model is:

$$
\begin{aligned}
p(x)-Mz_{in} &\ge L-M,\\
p(x)+Mz_{in} &\le U+M,\\
p(x)+Mz_{low} &\le L-\varepsilon+M,\\
p(x)-Mz_{high} &\ge U+\varepsilon-M,\\
z_{in}+z_{low}+z_{high} &=1,\\
y-p(x)-Mz_{in} &\ge -M,\\
y-p(x)+Mz_{in} &\le M,\\
y-Mz_{in} &\le 0,\\
y+Mz_{in} &\ge 0.
\end{aligned}
$$

The one-hot partition forces the inside branch throughout $[L,U]$, the low branch at or below $L-\varepsilon$, and the high branch at or above $U+\varepsilon$. The last four rows set $y=p(x)$ only on the inside branch and $y=0$ otherwise.

## API

::: code-group

```kotlin [Kotlin]
val p = QuadraticPolynomial(
    monomials = listOf(QuadraticMonomial.quadratic(Flt64.one, x, x)),
    constant = Flt64.zero
)
val f = QuadraticInStepRangeFunction(
    x = p,
    lower = Flt64.zero,
    upper = Flt64(4.0),
    bigM = Flt64(100.0),
    outsideTolerance = Flt64(1e-6),
    converter = IntoValue.Identity,
    name = "square_gate"
)
```

```rust [Rust]
let p = Quadratic::new(
    vec![QuadraticMonomial::new_quadratic(1.0, x_index, x_index)],
    0.0,
);
let f = QuadraticInStepRangeFunction::with_parameters(
    11, "square_gate", p, 0.0, 4.0, 100.0, 1e-6,
);
```

:::

The old Rust step-floor constructor and `with_quadratic_bounds` API were removed. A separate floor function should be used when the mathematical operation is $L+|s|\lfloor(U-L)/|s|\rfloor$.

## Evaluation and registration

Evaluation returns the input on the closed interval, zero beyond the exterior tolerance, and `null`/`None` in either undefined tolerance band. Registration adds three binary indicators and the signed result helper, then emits nine rows. A linear input is emitted through the linear mechanism model; a quadratic input is emitted through the quadratic mechanism model.

## Tests and source

- Kotlin source: [`QuadraticInStepRange.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticInStepRange.kt)
- Rust source: [`quadratic_in_step_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_in_step_range.rs)
- Kotlin independent tests: [`QuadraticInStepRangeFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticInStepRangeFunctionDedicatedTest.kt) and [`QuadraticFunctionGenericEvaluationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionGenericEvaluationTest.kt)
- Rust independent test: [`function_symbol_quadratic_in_step_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_in_step_range.rs)

## Related pages

- [Linear in-step range](../linear-functional/in-step-range)
- [Quadratic masking range](./quadratic-masking-range)
