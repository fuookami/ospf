# Same-As

`SameAsFunction` returns one when all input inequalities have the same satisfaction status: either all are satisfied or all are unsatisfied.

## Contract

- Input: a non-empty `List<LinearInequality<V>>`.
- `constraint = true` (the default) forces all satisfaction flags to be equal in the solver model.
- `constraint = false` measures equality of the flags without forcing the input inequalities to agree.
- Output: binary `resultPolynomial`; direct evaluation is one when all statuses match and zero otherwise.
- `epsilon` is the comparison tolerance and `m` is the optional Big-M for each input indicator.

## Definition and mathematical model

Let $u_i\in\{0,1\}$ indicate whether inequality $i$ is satisfied. The function is

$$
y=\mathbf{1}[u_0=u_1=\cdots=u_{n-1}].
$$

When `constraint` is true, the model adds

$$
u_0-u_i=0\quad(i=1,\ldots,n-1),\qquad y=u_0.
$$

When `constraint` is false and $n>1$, it creates difference flags $d_i=|u_i-u_0|$ and adds

$$
y+\sum_{i=1}^{n-1}d_i=1.
$$

For one input, measurement mode fixes $y=1$.

## Solver mathematical model

Kotlin first registers one satisfaction indicator $u_i\in\{0,1\}$ per inequality using the relation rows documented on [Inequality Indicator](./inequality). In constraint mode it then passes

$$
u_0-u_i=0\quad(1\le i<n),\qquad y-u_0=0.
$$

In measurement mode it creates $d_i=|u_i-u_0|$ with binary absolute-difference rows and passes

$$
y+\sum_{i=1}^{n-1}d_i=1.
$$

Rust's narrower pairwise function instead applies the shared zero-band Big-M encoding to $p-q$ and exposes the equality flag; it has no Kotlin-style list or hard-constraint mode.

## Current API

### Kotlin

Source: [`SameAs.kt` (`SameAsFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/SameAs.kt#L46-L305)

```kotlin
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.SameAsFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.inequality.LinearInequality
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val y = RealVar("y")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val yPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, y)), Flt64.zero)
val zero = LinearPolynomial<Flt64>(emptyList(), Flt64.zero)
val inequalities = listOf(
    LinearInequality(xPoly, zero, Comparison.LE, "x_le_0"),
    LinearInequality(yPoly, zero, Comparison.LE, "y_le_0")
)
val same = SameAsFunction(
    inequalities = inequalities,
    constraint = false,
    epsilon = Flt64(1e-6),
    m = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "same"
)
val value = same.evaluate(
    mapOf<Symbol, Flt64>(x to Flt64.zero, y to Flt64.one)
)
check(value == Flt64.zero)
```

### Rust

Rust provides [`SameAsFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/same_as.rs), but its shape is intentionally narrower than Kotlin's inequality-list API:

```rust
SameAsFunction::new(
    id: u64,
    name: &str,
    first: Linear<V>,
    second: Linear<V>,
    tolerance: V,
) -> SameAsFunction<V>
```

It compares two linear expressions within `tolerance`, creates a binary `result_variable()`, and has no Rust `constraint` switch or list of `LinearInequality` inputs. Thus it is the closest pair-equality API, not a one-to-one port of Kotlin's “all inequality statuses agree” function.

## Evaluate versus solver

Direct evaluation computes each inequality status and always returns the measurement $y$, regardless of the `constraint` parameter. Solver registration differs: `constraint = true` makes the statuses equal as a hard constraint, while `constraint = false` links the measurement result to their equality. Near an equality or strict boundary, direct comparisons use the implementation's epsilon rules and indicator constraints use Big-M/tolerance.

## Boundaries, tolerance, and Undefined

The inequality list must be non-empty (`init` enforces this). Missing symbols make direct evaluation return `null`. The caller must provide sufficient finite Big-M values or allow inference from each difference polynomial. This function does not return `TruthValue.Undefined`; invalid registration is a failed Result.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.SameAsFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.inequality.LinearInequality
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val y = RealVar("y")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val yPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, y)), Flt64.zero)
val zero = LinearPolynomial<Flt64>(emptyList(), Flt64.zero)
val inequalities = listOf(
    LinearInequality(xPoly, zero, Comparison.LE, "x_le_0"),
    LinearInequality(yPoly, zero, Comparison.LE, "y_le_0")
)
val same = SameAsFunction(
    inequalities = inequalities,
    constraint = false,
    epsilon = Flt64(1e-6),
    m = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "same"
)
val value = same.evaluate(
    mapOf<Symbol, Flt64>(x to Flt64.zero, y to Flt64.one)
)
check(value == Flt64.zero)
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::SameAsFunction;

let first = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let second = Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0);
let same = SameAsFunction::new(1, "same", first, second, 1.0e-6_f64);
let _result = same.result_variable();
```

:::

- Core test: [`FunctionSymbolSameAsGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolSameAsGenericRegistrationTest.kt)
- Example directory (no dedicated same-as file): [linear_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function)

Rust source: [`same_as.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/same_as.rs).

## Related pages

- [Inequality Indicator](./inequality)
- [Satisfied Amount](./satisfied-amount)
- [Logical XOR](./xor)
