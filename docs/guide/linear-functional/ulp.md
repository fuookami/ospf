# Univariate Linear Piecewise Function

`UnivariateLinearPiecewiseFunction` represents the graph obtained by
linearly interpolating consecutive points $(t_i,f_i)$. Points must contain at
least two entries and their $t_i$ values must be finite and strictly
increasing. Evaluation outside $[t_0,t_m]$ is undefined.

## Solver mathematical model

Kotlin uses one binary selector $z_j$ per segment. With the affine formula
$g_j(x)=a_jx+b_j$, it enforces $\sum_jz_j=1$, gates $x$ to the selected
interval, and gates $y=g_j(x)$ with finite Big-M bounds.

Rust uses point weights $\lambda_i\in[0,1]$ and the same one-hot segment
selectors $z_j\in\{0,1\}$. It enforces

$$
\begin{aligned}
\sum_i\lambda_i&=1,&\sum_j z_j&=1,\\
x&=\sum_i t_i\lambda_i,&y&=\sum_i f_i\lambda_i.
\end{aligned}
$$

Adjacency is enforced by

$$
\lambda_0\le z_0,
\qquad
\lambda_m\le z_{m-1},
\qquad
\lambda_i\le z_{i-1}+z_i\quad(0<i<m).
$$

Consequently only the two endpoints of the selected segment may have
positive weights. Although the internal formulas differ, both sides now
represent the same single active segment and exclude arbitrary convex
combinations of non-adjacent points.

## Kotlin/Rust example

::: code-group

```kotlin [Kotlin]
val piecewise = UnivariateLinearPiecewiseFunction.fromPoints(
    x = input,
    points = points,
    converter = IntoValue.Identity,
    name = "ulp"
)
```

```rust [Rust]
let piecewise = UnivariateLinearPiecewiseFunction::new(
    1,
    "ulp",
    input,
    vec![
        Point2::new(0.0, 0.0),
        Point2::new(1.0, 2.0),
        Point2::new(2.0, 0.0),
    ],
);
assert_eq!(piecewise.selector_variables().len(), 2);
```

:::

## Evaluation and boundaries

On segment $[t_i,t_{i+1}]$, direct evaluation uses

$$
y=f_i+\frac{x-t_i}{t_{i+1}-t_i}(f_{i+1}-f_i).
$$

Missing input values and values outside the point domain return `null` in
Kotlin and `None` in Rust. Duplicate, descending, non-finite, or insufficient
points are rejected.

## Tests and references

- Kotlin focused test: [`UnivariateLinearPiecewiseFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/UnivariateLinearPiecewiseFunctionDedicatedTest.kt)
- Kotlin boundary/registration test: [`UnivariateLinearPiecewiseFailureBoundaryTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/UnivariateLinearPiecewiseFailureBoundaryTest.kt)
- Kotlin implementation: [`UnivariateLinearPiecewise.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/UnivariateLinearPiecewise.kt)
- Rust implementation: [`univariate_linear_piecewise.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/univariate_linear_piecewise.rs)
- Rust dedicated test: [`function_symbol_univariate_linear_piecewise.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_univariate_linear_piecewise.rs)

The focused tests assert the helper counts and every selector/segment graph row,
along with endpoint and out-of-domain behavior and invalid point validation.

The point and segment forms share the same ordered-breakpoint contract. Kotlin
uses breakpoints/slopes/intercepts directly; Rust also provides
UnivariateLinearPiecewiseFunction::from_segments, with one slope and
intercept for each adjacent breakpoint pair. Invalid values are rejected
before helper registration.
