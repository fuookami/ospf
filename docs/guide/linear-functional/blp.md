# Bivariate linear piecewise function

`BivariateLinearPiecewiseFunction` represents a piecewise-linear surface over a list of triangles. Its current model is triangulation plus barycentric interpolation, not a rectangular list of independent points.

## Contract

- Inputs: `x` and `y`, each a `LinearPolynomial<V>`.
- Geometry: a non-empty `List<Triangle<Point<Dim3, Flt64>, Dim3, Flt64>>`; each vertex stores `(x, y, z)`.
- Output: a linear polynomial formed from barycentric weights and the vertices' z-coordinates.
- `evaluate` returns the interpolated z value for the first containing, non-degenerate triangle, and `null` when either input is missing, the point is outside every triangle, or every candidate triangle is degenerate.
- `V` must implement `RealNumber<V>` and `NumberField<V>`; geometric coordinates are `Flt64` and are converted through `IntoValue<V>`.

## Mathematical definition

For a triangle with vertices $P_1=(x_1,y_1,z_1)$, $P_2=(x_2,y_2,z_2)$, and $P_3=(x_3,y_3,z_3)$, a point in the triangle is represented by barycentric weights

$$
\lambda_1=1-u-v,\qquad \lambda_2=u,\qquad \lambda_3=v,
$$

with $u\ge0$, $v\ge0$, and $u+v\le1$. The interpolated value is

$$
z=\lambda_1z_1+\lambda_2z_2+\lambda_3z_3
 =z_1+(z_2-z_1)u+(z_3-z_1)v.
$$

The implementation computes `u` and `v` from the x/y coordinates and accepts triangle boundaries inclusively.

## Domain and boundaries

Construction requires at least one triangle. A triangle whose 2-D determinant has absolute value at most $10^{-12}$ is treated as degenerate and cannot produce an evaluation. Points outside all triangles return `null`. If triangles overlap, evaluation uses the first containing triangle in list order; shared boundaries are therefore order-sensitive when neighboring z values are inconsistent. Solver registration uses exactly one selected triangle and assumes the listed geometry describes the intended domain.

## Current API

### Kotlin

Source: [`BivariateLinearPiecewise.kt` (constructor, barycentric evaluation, and constraints)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/BivariateLinearPiecewise.kt#L58-L264)

```kotlin
BivariateLinearPiecewiseFunction(
    x: LinearPolynomial<V>,
    y: LinearPolynomial<V>,
    triangles: List<Triangle<Point<Dim3, Flt64>, Dim3, Flt64>>,
    converter: IntoValue<V>,
    name: String,
    displayName: String? = null
)
```

### Rust

Source: [`bivariate_linear_piecewise.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/bivariate_linear_piecewise.rs)

Rust uses a non-empty list of `Point3<V>` and a convex-combination formulation rather than Kotlin's list of triangles. The primary constructor and accessors are:

```rust
Point3::new(x: V, y: V, z: V) -> Point3<V>
BivariateLinearPiecewiseFunction::new(
    id: u64,
    name: &str,
    x_input: Linear<V>,
    y_input: Linear<V>,
    points: Vec<Point3<V>>,
) -> Self
```

`result_variable()`, `lambda_variables()`, `x_input_polynomial()`, `y_input_polynomial()`, and `points()` expose the registered model pieces. The Rust evaluator reads the lambda-variable values from tokens and returns their z-weighted sum; the x/y geometry is enforced by mechanism constraints.

## Auxiliary variables and registration

For every triangle, `lambdaVars` is a `PctVariable1` with shape 3 and `zVars` is a `BinVariable1` selector. Registration constrains x and y to the lambda-weighted vertex coordinates, constrains the result to the weighted z coordinates, sets the sum of all lambdas to one, gates each triangle's lambda sum by its selector, and sets the selector sum to one. Percentage variables provide the `[0, 1]` bounds.

## `evaluate` versus solver

`evaluate` searches triangles in order and returns a barycentric interpolation or `null`. Solver registration introduces one-hot triangle selection and lambda variables and does not itself repair overlapping, inconsistent, or degenerate geometry. A solver model should therefore be given a coherent triangulation whose covered domain matches the intended input range.

## Minimal current example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.BivariateLinearPiecewiseFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.geometry.*
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.eq
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

val x = RealVar("x")
val y = RealVar("y")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val yPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, y)), Flt64.zero)
val blp = BivariateLinearPiecewiseFunction(
    x = xPoly,
    y = yPoly,
    triangles = listOf(
        Triangle(
            point3(Flt64.zero, Flt64.zero, Flt64.zero),
            point3(Flt64.one, Flt64.zero, Flt64.one),
            point3(Flt64.zero, Flt64.one, Flt64.one)
        )
    ),
    converter = IntoValue.Identity,
    name = "blp"
)
val value = blp.evaluate(
    mapOf<Symbol, Flt64>(x to Flt64(0.25), y to Flt64(0.25))
)
check(value != null && (value eq Flt64(0.75)))
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::{BivariateLinearPiecewiseFunction, Point3};
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};

let blp = BivariateLinearPiecewiseFunction::new(
    1,
    "blp",
    Linear::new(vec![], 0.25),
    Linear::new(vec![], 0.25),
    vec![
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 1.0),
        Point3::new(0.0, 1.0, 1.0),
    ],
);
let mut tokens = VecTokenList::new();
for (lambda, value) in blp.lambda_variables().iter().zip([0.5, 0.25, 0.25]) {
    let token = Token::from_generic(lambda.clone(), lambda.index());
    token.set_result(value);
    tokens.add_token(token);
}
let value = <BivariateLinearPiecewiseFunction as FunctionSymbol>::calculate_value(
    &blp,
    &tokens,
    false,
);
assert_eq!(value, Some(0.5));
```

:::

Complete example: [`BLPTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/BLPTest.kt)

Core validation: [`TrigonometricAndBivariateGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/TrigonometricAndBivariateGenericEvaluateTest.kt)

Rust implementation and coverage: [`bivariate_linear_piecewise.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/bivariate_linear_piecewise.rs), [`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs)

## Related pages

- [`ulp`](./ulp): one-input piecewise interpolation from ordered points.
- [`max`](./max) and [`min`](./min): selector-based linear functions.
- [`masking`](./masking): binary gating of a linear input.
