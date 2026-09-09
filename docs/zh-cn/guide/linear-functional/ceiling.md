# 向上取整

`CeilingFunction` 表示线性多项式的向上取整：

$$
y=\lceil p\rceil.
$$

## 契约

- 输入：`x: LinearPolynomial<V>`。
- 输出：`IntVar`（`resultVar`），通过 `resultPolynomial` 暴露。
- 输入无法求值时，`evaluate` 返回 `null`；否则返回 `ceil(p)`。
- 没有除数 `d`：此 API 是 `ceil(p)`，不是 `ceil(p / d)`。
- `bigM` 作为兼容参数保留但未使用；当前编码不使用 Big-M。

## 数学定义

对于有限实数输入，

$$
\lceil p\rceil=k\quad\Longleftrightarrow\quad k-1<p\le k.
$$

solver 使用 `epsilon = NONZERO_TOLERANCE` 将严格下界表示为

$$
p\le k,\qquad p\ge k-1+\varepsilon,
$$

并注册 `resultVar = k`。

## 适用域与边界

数学函数接受任意有限实数，包括负值。solver 的严格不等式通过容差实现，因此整数下方 epsilon 范围内的值可能与数学上的精确 `ceil` 不同；远离该范围的普通值不受影响。辅助变量 `kVar` 与 `resultVar` 都是整数变量。

## 当前 API

### Kotlin

源码：[`Ceiling.kt`（构造、求值与约束）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Ceiling.kt#L39-L119)

```kotlin
CeilingFunction(
    x: LinearPolynomial<V>,
    converter: IntoValue<V>,
    bigM: V? = null,
    name: String,
    displayName: String? = null
)
```

### Rust

源码：[`ceiling.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/ceiling.rs)

Rust 接受平展后的 `Linear<V>`，并提供 `CeilingFunction::new(id, name, input)`、`CeilingFunction::named(name, input)` 与 `CeilingFunction::auto(input)`。`input_polynomial()`、`result_variable()` 和 `integer_variable()` 暴露输入及辅助变量。结果变量是与辅助整数变量关联的 `ContinuousVariableItem`；Rust 没有调用方可传入的 `big_m` 或 tolerance 参数，机理层使用固定的 `ROUNDING_EPSILON = 1e-8` 边界。

```rust
CeilingFunction::new(id: u64, name: &str, input: Linear<V>) -> Self
CeilingFunction::named(name: impl AsRef<str>, input: Linear<V>) -> Self
CeilingFunction::auto(input: Linear<V>) -> Self
```

## 辅助变量与注册模型

`helperVariables` 注册 `kVar` 与 `resultVar`。注册会添加两条带 epsilon 的边界和等式 `resultVar = kVar`。当前实现没有 `d` 参数，也没有 Big-M 约束。

## `evaluate` 与 solver 的差异

`evaluate` 通过 `IntoValue` 转换输入并调用数值类型的 `ceil`。solver 注册整数变量和带 epsilon 的不等式。因此差异仅来自表示严格不等式的有限数值容差；`bigM` 不影响此函数。

## 当前最小示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.CeilingFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.eq
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

val x = RealVar("x")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val ceil = CeilingFunction(
    x = xPoly,
    converter = IntoValue.Identity,
    name = "ceil"
)
val value = ceil.evaluate(mapOf<Symbol, Flt64>(x to Flt64(1.2)))
check(value != null && (value eq Flt64.two))
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::CeilingFunction;
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::token::VecTokenList;

let function = CeilingFunction::named("ceil", Linear::new(vec![], 1.2));
let value = <CeilingFunction as FunctionSymbol>::calculate_value(
    &function,
    &VecTokenList::<f64>::new(),
    false,
);
assert_eq!(value, Some(2.0));
```

:::

完整示例：[`CeilingTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/CeilingTest.kt)

Core 验证：[`FunctionSymbolDiscreteGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolDiscreteGenericEvaluateTest.kt)

Rust 实现与单元测试：[`ceiling.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/ceiling.rs)

## 相关页面

- [`floor`](./floor)：向下取整对应运算。
- [`rounding`](./rounding)：最近整数编码，半整数规则不同。
- [`mod`](./mod)：使用缩放值的 floor，但要求正除数。
