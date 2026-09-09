# Quadratic Product

`ProductFunction` represents the product of two linear polynomials as a quadratic intermediate expression.

> [!WARNING]
> The normal intermediate expression is $left\cdot right$ and has no public result variable. Calling its `registerConstraints` method explicitly adds the equation $left\cdot right=0$; that method is therefore an explicit zero-product constraint, not a generic “create y = product” operation.

## Contract

- Inputs: `left: LinearPolynomial<V>` and `right: LinearPolynomial<V>`.
- Output expression: the expanded `QuadraticPolynomial<V>` $left\cdot right$.
- Direct intermediate evaluation multiplies the two evaluated linear expressions; missing symbols return `null` through the token-table evaluation path.
- Generic values require `V : RealNumber<V>, V : Ring<V>, V : NumberField<V>` and an `IntoValue<V>` converter.
- The symbol is quadratic even when one of the input expressions happens to make some terms linear.

## Definition and mathematical model

For

$$
left=c_l+\sum_i a_i x_i,\qquad right=c_r+\sum_j b_j z_j,
$$

the expanded polynomial is

$$
left\cdot right
=c_lc_r+c_r\sum_i a_i x_i+c_l\sum_j b_j z_j+\sum_{i,j}a_i b_j x_i z_j.
$$

The intermediate's polynomial is this expansion. No auxiliary $y$ is needed merely to represent the expression.

## Implementation, helper variables, and constraints

`ProductFunction` expands the two linear inputs into quadratic monomials and evaluates by multiplying the inputs. It registers no auxiliary tokens. Its explicit `registerConstraints` implementation constructs one quadratic equality with the expanded polynomial on the left and zero on the right, so callers should invoke it only when that zero-product equation is intended.

## Current API

### Kotlin

Source: [`Product.kt` (`ProductFunction`)](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Product.kt#L37-L378)

```kotlin
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ProductFunction
import fuookami.ospf.kotlin.core.token.AutoTokenTable
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Quadratic
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val y = RealVar("y")
val left = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, x)), Flt64.two
)
val right = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, y)), -Flt64.one
)
val product = ProductFunction(
    left = left,
    right = right,
    converter = IntoValue.Identity,
    name = "product"
)
val tokens = AutoTokenTable<Flt64>(Quadratic, false)
tokens.add(listOf(x, y))
val value = product.prepare(
    mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0)),
    tokens,
    IntoValue.Identity
)
check(value == Flt64(16.0))
tokens.close()
```

### Rust

Rust exposes the same expression-level product as [`ProductFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/product.rs). Its constructor is:

```rust
ProductFunction::new(id: u64, name: &str, left: Linear<V>, right: Linear<V>) -> ProductFunction<V>
```

`left_polynomial`, `right_polynomial`, `prepare`, `FunctionSymbol::calculate_value`, and `QuadraticIntermediateSymbol::to_quadratic_polynomial` are the relevant public operations. The Rust implementation registers no helper tokens and returns no mechanism constraints; unlike the Kotlin implementation, it has no public `registerConstraints` operation that emits a zero-product equality.

```rust
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::ProductFunction;
use ospf_rust_core::symbol::QuadraticIntermediateSymbol;

let left = Linear::new(vec![LinearMonomial::new(1.0, 0)], 2.0);
let right = Linear::new(vec![LinearMonomial::new(1.0, 1)], -1.0);
let product = ProductFunction::new(7, "product", left, right);
let expanded = product.to_quadratic_polynomial();
assert_eq!(*expanded.constant(), -2.0);
```

The generic bounds are the Rust arithmetic traits used by the implementation (`Clone + Debug + Send + Sync + 'static` plus `Add`, `Mul`, and `Zero` where evaluation is used). `V = f64` is the default and is the smallest example choice.

## Evaluate versus solver

The intermediate evaluation APIs (`prepare`, token-table `evaluate`, and result-list `evaluate`) calculate the product directly. Quadratic mechanism registration consumes the expanded polynomial as a quadratic expression. If `registerConstraints` is called directly, the solver receives the zero-product equality described above; it does not create a free product-result variable.

## Boundaries, tolerance, and Undefined

Both linear inputs must be evaluable; missing token values produce `null`. The arithmetic is not a tolerance-based classifier and has no `TruthValue.Undefined` state. Large coefficients or products still must be representable by the chosen generic number type and solver.

## Examples and tests

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ProductFunction
import fuookami.ospf.kotlin.core.token.AutoTokenTable
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Quadratic
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val y = RealVar("y")
val left = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, x)), Flt64.two
)
val right = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, y)), -Flt64.one
)
val product = ProductFunction(
    left = left,
    right = right,
    converter = IntoValue.Identity,
    name = "product"
)
val tokens = AutoTokenTable<Flt64>(Quadratic, false)
tokens.add(listOf(x, y))
val value = product.prepare(
    mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0)),
    tokens,
    IntoValue.Identity
)
check(value == Flt64(16.0))
tokens.close()
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::ProductFunction;
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
let product = ProductFunction::new(
    8,
    "product",
    Linear::new(vec![LinearMonomial::new(1.0, 0)], 2.0),
    Linear::new(vec![LinearMonomial::new(1.0, 1)], -1.0),
);
assert_eq!(product.calculate_value(&tokens, false), Some(16.0));
```

:::

- Core evaluation: [`ProductFunctionGenericEvaluationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/ProductFunctionGenericEvaluationTest.kt)
- Core expansion/registration: [`ProductFunctionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/ProductFunctionTest.kt)
- Complete example: [`QuadraticProductEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/quadratic_function/QuadraticProductEvaluateTest.kt)

- Rust implementation and unit tests: [`product.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/product.rs)

## Related pages

- [Quadratic Linear](./quadratic-linear)
- [Quadratic Minimum](./quadratic-min)
