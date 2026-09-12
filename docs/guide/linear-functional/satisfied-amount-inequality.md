# Satisfied-Amount Inequality

`SatisfiedAmountInequalityFunction` counts flattened linear constraint inputs and, when an amount range is supplied, returns a binary indicator for whether the count lies in that range. The page also covers its `AnyFunction`, `AllFunction`, `AtLeastInequalityFunction`, `NotAllFunction`, and `NumerableFunction` variants.

## Contract

- Input: `List<LinearConstraintInput<V>>`, not a list of ordinary `LinearInequality` values.
- Each `LinearConstraintInput` carries a flattened relation, an `lhsRange`, and an `rhsConstant`; the range is used by solver registration to build the flag encoding.
- With `amount = null`, output is the raw satisfied count.
- With `amount = [l,u]`, output is one iff $l\le count\le u$.
- `epsilon` controls direct boundary checks; the `from` factories accept an `Flt64` epsilon and convert it through `IntoValue<V>`.

## Definition and mathematical model

For each input, let $u_i$ be one when its flattened relation to zero is satisfied and zero otherwise. Then

$$
c=\sum_{i=0}^{n-1}u_i.
$$

The base result is

$$
y=\begin{cases}
c,&amount=null,\\
\mathbf{1}[l\le c\le u],&amount=[l,u].
\end{cases}
$$

The convenience variants are:

| API | Internal amount range | Meaning |
| --- | --- | --- |
| `AnyFunction` | $[1,n]$ | at least one |
| `AllFunction` | $[n,n]$ | all |
| `AtLeastInequalityFunction(k)` | $[k,n]$ | at least k |
| `NotAllFunction` | $[1,n-1]$ when $n>1$ | not all |
| `NumerableFunction(amount)` | caller supplied | count in a range |

## Solver mathematical model

Kotlin creates one $u_i\in\{0,1\}$ per flattened constraint and links it with the two normalized relation rows from [Inequality Indicator](./inequality). Let $c=\sum_i u_i$. Without an amount range, $c$ is the result. For range $[l,u]$, Kotlin additionally creates $y\in\{0,1\}$ and passes the relaxed range rows

$$
c\ge l\,y,
\qquad
c\le u+n(1-y).
$$

Thus $y=1\Rightarrow l\le c\le u$; the reverse implication is not forced by these two rows alone. Rust wrappers instead receive existing indicators, register $r-\sum_i u_i=0$, and apply the requested count range as hard bounds, without Kotlin's separate $y$.

> [!WARNING]
> The current registration loop only encodes an input when both range bounds are present. An input with a missing bound can leave its flag without a corresponding solver constraint; provide finite, ordered `lhsRange` values.

## Current API

### Kotlin

