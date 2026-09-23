# Inequality Indicator

`InequalityFunction` turns a comparison between a linear polynomial and a scalar into a binary result.

## Contract

- Input: `lhs: LinearPolynomial<V>`, scalar `rhs: V`, and a `Comparison` sign.
- Direct `evaluate` supports `LE`, `LT`, `GE`, `GT`, `EQ`, and `NE`.
- Output: `result`, a linear polynomial containing the binary flag.
- Solver registration supports `LE/LT/GE/GT/EQ/NE`; `NE` uses the same zero-band side encoding as `EQ`, with the result flag acting as the nonzero indicator.
- Generic values use `V : RealNumber<V>, V : NumberField<V>` and an `IntoValue<V>` converter.

## Definition and mathematical model

Let $d=lhs-rhs$ and $y\in\{0,1\}$. The intended contract is

$$
y=\mathbf{1}[lhs\ \mathrel{\text{sign}}\ rhs].
$$

For `EQ` and `NE`, the solver uses a zero-band with tolerance and a side binary variable. For the other supported signs, two Big-M inequalities link the flag to the satisfied and violated branches. The exact direct-evaluation comparison is separate from the solver tolerance encoding.

## Solver mathematical model

Let $d=lhs-rhs$ and normalize the requested relation to $q\ge T$ for the true branch and $q\le F$ for the false branch:

| Relation | $q$ | $T$ | $F$ |
| --- | ---: | ---: | ---: |
| `GT` | $d$ | $g$ | $0$ |
| `GE` | $d$ | $0$ | $-g$ |
| `LT` | $-d$ | $g$ | $0$ |
| `LE` | $-d$ | $0$ | $-g$ |

For result $y\in\{0,1\}$ and a Big-M $M$ inferred from the finite lhs-rhs range, the implementation passes two Big-M rows to the solver:

$$
\begin{aligned}
q-M_1y&\le F,\\
q-M_2y&\ge T-M_2,
\end{aligned}
$$

where one multiplier is the gap-relaxed $M+g$ so that exactly one row is binding per indicator value ($M_1=M,\ M_2=M+g$ for `GT`/`LT`, $M_1=M+g,\ M_2=M$ for `LE`/`GE`). Hence $y=1\Rightarrow q\ge T$ and $y=0\Rightarrow q\le F$; the open interval $(F,T)$ is intentionally infeasible. For `EQ` and `NE`, both implementations additionally create a side binary and use the shared four-row zero/nonzero Big-M encoding: the `EQ` flag equals the complement of the nonzero flag, while the `NE` flag is the nonzero flag itself.

## Current API

### Kotlin

Source: [`Inequality.kt` (`InequalityFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Inequality.kt#L44-L229)

```kotlin
InequalityFunction(
    lhs: LinearPolynomial<V>,
    rhs: V,
    sign: Comparison,
    converter: IntoValue<V>,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    name: String = "ineq",
    displayName: String? = null
)
```

The public factory exposes `bigM` but not the constructor's optional `tolerance` and `strictBoundary`; use the class constructor when those parameters must be customized.

### Rust

Rust provides a direct flattened-expression counterpart, [`InequalityFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/inequality.rs). It takes one `Linear<V>`, a scalar right-hand value, an explicit [`InequalityKind`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/inequality.rs), and Big-M:

```rust
InequalityFunction::new(
    id: u64,
    name: &str,
    left: Linear<V>,
    right: V,
    kind: InequalityKind,
    big_m: V,
) -> InequalityFunction<V>

InequalityFunction::less_equal(
    id: u64,
    name: &str,
    left: Linear<V>,
    right: V,
    big_m: V,
) -> InequalityFunction<V>
```

`InequalityKind` contains `LessEqual`, `GreaterEqual`, `Less`, `Greater`, `Equal`, and `NotEqual`; `result_variable()` returns the binary indicator and EQ/NE also allocate a side variable. Rust has a mechanism encoding for `NotEqual` as well as direct evaluation, matching the Kotlin encoding documented above. Rust has no Kotlin converter/tolerance parameters on the constructor; its mechanism uses fixed indicator tolerances and the supplied or inferred Big-M.

## Evaluate versus solver

Direct evaluation classifies with the relation's gap (`tolerance` for `LE`/`GE`, `strictBoundary` for `LT`/`GT`, and the distance band for `EQ`/`NE`) and returns `null` inside the gap. Solver registration uses Big-M rows with the same thresholds; the gap is infeasible there rather than undefined. `NE` is fully supported: direct evaluation works, and `registerConstraints` writes the four-row zero/nonzero encoding.

## Boundaries, tolerance, and Undefined

Missing polynomial symbols make `evaluate` return `null`, as does a value inside the relation's gap. Big-M must be positive, finite, representable, and large enough for the lhs-rhs range. `EQ` and `NE` additionally need a valid finite strict boundary for their side encoding. Invalid inputs (a non-positive Big-M, an invalid equality band) surface as a failed registration Result.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.InequalityFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val xPoly = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, x)), Flt64.zero
)
val inequality = InequalityFunction(
    lhs = xPoly,
    rhs = Flt64.one,
    sign = Comparison.LE,
    converter = IntoValue.Identity,
    bigM = Flt64(10.0),
    name = "ineq"
)
val value = inequality.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero))
check(value == Flt64.one)
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{InequalityFunction, InequalityKind};

let left = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let inequality = InequalityFunction::less_equal(
    1,
    "x_le_1",
    left,
    1.0_f64,
    10.0_f64,
);
assert_eq!(inequality.inequality_kind(), InequalityKind::LessEqual);
let _result = inequality.result_variable();
```

:::

- Dedicated core test: [`InequalityFunctionDedicatedTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/InequalityFunctionDedicatedTest.kt)
- Example directory (no dedicated inequality file): [linear_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function)

Rust source and parity coverage: [`inequality.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/inequality.rs) and [`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs).

## Related pages

- [Satisfied Amount](./satisfied-amount)
- [Satisfied Amount Inequality](./satisfied-amount-inequality)
- [Imply](./imply)
