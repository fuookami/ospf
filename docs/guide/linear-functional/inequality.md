# Inequality Indicator

`InequalityFunction` turns a comparison between a linear polynomial and a scalar into a binary result.

## Contract

- Input: `lhs: LinearPolynomial<V>`, scalar `rhs: V`, and a `Comparison` sign.
- Direct `evaluate` supports `LE`, `LT`, `GE`, `GT`, `EQ`, and `NE`.
- Output: `result`, a linear polynomial containing the binary flag.
- Solver registration supports `LE/LT/GE/GT/EQ`. `NE` is explicitly rejected by the current MIP encoding.
- Generic values use `V : RealNumber<V>, V : NumberField<V>` and an `IntoValue<V>` converter.

## Definition and mathematical model

Let $d=lhs-rhs$ and $y\in\{0,1\}$. The intended contract is

$$
y=\mathbf{1}[lhs\ \mathrel{\text{sign}}\ rhs].
$$

For `EQ`, the solver uses a zero-band with tolerance and a side binary variable. For the other supported signs, two Big-M inequalities link the flag to the satisfied and violated branches. The exact direct-evaluation comparison is separate from the solver tolerance encoding.

## Implementation, helper variables, and constraints

The implementation creates `name` followed by `_flag` for every sign and additionally `name` followed by `_side` for `EQ`. Big-M defaults to the finite range of `lhs-rhs` and otherwise follows the current default fallback. Registration adds two indicator inequalities for LE/LT/GE/GT; EQ delegates to the zero-indicator encoding; NE returns a failed Result before model write.

## Current API

### Kotlin

Source: [`Inequality.kt` (`InequalityFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Inequality.kt#L42-L205)

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

`InequalityKind` contains `LessEqual`, `GreaterEqual`, `Less`, `Greater`, `Equal`, and `NotEqual`; `result_variable()` returns the binary indicator and EQ/NE also allocate a side variable. Unlike the Kotlin implementation documented above, Rust has a mechanism encoding for `NotEqual` as well as direct evaluation. Rust has no Kotlin converter/tolerance parameters on the constructor; its mechanism uses fixed indicator tolerances and the supplied or inferred Big-M.

## Evaluate versus solver

Direct evaluation uses the sign's direct numeric comparison. Solver registration uses Big-M and tolerance; therefore values at or near a strict boundary can classify differently. In particular, direct `NE` evaluation works, but calling `registerConstraints` for `NE` returns a failed Result and writes no constraints.

## Boundaries, tolerance, and Undefined

Missing polynomial symbols make `evaluate` return `null`. Big-M must be positive, finite, representable, and large enough for the lhs-rhs range. EQ additionally needs a valid finite strict boundary for its side encoding. There is no `TruthValue.Undefined` return from this function; unsupported solver signs surface as a failed registration Result.

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

- There is no dedicated current `InequalityFunction` test in the core function-test directory: [function tests](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function)
- Example directory (no dedicated inequality file): [linear_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function)

Rust source and parity coverage: [`inequality.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/inequality.rs) and [`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs).

## Related pages

- [Satisfied Amount](./satisfied-amount)
- [Satisfied Amount Inequality](./satisfied-amount-inequality)
- [Imply](./imply)