Source: [`SatisfiedAmountInequality.kt` (`SatisfiedAmountInequalityFunction` and variants)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/SatisfiedAmountInequality.kt#L56-L526)

Use `SatisfiedAmountInequalityFunction.from` for raw count or a custom amount range, and the variant `from` factories for the convenience classes. The variants share the same flags and amount indicator; they are wrappers over one base implementation.

```kotlin
import fuookami.ospf.kotlin.core.model.mechanism.LinearConstraintInput
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.AnyFunction
import fuookami.ospf.kotlin.core.symbol.function.SatisfiedAmountInequalityFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.value_range.Interval
import fuookami.ospf.kotlin.math.algebra.value_range.ValueRange
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.inequality.LinearInequality
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val one = LinearPolynomial<Flt64>(emptyList(), Flt64.one)
val lhsRange = ValueRange(
    lb = Flt64(-1000.0),
    ub = Flt64(1000.0),
    lbInterval = Interval.Closed,
    ubInterval = Interval.Closed,
    constants = Flt64.zero.constants
).value!!
val input = LinearConstraintInput.from(
    relation = LinearInequality(xPoly, one, Comparison.LE, "x_le_1"),
    lhsRange = lhsRange,
    rhsConstant = Flt64.one
).value!!
val any = AnyFunction.from(
    inputs = listOf(input),
    converter = IntoValue.Identity,
    name = "any"
)
val value = any.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero))
check(value == Flt64.one)
```

### Rust

Rust has no direct `SatisfiedAmountInequalityFunction` counterpart that accepts Kotlin `LinearConstraintInput` values, `lhsRange`, `rhsConstant`, and epsilon. The Rust module provides thin indicator-count wrappers over [`SatisfiedAmountFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/satisfied_amount.rs):

```rust
AnyFunction::new(id: u64, name: &str, indicators: Vec<BinaryVariableItem>) -> Self
AllFunction::new(id: u64, name: &str, indicators: Vec<BinaryVariableItem>) -> Self
AtLeastInequalityFunction::new(
    id: u64,
    name: &str,
    indicators: Vec<BinaryVariableItem>,
    amount: usize,
) -> Self
NotAllFunction::new(id: u64, name: &str, indicators: Vec<BinaryVariableItem>) -> Self
NumerableFunction::new(
    id: u64,
    name: &str,
    indicators: Vec<BinaryVariableItem>,
    lower: usize,
    upper: usize,
) -> Self
```

All five Rust APIs consume already-created binary indicators and expose a continuous count through `result_variable()`; their amount ranges are hard bounds, not Kotlin's separate binary amount indicator. To compose from an inequality, create an `InequalityFunction` indicator first and pass `result_variable().clone()` to one of these wrappers. Finite range and Big-M responsibility remains with that indicator function.

## Evaluate versus solver

Direct evaluation computes the flattened input value and compares it with zero using `epsilon`. Solver registration uses the declared finite ranges and Big-M inequalities; it does not derive missing ranges from the ordinary relation automatically. With an amount range, direct evaluation checks the closed range; solver uses a binary indicator and relaxed lower/upper count constraints.

## Boundaries, tolerance, and Undefined

Missing symbols make direct evaluation return `null`. An empty input list is not a useful count model; `NotAllFunction` deliberately uses `amount = null` for a one-input list, so its direct result is the raw count rather than a Boolean “not all”. The current constructors do not all use the same validation mechanism: `AtLeastInequalityFunction` uses assertions for $0<k\le n$. This function has no `TruthValue.Undefined` output.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.model.mechanism.LinearConstraintInput
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.AnyFunction
import fuookami.ospf.kotlin.core.symbol.function.SatisfiedAmountInequalityFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.value_range.Interval
import fuookami.ospf.kotlin.math.algebra.value_range.ValueRange
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.inequality.LinearInequality
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val one = LinearPolynomial<Flt64>(emptyList(), Flt64.one)
val lhsRange = ValueRange(
    lb = Flt64(-1000.0),
    ub = Flt64(1000.0),
    lbInterval = Interval.Closed,
    ubInterval = Interval.Closed,
    constants = Flt64.zero.constants
).value!!
val input = LinearConstraintInput.from(
    relation = LinearInequality(xPoly, one, Comparison.LE, "x_le_1"),
    lhsRange = lhsRange,
    rhsConstant = Flt64.one
).value!!
val any = AnyFunction.from(
    inputs = listOf(input),
    converter = IntoValue.Identity,
    name = "any"
)
val value = any.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero))
check(value == Flt64.one)
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{AnyFunction, InequalityFunction};

let x = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let le = InequalityFunction::less_equal(1, "x_le_1", x, 1.0_f64, 10.0_f64);
let any = AnyFunction::<f64>::new(
    2,
    "any",
    vec![le.result_variable().clone()],
);
assert_eq!(any.amount_range(), (Some(1), None));
let _count = any.result_variable();
```

:::

- Core evaluate test: [`SatisfiedAmountFunctionsGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/SatisfiedAmountFunctionsGenericEvaluateTest.kt)
- Core registration test: [`FunctionSymbolSatisfiedAmountInequalityGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolSatisfiedAmountInequalityGenericRegistrationTest.kt)
- Example directory (no dedicated satisfied-amount-inequality file): [linear_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function)

Register `le` and `any` together in the model. Rust source: [`satisfied_amount_inequality.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/satisfied_amount_inequality.rs) and [`inequality.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/inequality.rs).

## Related pages

- [Inequality Indicator](./inequality)
- [Satisfied Amount](./satisfied-amount)
- [Same-As](./same-as)
