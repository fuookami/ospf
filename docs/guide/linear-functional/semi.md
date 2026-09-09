# Semi-Continuous Marker

## Current API

### Kotlin

`SemiFunction<V>` is a marker carrying the active interval of a semi-continuous variable:

$$
y = 0 \quad\text{or}\quad lb \le y \le ub.
$$

It is **not** the positive-part function `max(0,x)` and has no input expression. The implementation is [`Semi.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Semi.kt#L37-L107). Its constructors are:

```kotlin
SemiFunction(
    lb: V? = null,
    ub: V? = null,
    converter: IntoValue<V>,
    name: String = "semi",
    displayName: String? = null
)

SemiFunction.from(
    variable: AbstractVariableItem<*, *>,
    lb: V? = null,
    ub: V? = null,
    converter: IntoValue<V>,
    name: String = "semi",
    displayName: String? = null
)
```

The defaults are `lb = 0` and `ub = 1e6` (`Semi.kt:37-50`); `lb <= ub` is required. `from` infers missing bounds from `variable.range.valueRange` (`Semi.kt:89-106`).

### Rust

Rust provides an executable [`SemiFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/semi.rs), not a no-op marker:

```rust
SemiFunction::new(
    id: u64,
    name: &str,
    lower: V,
    upper: V,
) -> SemiFunction<V>

SemiFunction::try_from_variable(
    id: u64,
    name: &str,
    variable: &ContinuousVariableItem,
    lower: Option<V>,
    upper: Option<V>,
) -> Result<SemiFunction<V>>
```

The symbol creates a continuous `result_variable()` and a binary `indicator_variable()`, then registers `result <= upper * indicator` and `result >= lower * indicator`. `try_from_variable` (also aliased as `from_variable`) can infer missing finite bounds from a `ContinuousVariableItem`. This is a semantic difference from Kotlin, whose `SemiFunction` has no helpers and does not register domain constraints.

## Runtime and registration semantics

The marker creates no helper variables (`helperVariables` is empty), `evaluate` always returns `null`, and both `registerAuxiliaryTokens` and `registerConstraints` return success without adding anything (`Semi.kt:53-65`). It therefore does not compute `max(0,x)`, does not attach to a linear expression, and does not itself enforce a semi-continuous domain. A solver/backend integration that understands this marker must consume it separately; merely constructing or retaining `SemiFunction` does not change a model.

## References

- Implementation: [`Semi.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Semi.kt)
- Complete example: [`SemiTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/SemiTest.kt)

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.SemiFunction

val semi = SemiFunction(
    lb = Flt64.two,
    ub = Flt64.five,
    converter = IntoValue.Identity,
    name = "semi"
)
check(semi.lb == Flt64.two)
check(semi.ub == Flt64.five)
check(semi.helperVariables.isEmpty())
check(semi.evaluate(emptyMap()) == null)
```

```rust [Rust]
use ospf_rust_core::symbol::function::SemiFunction;

let semi = SemiFunction::new(1, "semi", 2.0_f64, 5.0_f64);
assert_eq!(semi.lower_bound(), &2.0);
assert_eq!(semi.upper_bound(), &5.0);
let _result = semi.result_variable();
let _indicator = semi.indicator_variable();
```

:::

The current smoke test checks the bounds, empty helper list, and unresolved evaluation ([`SemiTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/SemiTest.kt#L16-L24)):

To model `max(0,x)`, use an explicit positive-part formulation; do not pass an expression to `SemiFunction`, because no such parameter exists.

Rust source: [`semi.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/semi.rs).
