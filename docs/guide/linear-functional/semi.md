# Semi-Continuous Variable

`SemiFunction<V>` models a variable that is either zero or lies in the
active interval `[lb, ub]`.

## Solver mathematical model

The symbol creates a continuous result $y$ and a binary activation variable
$b$. The rows sent to the solver are

$$
y-ub\,b\le0,
\qquad
y-lb\,b\ge0.
$$

When $b=0$, these rows force $y=0$; when $b=1$, they enforce
$lb\le y\le ub$. The constructor validates `lb <= ub` and the result and
indicator are registered as auxiliary variables.

## Kotlin API

```kotlin
val semi = SemiFunction(
    lb = Flt64(2.0),
    ub = Flt64(5.0),
    converter = IntoValue.Identity,
    name = "semi"
)
```

`SemiFunction.from(variable, ...)` infers missing finite bounds from a
continuous variable. `resultVar`, `indicatorVar`, and `resultPolynomial`
expose the model representation.

## Rust API

```rust
let semi = SemiFunction::new(1, "semi", 2.0_f64, 5.0_f64);
assert!(semi.result_variable().name().contains("semi"));
```

Rust also provides `try_from_variable`/`from_variable` for finite-bound
inference. Both implementations register the same two domain rows.

## Kotlin/Rust example

::: code-group

```kotlin [Kotlin]
val semi = SemiFunction(
    lb = Flt64(2.0),
    ub = Flt64(5.0),
    converter = IntoValue.Identity,
    name = "semi"
)
check(semi.helperVariables.size == 2)
```

```rust [Rust]
let semi = SemiFunction::new(1, "semi", 2.0_f64, 5.0_f64);
assert_eq!(semi.lower_bound(), &2.0);
assert_eq!(semi.upper_bound(), &5.0);
```

:::

## Tests and references

- Kotlin focused test: [`SemiFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/SemiFunctionDedicatedTest.kt)
- Kotlin regression test: [`SemiFunctionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/SemiFunctionTest.kt)
- Kotlin implementation: [`Semi.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Semi.kt)
- Rust implementation: [`semi.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/semi.rs)
- Rust focused test: [`function_symbol_semi.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_semi.rs)

The focused tests assert both helper tokens and both registered domain rows,
plus the shared default interval and eager reversed/non-finite bound validation.

Bounds are validated eagerly and must be finite with lower no greater than
upper. Omitting Kotlin bounds uses the shared interval [0, 1e6]; Rust exposes
SemiFunction::with_default_bounds for the same interval. Missing token values
remain explicit: Kotlin returns null, while Rust follows zero_if_none.
