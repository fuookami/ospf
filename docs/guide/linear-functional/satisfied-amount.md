# Satisfied Amount

`SatisfiedAmountFunction` counts how many input linear inequalities are satisfied, or tests whether that count reaches a threshold.

## Contract

- Input: `List<LinearInequality<V>>`, with a caller-supplied `epsilon`.
- With `amount = null`, output `result` is the raw count $0,\ldots,n$.
- With `amount` set, output is binary: one iff the count is at least `amount`.
- Direct evaluation recognizes LE/LT/GE/GT/EQ/NE; solver registration currently supports only LE/GE/EQ and fails for LT/GT/NE.
- Each inequality uses a binary satisfaction flag; EQ also uses a side flag.
- Big-M defaults from each inequality's difference polynomial when possible.

## Definition and mathematical model

For inequality $i$, let

$$
u_i=\mathbf{1}[\text{inequality }i\text{ is satisfied}],\qquad
c=\sum_{i=0}^{n-1}u_i.
$$

The result is

$$
y=\begin{cases}
c,&\text{if amount is null},\\
\mathbf{1}[c\ge amount],&\text{otherwise}.
\end{cases}
$$

The threshold mode is “at least”, not exact-count equality.

## Solver mathematical model

Kotlin creates $u_i\in\{0,1\}$ and links every supported inequality to $u_i$ using the two relation rows (or the four-row zero-band model for `EQ`). It exposes

$$
c=\sum_i u_i.
$$

When `amount = k`, the only additional solver row is

$$
\sum_i u_i\ge k.
$$

The solver result remains the count $c$; Kotlin does **not** create the binary threshold result suggested by the direct-evaluation formula above. Rust receives already-created indicators, registers $y-\sum_i u_i=0$, and `with_amount_range` adds hard lower/upper bounds on that count.

## Current API

### Kotlin

Source: [`SatisfiedAmount.kt` (`SatisfiedAmountFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/SatisfiedAmount.kt#L42-L224)

The companion factory has the same public arguments. Use `amount = null` to retain the count rather than the threshold indicator.

```kotlin
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.SatisfiedAmountFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.inequality.LinearInequality
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val one = LinearPolynomial<Flt64>(emptyList(), Flt64.one)
val inequality = LinearInequality(xPoly, one, Comparison.LE, "x_le_1")
val satisfied = SatisfiedAmountFunction(
    inequalities = listOf(inequality),
    amount = UInt64.one,
    epsilon = Flt64(1e-6),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "satisfied"
)
val value = satisfied.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero))
check(value == Flt64.one)
```

### Rust

Rust currently has no direct counterpart that accepts `Vec<LinearInequality<V>>`, an epsilon, and a Kotlin-style threshold result. [`SatisfiedAmountFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/satisfied_amount.rs) counts already-created binary indicator variables:

```rust
SatisfiedAmountFunction::new(
    id: u64,
    name: &str,
    indicators: Vec<BinaryVariableItem>,
) -> Self

pub fn with_amount_range(
    mut self,
    lower: Option<usize>,
    upper: Option<usize>,
) -> Self
```

The convenience constructors are `any`, `all`, `at_least`, `not_all`, and `numerable`. `result_variable()` is a continuous count equal to the sum of the indicators; `with_amount_range` adds hard count bounds and does not create Kotlin's separate binary threshold result. Build each inequality indicator separately (for example with Rust `InequalityFunction`) and register both symbols in the model.

## Evaluate versus solver

Direct evaluation applies epsilon to each comparison and can count all six comparison kinds. Solver registration uses the supported encodings and returns a failed Result for LT, GT, or NE before model write. A non-positive or insufficient Big-M, missing symbols, or a difference range that cannot be inferred can make registration fail even when direct counting is defined.

## Boundaries, tolerance, and Undefined

Missing input values return `null` from direct `evaluate`. `amount` is converted to an Int-sized count by the current implementation; callers should keep it within the input-list size and the host Int range. There is no three-valued Undefined result; unsupported relation kinds are explicit registration failures.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.SatisfiedAmountFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.inequality.LinearInequality
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val one = LinearPolynomial<Flt64>(emptyList(), Flt64.one)
val inequality = LinearInequality(xPoly, one, Comparison.LE, "x_le_1")
val satisfied = SatisfiedAmountFunction(
    inequalities = listOf(inequality),
    amount = UInt64.one,
    epsilon = Flt64(1e-6),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "satisfied"
)
val value = satisfied.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero))
check(value == Flt64.one)
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{
    InequalityFunction, SatisfiedAmountFunction,
};

let x = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let le = InequalityFunction::less_equal(1, "x_le_1", x, 1.0_f64, 10.0_f64);
let satisfied = SatisfiedAmountFunction::<f64>::new(
    2,
    "satisfied",
    vec![le.result_variable().clone()],
)
.with_amount_range(Some(1), None);
let _count = satisfied.result_variable();
```

:::

- Core test: [`SatisfiedAmountFunctionsGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/SatisfiedAmountFunctionsGenericEvaluateTest.kt)
- Registration test: [`FunctionSymbolSameAsGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolSameAsGenericRegistrationTest.kt)
- Example directory (no dedicated satisfied-amount file): [linear_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function)

Register `le` and `satisfied` with the model to enforce the indicator and count constraints. Rust source: [`satisfied_amount.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/satisfied_amount.rs) and [`inequality.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/inequality.rs).

## Related pages

- [Inequality Indicator](./inequality)
- [Satisfied Amount Inequality](./satisfied-amount-inequality)
- [Same-As](./same-as)
