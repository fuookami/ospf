# Quadratic Linear

`QuadraticLinearFunction` wraps a `QuadraticPolynomial<V>` as a quadratic intermediate symbol and conditionally introduces a result variable.

## Contract

- Input: `polynomial: QuadraticPolynomial<V>`.
- Direct evaluation returns the value of the wrapped polynomial.
- If the polynomial has no quadratic monomials, the symbol is categorized as linear and registers no helper variable or constraint.
- If a quadratic monomial exists, the implementation creates a non-negative real helper named by appending `_y` to `name` and registers $y=polynomial$.
- Generic values require `V : RealNumber<V>, V : Ring<V>, V : NumberField<V>` and an `IntoValue<V>` converter.

## Definition and mathematical model

For an input polynomial $p(x)$,

$$
y=p(x)
$$

is the registered equality only in the genuinely quadratic case. The public polynomial remains $p(x)$; the helper variable is a solver-side equality target, not a change to the mathematical expression.

## Implementation, helper variables, and constraints

The implementation checks `monomial.isQuadratic`. Purely linear input has category `Linear` and no helper registration. Otherwise it creates `URealVar` whose name is `name` + `_y`, registers one quadratic equality with the helper variable, and evaluates the original polynomial through token tables.

## Current API

### Kotlin

Source: [`QuadraticLinear.kt` (`QuadraticLinearFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticLinear.kt#L39-L325)

```kotlin
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticLinearFunction
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
val function = QuadraticLinearFunction(
    polynomial = polynomial,
    converter = IntoValue.Identity,
    name = "quadratic_linear"
)
val tokens = AutoTokenTable<Flt64>(Quadratic, false)
tokens.add(listOf(x, y))
val value = function.prepare(
    mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0)),
    tokens,
    IntoValue.Identity
)
check(value == Flt64(13.0))
tokens.close()
```

### Rust

Rust's [`QuadraticLinearFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_linear.rs) is an explicit bridge from a `Quadratic<V>` expression to a result variable:

```rust
QuadraticLinearFunction::new(id: u64, name: &str, input: Quadratic<V>)
    -> QuadraticLinearFunction<V>
```

The result variable is named `name + "_lin_y"`. `calculate_value` evaluates the input quadratic directly; `prepare` first uses a supplied result-variable value and otherwise evaluates the input. The mechanism emits a linear equality when the input has no quadratic monomials and a quadratic equality otherwise. Unlike the Kotlin implementation, Rust creates this bridge for every input rather than conditionally omitting it for a purely linear polynomial.

```rust
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticLinearFunction;

let polynomial = Quadratic::new(
    vec![
        QuadraticMonomial::new_quadratic(1.0, 0, 1),
        QuadraticMonomial::new_linear(1.0, 0),
    ],
    1.0,
);
let bridge = QuadraticLinearFunction::new(12, "quadratic_linear", polynomial);
assert!(bridge.result_variable().name().contains("quadratic_linear_lin_y"));
```

## Evaluate versus solver

Direct evaluation and `prepare` always evaluate the original polynomial. Solver registration adds the equality only for a genuinely quadratic input, so the helper variable and its non-negative domain can impose solver restrictions that are not present in a direct negative evaluation of the same polynomial.

## Boundaries, tolerance, and Undefined

The input polynomial must be evaluable and representable; missing symbols return `null`. There is no tolerance or three-valued undefined state. A negative quadratic value is valid for direct evaluation but cannot be represented by the generated `URealVar` helper without an appropriate model/domain change.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticLinearFunction
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
val function = QuadraticLinearFunction(
    polynomial = polynomial,
    converter = IntoValue.Identity,
    name = "quadratic_linear"
)
val tokens = AutoTokenTable<Flt64>(Quadratic, false)
tokens.add(listOf(x, y))
val value = function.prepare(
    mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0)),
    tokens,
    IntoValue.Identity
)
check(value == Flt64(13.0))
tokens.close()
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticLinearFunction;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
let y = ContinuousVariableItem::create(VariableId::standalone(1), "y");
let mut tokens = VecTokenList::<f64>::new();
let tx = Token::from_generic(x, 0);
tx.set_result(2.0);
tokens.add_token(tx);
let ty = Token::from_generic(y, 1);
ty.set_result(5.0);
tokens.add_token(ty);
let bridge = QuadraticLinearFunction::new(
    13,
    "qlinear",
    Quadratic::new(
        vec![
            QuadraticMonomial::new_quadratic(1.0, 0, 1),
            QuadraticMonomial::new_linear(1.0, 0),
        ],
        1.0,
    ),
);
assert_eq!(bridge.calculate_value(&tokens, false), Some(13.0));
```

:::

- Core evaluation: [`QuadraticFunctionGenericEvaluationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionGenericEvaluationTest.kt)
- Core registration: [`FunctionSymbolGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolGenericRegistrationTest.kt)
- Example directory (no dedicated quadratic-linear file): [quadratic_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/quadratic_function)

- Rust implementation and tests: [`quadratic_linear.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_linear.rs) and [`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)

## Related pages

- [Quadratic Product](./product)
- [Quadratic In-Step Range](./quadratic-in-step-range)
