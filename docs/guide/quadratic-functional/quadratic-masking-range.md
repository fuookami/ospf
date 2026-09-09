# Quadratic Masking Range

`QuadraticMaskingRangeFunction` gates a quadratic polynomial with a caller-supplied control variable.

> [!WARNING]
> Kotlin's name says “range”, but its current implementation has no lower/upper range parameters. When `z=1` it enforces `y=polynomial`; when `z=0`, `y` is free inside its variable bounds. It does not enforce `y=0` in the off branch. Rust has a separate `QuadraticMaskingRangeFunction` contract described below.

## Contract

- Input: `polynomial: QuadraticPolynomial<V>` and control `z: AbstractVariableItem<*, *>`, intended to be binary.
- Output/helper: real `resultVar` named by appending `_y` to `name`.
- Direct evaluation returns zero for missing/zero z and the polynomial value for any other z value.
- Solver registration uses two Big-M quadratic inequalities linking y and polynomial when z is one.
- Generic values require `V : RealNumber<V>, V : Ring<V>, V : NumberField<V>` and an `IntoValue<V>` converter.

## Definition and mathematical model

Let $p$ be the quadratic input and $y$ the helper result:

$$
y-p\le M(1-z),\qquad y-p\ge-M(1-z).
$$

Thus $z=1\Rightarrow y=p$, but $z=0$ only relaxes the relation. The direct evaluator uses the stronger procedural convention $z=0\Rightarrow0$, which is not fully enforced by the current solver constraints.

## Implementation, helper variables, and constraints

The implementation creates one real `resultVar` and registers it. It emits the two quadratic Big-M inequalities above; it does not add the two zero-off bounds used by the linear masking function. Big-M defaults to the inferred range of the quadratic polynomial.

## Current API

### Kotlin

Source: [`QuadraticMaskingRange.kt` (`QuadraticMaskingRangeFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticMaskingRange.kt#L41-L345)

```kotlin
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticMaskingRangeFunction
import fuookami.ospf.kotlin.core.token.AutoTokenTable
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Quadratic
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial
import fuookami.ospf.kotlin.core.variable.BinVar
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val y = RealVar("y")
val z = BinVar("z")
val polynomial = QuadraticPolynomial(
    monomials = listOf(
        QuadraticMonomial.quadratic(Flt64.one, x, y),
        QuadraticMonomial.linear(Flt64.one, x)
    ),
    constant = Flt64.one
)
val function = QuadraticMaskingRangeFunction(
    polynomial = polynomial,
    z = z,
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "quadratic_mask"
)
val tokens = AutoTokenTable<Flt64>(Quadratic, false)
tokens.add(listOf(x, y, z))
val value = function.prepare(
    mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0), z to Flt64.one),
    tokens,
    IntoValue.Identity
)
check(value == Flt64(13.0))
tokens.close()
```

### Rust

Rust provides a distinct [`QuadraticMaskingRangeFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_masking_range.rs). It is not the Kotlin “mask a polynomial with z” operation. The constructors are:

```rust
QuadraticMaskingRangeFunction::new(
    id: u64,
    name: &str,
    mask: Quadratic<V>,
    lower: V,
    upper: V,
) -> QuadraticMaskingRangeFunction<V>

QuadraticMaskingRangeFunction::with_quadratic_bounds(
    id: u64,
    name: &str,
    mask: Quadratic<V>,
    lower: Quadratic<V>,
    upper: Quadratic<V>,
) -> QuadraticMaskingRangeFunction<V>
```

Rust creates `name + "_masking_range"` as the result variable, bridges the mask and both bounds, and registers the quadratic inequalities `result <= upper * mask` and `result >= lower * mask`. Direct evaluation returns zero when the mask is numerically zero; otherwise it clamps the solved result token to the interval whose endpoints are `lower * mask` and `upper * mask` (ordered at runtime). There is no binary `z` parameter and no direct `y = polynomial` branch, so the two APIs are not interchangeable.

## Evaluate versus solver

Direct evaluation checks z procedurally and returns zero when z is exactly zero. Solver registration only links y to p when z=1; with z=0, y can take any value allowed by its variable domain, subject only to the Big-M relaxation. A non-binary nonzero z is also accepted by direct evaluation but is outside the intended binary solver contract.

## Boundaries, tolerance, and Undefined

Missing z returns zero in direct evaluation; missing polynomial symbols return `null` only when z is nonzero. Big-M must cover the quadratic range. There is no tolerance or three-valued Undefined state. Because `resultVar` is a `RealVar`, the solver result uses the variable's current domain; consult that domain before relying on negative off-branch values.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticMaskingRangeFunction
import fuookami.ospf.kotlin.core.token.AutoTokenTable
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Quadratic
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial
import fuookami.ospf.kotlin.core.variable.BinVar
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val y = RealVar("y")
val z = BinVar("z")
val polynomial = QuadraticPolynomial(
    monomials = listOf(
        QuadraticMonomial.quadratic(Flt64.one, x, y),
        QuadraticMonomial.linear(Flt64.one, x)
    ),
    constant = Flt64.one
)
val function = QuadraticMaskingRangeFunction(
    polynomial = polynomial,
    z = z,
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "quadratic_mask"
)
val tokens = AutoTokenTable<Flt64>(Quadratic, false)
tokens.add(listOf(x, y, z))
val value = function.prepare(
    mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0), z to Flt64.one),
    tokens,
    IntoValue.Identity
)
check(value == Flt64(13.0))
tokens.close()
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticMaskingRangeFunction;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

let mask_var = ContinuousVariableItem::create(VariableId::standalone(0), "mask");
let mask = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
let range = QuadraticMaskingRangeFunction::new(14, "qmask_range", mask, -2.0, 3.0);
let mut tokens = VecTokenList::<f64>::new();
let tm = Token::from_generic(mask_var, 0);
tm.set_result(1.0);
tokens.add_token(tm);
let ty = Token::from_generic(range.result_variable().clone(), range.result_variable().index());
ty.set_result(2.5);
tokens.add_token(ty);
assert_eq!(range.calculate_value(&tokens, false), Some(2.5));
```

:::

- Core evaluation: [`QuadraticFunctionGenericEvaluationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionGenericEvaluationTest.kt)
- Core registration: [`FunctionSymbolGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolGenericRegistrationTest.kt)
- Example directory (no dedicated quadratic masking-range file): [quadratic_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/quadratic_function)

- Source and focused tests: [`quadratic_masking_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_masking_range.rs) and [`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)

## Related pages

- [Masking](../linear-functional/masking)
- [Quadratic In-Step Range](./quadratic-in-step-range)
