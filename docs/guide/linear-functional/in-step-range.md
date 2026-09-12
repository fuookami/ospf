# In-Step Range

`InStepRangeFunction` returns the greatest grid point not exceeding the
upper expression:

$$
y=lb+\left\lfloor\frac{ub-lb}{step}\right\rfloor step.
$$

It is a numeric stepping function, not a Boolean membership test. `step`
must be finite and strictly positive, and the implementation rejects
`ub < lb` when the value is evaluated or constrained.

## Solver mathematical model

Let $d=ub-lb$, $k=\lfloor d/step\rfloor$, and $y=lb+step\,k$. The solver
receives the delegated floor encoding for $k$, together with the explicit
ordering row

$$
ub-lb\ge0.
$$

The quotient is divided by `step` before flooring; therefore a range from 1
to 10 with step 3 returns 10, while a range from 1 to 9 returns 7.

## Kotlin API

```kotlin
val stepped = InStepRangeFunction(
    lb = lower,
    ub = upper,
    step = Flt64(3.0),
    converter = IntoValue.Identity,
    name = "stepped"
)
```

## Rust API

Rust now exposes the same numeric function:

```rust
let stepped = InStepRangeFunction::new(
    1, "stepped", lower, upper, 3.0_f64,
);
```

The former grid-membership function is explicitly named
`InStepRangeIndicatorFunction`; use it when the required result is a binary
“is this value one of the grid points?” flag.

## Kotlin/Rust example

::: code-group

```kotlin [Kotlin]
val stepped = InStepRangeFunction(
    lb = constant(1.0),
    ub = constant(10.0),
    step = Flt64(3.0),
    converter = IntoValue.Identity,
    name = "stepped"
)
check(stepped.evaluate(emptyMap()) == Flt64(10.0))
```

```rust [Rust]
let stepped = InStepRangeFunction::new(1, "stepped", lower, upper, 3.0_f64);
assert_eq!(stepped.calculate_value(&tokens, false), Some(10.0));
```

:::

## Tests and references

- Kotlin numeric focused test: [`InStepRangeFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/InStepRangeFunctionDedicatedTest.kt)
- Kotlin membership focused test: [`InStepRangeIndicatorFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/InStepRangeIndicatorFunctionDedicatedTest.kt)
- Kotlin implementation: [`InStepRange.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/InStepRange.kt)
- Rust numeric implementation: [`in_step_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/in_step_range.rs)
- Rust numeric focused test: [`function_symbol_in_step_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_in_step_range.rs)
- Rust membership focused test: [`function_symbol_in_step_range_indicator.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_in_step_range_indicator.rs)

The focused tests assert the numeric function's two Floor helpers and four
registered rows, and the indicator's seven helpers and sixteen point/OR rows;
they also cover grid boundaries, missing input, and invalid step/Big-M values.

The endpoint function has no unused Big-M parameter because FloorFunction does
not use one. InStepRangeIndicatorFunction is available in both Kotlin and Rust
for membership tests and uses the shared 1e-10 boundary epsilon. Kotlin accepts
an optional `bigM` constructor argument; Rust uses
`InStepRangeIndicatorFunction::with_big_m` when an explicit positive finite
value is required, and `new` otherwise follows the token-derived/default
policy.
