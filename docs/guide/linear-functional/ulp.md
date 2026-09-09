# Univariate linear piecewise function

`UnivariateLinearPiecewiseFunction` represents a one-input piecewise linear function. The most convenient current factory builds it from ordered sample points.

## Contract

- Input: `x: LinearPolynomial<V>` and at least two two-dimensional points `(x_i, y_i)`.
- Output: a `RealVar` (`resultVar`) exposed through `resultPolynomial`.
- `.fromPoints` computes one slope and intercept for each adjacent pair.
- `evaluate` returns the first segment's affine value when the input lies in a closed segment; it returns `null` outside the total breakpoint interval or when the input cannot be evaluated.
- `V` must also implement `FloatingNumber<V>` for `.fromPoints`, because slopes and intercepts are computed by division.

## Mathematical definition

For strictly increasing breakpoints $t_0<t_1<\cdots<t_m$, adjacent points define

$$
a_i=\frac{y_{i+1}-y_i}{t_{i+1}-t_i},\qquad b_i=y_i-a_i t_i,
$$

and the segment function

$$
f(x)=a_i x+b_i\quad\text{for }t_i\le x\le t_{i+1}
$$

The implementation tests segments in list order, so a shared breakpoint belongs to the first matching segment. With points from a normal graph, adjacent formulas agree at that boundary.

## Domain and boundaries

`.fromPoints` requires at least two points and strictly increasing x-coordinates. Repeated or descending x-coordinates make `fromPointsResult` return `Failed`; the convenience `.fromPoints` returns an invalid placeholder whose later registration/result-boundary operation reports the failure. Use `.fromPointsResult` when the construction error must be handled explicitly. Evaluation is defined on the closed interval `[t_0, t_m]`; an input below `t_0` or above `t_m` returns `null`.

The direct constructor additionally expects `breakpoints.size = slopes.size + 1 = intercepts.size + 1`. For solver registration, omit `m` only when a finite input range can be proved; otherwise pass a valid explicit `m` Big-M. The function derives output bounds from the segments and validates finite/representable values.

## Current API

### Kotlin

Source: [`UnivariateLinearPiecewise.kt` (constructor and evaluation)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/UnivariateLinearPiecewise.kt#L60-L94) and [`fromPoints`/`fromPointsResult`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/UnivariateLinearPiecewise.kt#L921-L1047)

```kotlin
UnivariateLinearPiecewiseFunction.fromPoints(
    x: LinearPolynomial<V>,
    points: List<Point<Dim2, V>>,
    m: V? = null,
    converter: IntoValue<V>,
    name: String,
    displayName: String? = null
)
```

For a precomputed representation, the direct constructor/factory accepts `breakpoints`, `slopes`, and `intercepts`; `.fromPointsResult` is the safe `Ret`-returning alternative.

### Rust

Rust exposes [`Point2`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/univariate_linear_piecewise.rs) and [`UnivariateLinearPiecewiseFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/univariate_linear_piecewise.rs):

```rust
Point2::new(x: V, y: V) -> Point2<V>

UnivariateLinearPiecewiseFunction::new(
    id: u64,
    name: &str,
    input: Linear<V>,
    points: Vec<Point2<V>>,
) -> UnivariateLinearPiecewiseFunction<V>
```

The Rust symbol sorts points by x, creates convex-combination `lambda_variables()`, and exposes a continuous `result_variable()`. Its direct evaluator clamps values below/above the first/last point to the corresponding endpoint, whereas Kotlin `evaluate` returns `null` outside the breakpoint interval. Rust has no Kotlin `fromPoints`/`m` converter parameters; callers construct `Point2` values directly.

## Auxiliary variables and registration

`helperVariables` contains the real `resultVar` and one binary `selectorVar` per segment. Registration requires exactly one active segment, gates each segment's breakpoint interval with Big-M, and gates `result = slope * x + intercept` for the active segment. It also validates or derives the input/output ranges before committing tokens and constraints.

## `evaluate` versus solver

`evaluate` is a simple closed-segment scan and returns `null` outside the breakpoint interval. Solver registration is stricter: it needs a finite input range when `m` is omitted, computes finite output bounds, and adds Big-M segment gates. A direct evaluation can therefore be available even when solver registration correctly reports that no finite range or valid constraint representation was proven.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.UnivariateLinearPiecewiseFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.geometry.*
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.eq
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

val x = RealVar("x")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val ulp = UnivariateLinearPiecewiseFunction.fromPoints(
    x = xPoly,
    points = listOf(
        point2(),
        point2(x = Flt64.one, y = Flt64.two),
        point2(x = Flt64.two, y = Flt64.one)
    ),
    converter = IntoValue.Identity,
    name = "y"
)
val value = ulp.evaluate(mapOf<Symbol, Flt64>(x to Flt64.one))
check(value != null && (value eq Flt64.two))
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{Point2, UnivariateLinearPiecewiseFunction};

let input = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let ulp = UnivariateLinearPiecewiseFunction::new(
    1,
    "y",
    input,
    vec![
        Point2::new(0.0_f64, 0.0_f64),
        Point2::new(1.0_f64, 2.0_f64),
        Point2::new(2.0_f64, 1.0_f64),
    ],
);
assert_eq!(ulp.points().len(), 3);
let _result = ulp.result_variable();
```

:::

Complete example: [`ULPTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/ULPTest.kt)

Core validation: [`UnivariateLinearPiecewiseGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/UnivariateLinearPiecewiseGenericEvaluateTest.kt) and [`UnivariateLinearPiecewiseFailureBoundaryTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/UnivariateLinearPiecewiseFailureBoundaryTest.kt)

Rust source: [`univariate_linear_piecewise.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/univariate_linear_piecewise.rs).

## Related pages

- [`blp`](./blp): two-input triangulated piecewise interpolation.
- [`max`](./max) and [`min`](./min): discrete selection among affine candidates.
- [`rounding`](./rounding): integer output rather than continuous interpolation.
