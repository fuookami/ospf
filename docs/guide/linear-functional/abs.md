# Absolute value

`AbsFunction` represents the absolute value of a linear polynomial:

$$
y = |p|.
$$

The implementation in `ospf-kotlin-core` is the authoritative contract for this page.

## Contract

- Input: one `LinearPolynomial<V>` named `polynomial`.
- Output: a non-negative `URealVar` exposed through `resultPolynomial`.
- `evaluate` returns `null` when the input polynomial cannot be evaluated from the supplied symbol values; otherwise it returns `|p|`.
- `V` must implement both `RealNumber<V>` and `NumberField<V>`, and the same `IntoValue<V>` converter must be used for constants and evaluation.

## Mathematical definition

For an evaluated input value $p$,

$$
|p| = \begin{cases}p,&p\ge 0,\\-p,&p<0.\end{cases}
$$

The solver model decomposes the value into non-negative parts:

$$
p=p^+-p^-,\qquad y=p^++p^-,
$$

and uses a binary selector $s$ with Big-M bounds

$$
0\le p^+\le M s,\qquad 0\le p^-\le M(1-s).
$$

## Domain and boundaries

The mathematical function accepts any finite real value. The solver encoding needs a usable Big-M bound. If `bigM` is omitted, the implementation first tries to derive a finite bound from `polynomial`; when that is not possible it falls back to the library default (currently $10^6$). Choose an explicit, valid `bigM` for a tightly bounded model. The result variable is non-negative, while the input polynomial itself may be negative.

## Current API

### Kotlin

Source: [`Abs.kt` (constructor, variables, evaluation, and constraints)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Abs.kt#L41-L115)

The primary constructor/factory is:

```kotlin
AbsFunction(
    polynomial: LinearPolynomial<V>,
    converter: IntoValue<V>,
    bigM: V? = null,
    name: String,
    displayName: String? = null
)
```

The public helper/result variables are `resultVar`, `posVar`, `negVar`, and `signVar`; `helperVariables` registers all four. `resultPolynomial` is the one-term polynomial for `resultVar`.

### Rust

Source: [`abs.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/abs.rs)

The Rust implementation takes a flattened `Linear<V>` and exposes `AbsFunction::new(id, name, input)`, `AbsFunction::named(name, input)`, and `AbsFunction::auto(input)`. The result and sign helper variables are available through `result_variable()` and `side_variable()`. Big-M is inferred from registered variable bounds when possible and otherwise uses the core fallback; there is no explicit `big_m` constructor argument.

```rust
AbsFunction::new(id: u64, name: &str, input: Linear<V>) -> Self
AbsFunction::named(name: impl AsRef<str>, input: Linear<V>) -> Self
AbsFunction::auto(input: Linear<V>) -> Self
```

## Auxiliary variables and registration

`registerAuxiliaryTokens` adds the four variables. `registerConstraints` adds the equality decomposition and the two Big-M gating inequalities above. The input's own bounds are not added by `AbsFunction`; they are only used to infer the default Big-M value.

## `evaluate` versus solver

`evaluate` directly evaluates `polynomial` and applies the sign test. The solver uses the binary decomposition, so it additionally depends on `bigM` being large enough. At a valid finite input, both describe the same absolute value; an undersized Big-M can make the solver model infeasible or exclude the correct value even though `evaluate` still succeeds.

## Minimal current example

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.AbsFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.eq
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

val x = RealVar("x")
val xPoly = LinearPolynomial(
    monomials = listOf(LinearMonomial(Flt64.one, x)),
    constant = Flt64.zero
)
val abs = AbsFunction(
    polynomial = xPoly,
    converter = IntoValue.Identity,
    name = "abs"
)
val value = abs.evaluate(mapOf<Symbol, Flt64>(x to Flt64(-3.0)))
check(value != null && (value eq Flt64(3.0)))
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::AbsFunction;
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

let x = ContinuousVariableItem::create(VariableId::standalone(1), "x");
let token = Token::from_generic(x, 0);
token.set_result(-3.0);
let mut tokens = VecTokenList::new();
tokens.add_token(token);
let input = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let abs = AbsFunction::named("abs", input);
let value = <AbsFunction as FunctionSymbol>::calculate_value(&abs, &tokens, false);
assert_eq!(value, Some(3.0));
```

:::

Rust evaluation/registration coverage: [`p0_evaluation_tests.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/p0_evaluation_tests.rs)

Complete example: [`AbsTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/AbsTest.kt)

Core validation: [`FunctionSymbolGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolGenericRegistrationTest.kt)

## Related pages

- [`masking`](./masking): gate a polynomial by a binary variable.
- [`max`](./max) and [`min`](./min): select among several linear polynomials.
- [`ceiling`](./ceiling), [`floor`](./floor), and [`rounding`](./rounding): discrete-valued transformations.
