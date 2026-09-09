# In-Step Range

`InStepRangeFunction` rounds the difference between two linear expressions down to a multiple of a supplied step and adds it back to the lower expression. It returns a stepped numeric value; it is not a Boolean membership test.

## Contract

- Inputs: `lb: LinearPolynomial<V>`, `ub: LinearPolynomial<V>`, and scalar `step: V`.
- Result: $lb+\lfloor(ub-lb)/step\rfloor step$, exposed as `result`.
- `m` is the optional Big-M passed to the delegated `FloorFunction`.
- Generic values use `V : RealNumber<V>, V : NumberField<V>` and an `IntoValue<V>` converter.
- The current constructor does not validate that `step` is positive; callers must enforce a non-zero positive step.

## Definition and mathematical model

Let $d=ub-lb$. The implementation computes

$$
q=\left\lfloor\frac{d}{step}\right\rfloor,\qquad
y=lb+q\cdot step.
$$

For example, with $lb=1$, $ub=4$, and $step=2$, the result is $3$. The name “range” describes the endpoints used to form the difference; the function does not return one when a value belongs to a set.

## Implementation, helper variables, and constraints

The implementation builds a private `FloorFunction` over $ub-lb$, passing `m` as its Big-M, and scales its floor result by `step` before adding `lb`. Its helper variables and constraints are exactly those of the delegated floor function; there is no independent membership indicator.

## Current API

### Kotlin

Source: [`InStepRange.kt` (`InStepRangeFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/InStepRange.kt#L45-L140)

```kotlin
InStepRangeFunction(
    lb: LinearPolynomial<V>,
    ub: LinearPolynomial<V>,
    step: V,
    m: V? = null,
    converter: IntoValue<V>,
    name: String = "inStepRange",
    displayName: String? = null
)
```

### Rust

Rust exposes [`InStepRangeFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/in_step_range.rs), but its contract is not the Kotlin numeric stepping formula above. Rust tests whether one input lies in `[lower, upper]` and on the grid `lower + k * abs(step)`, returning a binary result:

```rust
InStepRangeFunction::new(
    id: u64,
    name: &str,
    input: Linear<V>,
    lower: V,
    upper: V,
    step: V,
) -> InStepRangeFunction<V>
```

`result_variable()` is a `BinaryVariableItem`; `input_polynomial()`, `lower_bound()`, `upper_bound()`, and `step()` expose the stored inputs. The evaluator uses an approximately `1e-8` tolerance, while registration expands the step points and refuses more than 4096 points. There is therefore no direct Rust equivalent of Kotlin's `lb + floor((ub - lb) / step) * step`; compose Rust [`FloorFunction::new`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/floor.rs) with caller-built linear expressions when that numeric result is required.

## Evaluate versus solver

Direct `evaluate` computes the floor using converted Flt64 arithmetic. Solver registration delegates to `FloorFunction`, so the same floor formulation and its tolerance/Big-M behavior are used. A zero or negative step can produce invalid arithmetic or a model that does not represent the intended stepping semantics; this is a caller precondition, not a constructor error.

## Boundaries, tolerance, and Undefined

Missing `lb` or `ub` symbols return `null`. There is no `TruthValue.Undefined` state. If $ub<lb$, the implementation still evaluates the mathematical floor expression (which can move below `lb`); it does not clamp or reject the ordering. Big-M inference must be sufficient for $ub-lb$.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.InStepRangeFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val lb = RealVar("lb")
val ub = RealVar("ub")
val lbPoly = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, lb)), Flt64.zero
)
val ubPoly = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, ub)), Flt64.zero
)
val stepped = InStepRangeFunction(
    lb = lbPoly,
    ub = ubPoly,
    step = Flt64.two,
    converter = IntoValue.Identity,
    name = "step"
)
val value = stepped.evaluate(
    mapOf<Symbol, Flt64>(lb to Flt64.one, ub to Flt64(4.0))
)
check(value == Flt64(3.0))
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::InStepRangeFunction;

let input = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let indicator = InStepRangeFunction::new(
    1,
    "in_step_range",
    input,
    0.0_f64,
    4.0_f64,
    2.0_f64,
);
assert_eq!(indicator.step(), &2.0);
let _result = indicator.result_variable();
```

:::

- Core test: [`FunctionSymbolDiscreteGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolDiscreteGenericEvaluateTest.kt)
- Example directory (no dedicated in-step-range file): [linear_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function)

Rust source: [`in_step_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/in_step_range.rs).

## Related pages

- [Floor](./floor)
- [Modulo](./mod)
- [Slack Range](./slack-range)
