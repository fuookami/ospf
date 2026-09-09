# First Nonzero Index

`FirstFunction` returns the zero-based index of the first input polynomial whose value is strictly greater than `epsilon`. If no input passes that test, it returns the number of inputs.

## Contract

- Input: an ordered, normally non-empty `List<LinearPolynomial<V>>`.
- Output: a numeric index in $[0,n]$, exposed as the linear polynomial `result`.
- Selection condition: `p_i > epsilon`, not `abs(p_i) > epsilon`; negative values never qualify.
- `epsilon` is an `Flt64` parameter (default `1e-6`), while coefficients/results use the generic `V` converter.
- Missing input values make direct `evaluate` return `null`.

## Definition and mathematical model

Let $p_0,\ldots,p_{n-1}$ be the input values and

$$
b_i=\mathbf{1}[p_i>\varepsilon].
$$

Let $y_i$ be the one-hot first-hit flags:

$$
y_i=\mathbf{1}\!\left[b_i=1\land\sum_{j<i}y_j=0\right].
$$

The returned index is

$$
r=\sum_{i=0}^{n-1} i\,y_i+n\left(1-\sum_{i=0}^{n-1}y_i\right).
$$

Consequently, an all-false list returns $n$, not `null`.

## Implementation, helper variables, and constraints

For each input, the implementation creates a `BinaryzationFunction`. It also creates a binary array `name` followed by `_first` with one entry per input. The binaryization flags and first-hit flags are linked by upper/lower/monotonic constraints; `result` is the weighted expression above. Registration therefore needs a valid Big-M range for every input polynomial (inferred by the binaryization helper unless explicitly supplied there).

## Current API

### Kotlin

Source: [`First.kt` (`FirstFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/First.kt#L53-L215)

```kotlin
FirstFunction(
    polynomials: List<LinearPolynomial<V>>,
    epsilon: Flt64 = Flt64(1e-6),
    converter: IntoValue<V>,
    name: String,
    displayName: String? = null
)
```

### Rust

Source: [`first.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/first.rs)

Rust has a same-named helper, but it is not a one-to-one replacement for Kotlin's thresholding function. `FirstFunction::new` receives the candidate `Linear<V>` values and a same-length `Vec<BinaryVariableItem>` of already-computed conditions; it has no `epsilon` parameter and does not create `BinaryzationFunction`s internally. The public accessors are `result_variable()`, `polynomials()`, and `condition_variables()`. With no active condition, direct evaluation returns `None` (or zero when `zero_if_none` is true) and the mechanism constraints force result zero, whereas Kotlin returns the input count `n`. Compose one `BinaryzationFunction` per input and pass its indicator variables when the Kotlin threshold contract is required.

```rust
FirstFunction::new(
    id: u64,
    name: &str,
    polynomials: Vec<Linear<V>>,
    conditions: Vec<BinaryVariableItem>,
) -> Self
FirstFunction::result_variable(&self) -> &ContinuousVariableItem
FirstFunction::polynomials(&self) -> &[Linear<V>]
FirstFunction::condition_variables(&self) -> &[BinaryVariableItem]
```

## Evaluate versus solver

Direct evaluation scans the list from index 0 and uses the caller's `epsilon`. Solver registration first builds binaryization functions, whose current constraint tolerance is the shared `NONZERO_TOLERANCE`, then links the first-hit array. With a non-default `epsilon`, direct evaluation and solver classification can therefore disagree near the threshold.

## Boundaries, tolerance, and Undefined

The first-hit result is not a Boolean and is not one-based. Equality with `epsilon` is not selected because the test is strict `>`. An empty list is not a useful model input; callers should provide at least one polynomial. Missing symbols produce `null` during direct evaluation; failed Big-M inference or invalid polynomial values fail registration through the underlying binaryization path.

## Minimal current example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.FirstFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x0 = RealVar("x0")
val x1 = RealVar("x1")
val first = FirstFunction(
    polynomials = listOf(
        LinearPolynomial(listOf(LinearMonomial(Flt64.one, x0)), Flt64.zero),
        LinearPolynomial(listOf(LinearMonomial(Flt64.one, x1)), Flt64.zero)
    ),
    converter = IntoValue.Identity,
    name = "first"
)
val value = first.evaluate(
    mapOf<Symbol, Flt64>(x0 to Flt64.zero, x1 to Flt64.two)
)
check(value != null && value == Flt64.one)
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::FirstFunction;
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{BinaryVariableItem, VariableId};

let c0 = BinaryVariableItem::create(VariableId::standalone(1), "c0");
let c1 = BinaryVariableItem::create(VariableId::standalone(2), "c1");
let function = FirstFunction::new(
    1,
    "first",
    vec![Linear::new(vec![], 10.0), Linear::new(vec![], 20.0)],
    vec![c0.clone(), c1.clone()],
);
let mut tokens = VecTokenList::new();
let token0 = Token::from_generic(c0.clone(), c0.index());
token0.set_result(0.0);
tokens.add_token(token0);
let token1 = Token::from_generic(c1.clone(), c1.index());
token1.set_result(1.0);
tokens.add_token(token1);
let value = <FirstFunction as FunctionSymbol>::calculate_value(&function, &tokens, false);
assert_eq!(value, Some(20.0));
```

:::

## Tests and examples

- Core test: [`FirstFunctionGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FirstFunctionGenericEvaluateTest.kt)
- Example directory (no dedicated first-index file): [linear_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function)
- Rust implementation and evaluation: [`first.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/first.rs), [`p0_evaluation_tests.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/p0_evaluation_tests.rs)

## Related pages

- [Binaryzation](./bin)
- [If](./if)
- [Univariate Linear Piecewise](./ulp)
