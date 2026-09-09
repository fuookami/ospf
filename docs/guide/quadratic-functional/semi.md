# Semi-Continuous Marker in a Quadratic Model

## Availability

Kotlin has no quadratic-specific `SemiFunction` and no overload accepting `QuadraticPolynomial<V>`. Its only class is the marker [`SemiFunction`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Semi.kt#L37-L107). It accepts only bounds and a converter; it has no expression argument. Rust has both a functional generic `SemiFunction` and a direct quadratic-input `QuadraticSemiFunction`; their APIs are documented below.

The quadratic mechanism can fall back to registering a linear `MathFunctionSymbolBase` (`MechanismModel.kt:1350-1355`), but that fact does not turn `SemiFunction` into a quadratic positive-part function. `SemiFunction` has no helper variables or constraints and is not a quadratic expression. It may be constructed and retained as metadata alongside a quadratic model, but adding it does not change that model.

## Meaning and boundaries

The marker describes the intended domain

$$
y = 0 \quad\text{or}\quad lb \le y \le ub.
$$

The constructor is:

```kotlin
SemiFunction(
    lb: V? = null,
    ub: V? = null,
    converter: IntoValue<V>,
    name: String = "semi",
    displayName: String? = null
)
```

Defaults are `lb = 0` and `ub = 1e6`; `lb <= ub` is required (`Semi.kt:37-50`). `SemiFunction.from(variable, ...)` can infer missing bounds from a variable's finite range (`Semi.kt:89-106`). `helperVariables` is empty, `evaluate` always returns `null`, and registration is a no-op (`Semi.kt:53-65`). It therefore cannot represent `max(0,q)` for a quadratic polynomial `q`.

## Current API

### Kotlin

This is a marker-only construction, matching the current quadratic smoke test:

For a quadratic expression, write the required domain constraints in the `QuadraticMetaModel` explicitly or use a quadratic function class that actually accepts `QuadraticPolynomial`; do not invent or call a quadratic `SemiFunction` overload.

```kotlin
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.core.model.mechanism.QuadraticMetaModel
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.SemiFunction

val model = QuadraticMetaModel<Flt64>(
    name = "quadratic-model-with-semi-marker",
    converter = IntoValue.Identity
)
val semi = SemiFunction(
    lb = Flt64.one,
    ub = Flt64(4.0),
    converter = IntoValue.Identity,
    name = "semi"
)
check(semi.helperVariables.isEmpty())
check(semi.evaluate(emptyMap()) == null)
// Keep `semi` as metadata; it has no quadratic expression or model constraints.
model.close()
```

### Rust

Rust's [`QuadraticSemiFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs) is the direct quadratic counterpart that Kotlin currently lacks:

```rust
QuadraticSemiFunction::new(
    id: u64,
    name: &str,
    input: Quadratic<V>,
) -> QuadraticSemiFunction<V>
```

It bridges `input` through `QuadraticLinearFunction` and an exact two-candidate `MaxFunction(input, 0)`. Direct evaluation is `max(input, 0)`, and `result_variable` returns the inner non-negative result variable. Big-M for selector constraints is inferred from token bounds when possible. Rust also has the separate [`SemiFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/semi.rs), whose `new(id, name, lower, upper)` models a semi-continuous variable with a result variable and an indicator variable; `try_from_variable`/`from_variable` can infer finite bounds. Neither Rust type is the Kotlin marker-only object.

```rust
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticSemiFunction;

let input = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
let semi = QuadraticSemiFunction::new(17, "qsemi", input);
assert!(semi.result_variable().name().contains("qsemi_max"));
```

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.core.model.mechanism.QuadraticMetaModel
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.SemiFunction

val model = QuadraticMetaModel<Flt64>(
    name = "quadratic-model-with-semi-marker",
    converter = IntoValue.Identity
)
val semi = SemiFunction(
    lb = Flt64.one,
    ub = Flt64(4.0),
    converter = IntoValue.Identity,
    name = "semi"
)
check(semi.helperVariables.isEmpty())
check(semi.evaluate(emptyMap()) == null)
// Keep `semi` as metadata; it has no quadratic expression or model constraints.
model.close()
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticSemiFunction;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
let input = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
let semi = QuadraticSemiFunction::new(18, "qsemi", input);
let mut tokens = VecTokenList::<f64>::new();
let tx = Token::from_generic(x, 0);
tx.set_result(-1.0);
tokens.add_token(tx);
assert_eq!(semi.calculate_value(&tokens, false), Some(0.0));
```

:::

- Marker example: [`SemiTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/quadratic_function/SemiTest.kt)

- Rust quadratic implementation and tests: [`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)

## References

- Marker implementation: [`Semi.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Semi.kt)
- Quadratic fallback dispatch: [`MechanismModel.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/model/mechanism/MechanismModel.kt#L1350-L1355)
- Marker test: [`SemiTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/quadratic_function/SemiTest.kt)
