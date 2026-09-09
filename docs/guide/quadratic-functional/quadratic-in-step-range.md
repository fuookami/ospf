# Quadratic In-Step Range

`QuadraticInStepRangeFunction` gates a quadratic polynomial by a closed interval: it returns the polynomial when the value is in range and zero otherwise.

> [!WARNING]
> Despite the shared “in-step” name, this quadratic implementation does not round to a step. It is an interval gate.

## Contract

- Input: `x: QuadraticPolynomial<V>` and scalar bounds `lower`/`upper`.
- The constructor requires $lower\le upper$.
- Direct evaluation returns `x` for $lower\le x\le upper$, otherwise zero; missing symbols return `null`.
- Solver helpers are binary `z` and real `y`; the public polynomial is the helper result.
- Generic values require `V : RealNumber<V>, V : Ring<V>, V : NumberField<V>` and an `IntoValue<V>` converter.

## Definition and mathematical model

For $p=x$ and $z,y$,

$$
z=1\Longleftrightarrow lower\le p\le upper,\qquad
y=\begin{cases}p,&z=1,\\0,&z=0.\end{cases}
$$

The registration emits the six current Big-M constraints:

$$
\begin{aligned}
p+M(1-z)&\ge lower,&p-M(1-z)&\le upper,\\
y-p+M(1-z)&\ge0,&y-p-M(1-z)&\le0,\\
y&\le Mz,&y\ge-Mz.
\end{aligned}
$$

## Implementation, helper variables, and constraints

The implementation creates `name` + `_z` as a binary variable and `name` + `_y` as a real variable. It registers both, then adds the interval gate, equality-on, and zero-off constraints above. Big-M defaults to the quadratic input's inferred bound when omitted.

## Current API

### Kotlin

Source: [`QuadraticInStepRange.kt` (`QuadraticInStepRangeFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticInStepRange.kt#L48-L375)

```kotlin
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticInStepRangeFunction
import fuookami.ospf.kotlin.core.token.AutoTokenTable
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Quadratic
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val y = RealVar("y")
val polynomial = QuadraticPolynomial(
    monomials = listOf(
        QuadraticMonomial.quadratic(Flt64.one, x, y),
        QuadraticMonomial.linear(Flt64.one, x)
    ),
    constant = Flt64.one
)
val function = QuadraticInStepRangeFunction(
    x = polynomial,
    lower = Flt64.zero,
    upper = Flt64(10.0),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "quadratic_step"
)
val tokens = AutoTokenTable<Flt64>(Quadratic, false)
tokens.add(listOf(x, y))
val value = function.prepare(
    mapOf<Symbol, Flt64>(x to Flt64.one, y to Flt64.two),
    tokens,
    IntoValue.Identity
)
check(value == Flt64(4.0))
tokens.close()
```

### Rust

Rust's [`QuadraticInStepRangeFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_in_step_range.rs) is a different, step-valued operation from the Kotlin interval gate. Its public constructors are:

```rust
QuadraticInStepRangeFunction::new(
    id: u64,
    name: &str,
    input: Quadratic<V>,
    lower: V,
    upper: V,
    step: V,
) -> QuadraticInStepRangeFunction<V>

QuadraticInStepRangeFunction::with_quadratic_bounds(
    id: u64,
    name: &str,
    lower: Quadratic<V>,
    upper: Quadratic<V>,
    step: V,
) -> QuadraticInStepRangeFunction<V>
```

`new` treats `input` as the runtime upper bound, uses a scalar lower bound, and applies `upper` as a hard cap. Direct evaluation returns `lower + floor((min(upper_value, cap) - lower) / |step|) * |step|`; a near-zero step returns the lower bound. The implementation creates a result variable named `name + "_in_step_range"` and bridges the quadratic bounds through `QuadraticLinearFunction`. It therefore has no one-to-one solver contract with the Kotlin closed-interval gate.

## Evaluate versus solver

Direct evaluation only computes $p$ and tests the closed interval; it does not read the helper z/y values. Solver registration creates z/y and enforces the six constraints. Missing or insufficient Big-M can make the solver relaxation inaccurate or infeasible even when direct evaluation is defined.

## Boundaries, tolerance, and Undefined

The lower and upper bounds are inclusive. Reversed bounds fail in the constructor. There is no tolerance or three-valued Undefined state; values just outside the interval return zero in direct evaluation. The control variable is intended to be binary, but the constructor accepts a generic variable item and does not itself enforce that type.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticInStepRangeFunction
import fuookami.ospf.kotlin.core.token.AutoTokenTable
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Quadratic
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val y = RealVar("y")
val polynomial = QuadraticPolynomial(
    monomials = listOf(
        QuadraticMonomial.quadratic(Flt64.one, x, y),
        QuadraticMonomial.linear(Flt64.one, x)
    ),
    constant = Flt64.one
)
val function = QuadraticInStepRangeFunction(
    x = polynomial,
    lower = Flt64.zero,
    upper = Flt64(10.0),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "quadratic_step"
)
val tokens = AutoTokenTable<Flt64>(Quadratic, false)
tokens.add(listOf(x, y))
val value = function.prepare(
    mapOf<Symbol, Flt64>(x to Flt64.one, y to Flt64.two),
    tokens,
    IntoValue.Identity
)
check(value == Flt64(4.0))
tokens.close()
```

```rust [Rust]
use ospf_rust_core::symbol::{FunctionSymbol};
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticInStepRangeFunction;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
let upper = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
let step = QuadraticInStepRangeFunction::new(11, "qstep", upper, 0.0, 4.0, 2.0);
let mut tokens = VecTokenList::<f64>::new();
let tx = Token::from_generic(x, 0);
tx.set_result(3.0);
tokens.add_token(tx);
assert_eq!(step.calculate_value(&tokens, false), Some(2.0));
```

:::

- Core evaluation: [`QuadraticFunctionGenericEvaluationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionGenericEvaluationTest.kt)
- Core registration: [`FunctionSymbolGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolGenericRegistrationTest.kt)
- Example directory (no dedicated quadratic in-step-range file): [quadratic_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/quadratic_function)

- Source and focused evaluation/integration coverage: [`quadratic_in_step_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_in_step_range.rs) and [`quadratic_function.rs` tests](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)

## Related pages

- [In-Step Range](../linear-functional/in-step-range)
- [Quadratic Masking Range](./quadratic-masking-range)
