# 绝对值

`AbsFunction` 表示线性多项式的绝对值：

$$
y = |p|.
$$

本页以 `ospf-kotlin-core` 中的当前实现作为唯一契约。

## 契约

- 输入：一个 `LinearPolynomial<V>`，名称为 `polynomial`。
- 输出：非负 `URealVar`，通过 `resultPolynomial` 暴露。
- 当输入多项式无法由给定符号值求出时，`evaluate` 返回 `null`；否则返回 `|p|`。
- `V` 必须同时实现 `RealNumber<V>` 与 `NumberField<V>`，常量和求值必须使用同一个 `IntoValue<V>` 转换器。

## 数学定义

对于求值后的输入值 $p$，

$$
|p| = \begin{cases}p,&p\ge 0,\\-p,&p<0.\end{cases}
$$

求解器模型将其分解为非负部分：

$$
p=p^+-p^-,\qquad y=p^++p^-,
$$

并使用二进制选择变量 $s$ 与 Big-M 界：

$$
0\le p^+\le M s,\qquad 0\le p^-\le M(1-s).
$$

## 适用域与边界

数学函数接受任意有限实数。求解器编码需要可用的 Big-M 界。省略 `bigM` 时，实现会先尝试从 `polynomial` 推导有限界；无法推导时退回库默认值（当前为 $10^6$）。对于有明确范围的模型，应传入有效且足够紧的 `bigM`。结果变量非负，但输入多项式可以为负。

## 当前 API

### Kotlin

源码：[`Abs.kt`（构造、变量、求值与约束）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Abs.kt#L41-L115)

主要构造器/工厂为：

```kotlin
AbsFunction(
    polynomial: LinearPolynomial<V>,
    converter: IntoValue<V>,
    bigM: V? = null,
    name: String,
    displayName: String? = null
)
```

公开的辅助/结果变量为 `resultVar`、`posVar`、`negVar` 和 `signVar`；`helperVariables` 会注册这四个变量。`resultPolynomial` 是仅含 `resultVar` 的单项线性多项式。

### Rust

源码：[`abs.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/abs.rs)

Rust 实现接收平展后的 `Linear<V>`，提供 `AbsFunction::new(id, name, input)`、`AbsFunction::named(name, input)` 和 `AbsFunction::auto(input)`。可通过 `result_variable()` 与 `side_variable()` 获取结果变量和符号辅助变量。注册变量有范围时会推导 Big-M，否则使用核心层回退值；Rust 构造器没有显式 `big_m` 参数。

```rust
AbsFunction::new(id: u64, name: &str, input: Linear<V>) -> Self
AbsFunction::named(name: impl AsRef<str>, input: Linear<V>) -> Self
AbsFunction::auto(input: Linear<V>) -> Self
```

## 辅助变量与注册模型

`registerAuxiliaryTokens` 添加四个变量。`registerConstraints` 添加上面的分解等式与两条 Big-M 门控不等式。输入自身的范围约束不会由 `AbsFunction` 添加；范围只用于推导默认 Big-M。

## `evaluate` 与 solver 的差异

`evaluate` 直接求值 `polynomial` 并根据符号取值。求解器使用二进制分解，因此还依赖 `bigM` 足够大。在有效有限输入上两者表示同一个绝对值；但 Big-M 过小会使求解器模型不可行或排除正确值，而 `evaluate` 仍可能成功。

## 当前最小示例

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

Rust 求值/注册覆盖：[`p0_evaluation_tests.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/p0_evaluation_tests.rs)

完整示例：[`AbsTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/AbsTest.kt)

Core 验证：[`FunctionSymbolGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolGenericRegistrationTest.kt)

## 相关页面

- [`masking`](./masking)：用二进制变量门控多项式。
- [`max`](./max) 与 [`min`](./min)：在多个线性多项式中选择。
- [`ceiling`](./ceiling)、[`floor`](./floor) 与 [`rounding`](./rounding)：离散值变换。
