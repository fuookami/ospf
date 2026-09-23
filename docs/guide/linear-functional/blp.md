# Bivariate Linear Piecewise Function

`BivariateLinearPiecewiseFunction` represents a piecewise-planar surface over
a triangulation. Every cell has three vertices $(x_{tk},y_{tk},z_{tk})$.

## Solver mathematical model

Let $s_t\in\{0,1\}$ select a triangle and
$\lambda_{tk}\in[0,1]$ be its barycentric weights. Both implementations enforce
exactly one active triangle whose weights sum to one, and express the
coordinates as weighted sums over all vertices:

$$
\begin{aligned}
\sum_t s_t&=1,\\
x&=\sum_{t,k}x_{tk}\lambda_{tk},\\
y&=\sum_{t,k}y_{tk}\lambda_{tk},\\
z&=\sum_{t,k}z_{tk}\lambda_{tk}.
\end{aligned}
$$

Rust ties each weight group to its selector with the per-triangle equality
$\sum_{k=0}^{2}\lambda_{tk}=s_t$ and registers the $z$ relation as an explicit
equality row. Kotlin instead combines the global row
$\sum_{t,k}\lambda_{tk}=1$ with the SOS2-style rows
$\sum_{k=0}^{2}\lambda_{tk}\le 3s_t$ per triangle and exposes
$z=\sum_{t,k}z_{tk}\lambda_{tk}$ directly as `resultPolynomial`; for binary
$s_t$ the two encodings describe the same feasible set.

Only one triangle may carry nonzero weights. This is different from an
unrestricted convex hull over all vertices and therefore preserves the
piecewise surface across non-coplanar cells.

## Direct evaluation

For each triangle, the evaluator computes barycentric coordinates
$(\lambda_0,\lambda_1,\lambda_2)$. Both implementations use the same
geometry tolerance `1e-12`: weights down to `-1e-12` are accepted. The first
triangle for which every weight is within that tolerance contains the input, and

$$
z=\lambda_0z_0+\lambda_1z_1+\lambda_2z_2.
$$

Inputs outside every triangle return `null`/`None`. Construction rejects every
triangle with a non-finite coordinate or an absolute 2-D determinant at or
below `1e-12`; degenerate triangles therefore cannot reach evaluation in either
implementation.

## Kotlin/Rust example

::: code-group

```kotlin [Kotlin]
val surface = BivariateLinearPiecewiseFunction(
    x = xPolynomial,
    y = yPolynomial,
    triangles = triangles,
    converter = IntoValue.Identity,
    name = "surface"
)
```

```rust [Rust]
let surface = BivariateLinearPiecewiseFunction::new(
    1,
    "surface",
    x_input,
    y_input,
    vec![Triangle3::new(
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 10.0),
        Point3::new(0.0, 1.0, 20.0),
    )],
);
assert_eq!(surface.selector_variables().len(), 1);
```

:::

## Tests and references

- Kotlin implementation: [`BivariateLinearPiecewise.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/BivariateLinearPiecewise.kt)
- Kotlin dedicated test: [`BivariateLinearPiecewiseFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/BivariateLinearPiecewiseFunctionDedicatedTest.kt)
- Rust implementation: [`bivariate_linear_piecewise.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/bivariate_linear_piecewise.rs)
- Rust dedicated test: [`function_symbol_bivariate_linear_piecewise.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_bivariate_linear_piecewise.rs)
