# 二次乘积

`ProductFunction` 将两个线性多项式的乘积表示为二次中间表达式。

> [!WARNING]
> 普通中间表达式是 $left\cdot right$，没有公开结果变量。显式调用 `registerConstraints` 会加入 $left\cdot right=0$ 等式；因此该方法表示显式的零乘积约束，不是通用的“创建 y = product”操作。

## 契约

- 输入：`left: LinearPolynomial<V>` 和 `right: LinearPolynomial<V>`。
- 输出表达式：展开后的 `QuadraticPolynomial<V>`，即 $left\cdot right$。
- 中间符号直接求值会把两个线性表达式求值后相乘；token 表中缺少符号时沿求值路径返回 `null`。
- 泛型值要求 `V : RealNumber<V>, V : Ring<V>, V : NumberField<V>`，并配合 `IntoValue<V>` 转换器。
- 即使输入表达式使部分项退化为线性，该符号仍属于二次中间符号。

## 定义与数学模型

对于

$$
left=c_l+\sum_i a_i x_i,\qquad right=c_r+\sum_j b_j z_j,
$$

展开式为

$$
left\cdot right
=c_lc_r+c_r\sum_i a_i x_i+c_l\sum_j b_j z_j+\sum_{i,j}a_i b_j x_i z_j.
$$

中间符号的多项式就是该展开式；仅表示表达式不需要辅助 $y$。

## 实现、辅助变量与约束

`ProductFunction` 将两个线性输入展开为二次单项式，并通过相乘求值。不注册辅助 token。其显式 `registerConstraints` 实现把展开多项式放在左侧、零放在右侧构造一个二次等式，因此只有在确实需要零乘积等式时才应调用它。

## 当前 API

### Kotlin

源码：[`Product.kt`（`ProductFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Product.kt#L37-L378)

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

Rust 通过 [`ProductFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/product.rs) 提供相同的表达式级乘积。构造器为：

```rust
ProductFunction::new(id: u64, name: &str, left: Linear<V>, right: Linear<V>) -> ProductFunction<V>
```

相关公开操作包括 `left_polynomial`、`right_polynomial`、`prepare`、`FunctionSymbol::calculate_value` 和 `QuadraticIntermediateSymbol::to_quadratic_polynomial`。Rust 实现不注册辅助 token，也不返回机理约束；与 Kotlin 实现不同，它没有发出零乘积等式的公开 `registerConstraints` 操作。

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

泛型边界是实现使用的 Rust 算术 trait（求值时还需要 `Clone + Debug + Send + Sync + 'static`、`Add`、`Mul` 和 `Zero`）。默认类型为 `f64`，也是最小示例的选择。

## evaluate 与 solver 的差异

中间求值 API（`prepare`、带 token 表的 `evaluate` 和结果列表 `evaluate`）直接计算乘积。二次机制注册把展开后的多项式作为二次表达式使用。若直接调用 `registerConstraints`，solver 会收到上面所述的零乘积等式，不会创建自由的乘积结果变量。

## 边界、tolerance 与 Undefined

两个线性输入都必须可求值；token 值缺失时返回 `null`。该算术没有基于 tolerance 的分类，也没有 `TruthValue.Undefined` 状态。大系数或大乘积仍必须能由所选泛型数值和 solver 表示。

## 示例与测试

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

- Core 求值：[`ProductFunctionGenericEvaluationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/ProductFunctionGenericEvaluationTest.kt)
- Core 展开/注册：[`ProductFunctionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/ProductFunctionTest.kt)
- 完整示例：[`QuadraticProductEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/quadratic_function/QuadraticProductEvaluateTest.kt)

- Rust 实现与单元测试：[`product.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/product.rs)

## 相关页面

- [二次线性](./quadratic-linear)
- [二次最小值](./quadratic-min)
