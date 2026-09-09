# 向下取整

`FloorFunction` 表示线性多项式的向下取整：

$$
y=\lfloor p\rfloor.
$$

## 契约

- 输入：`x: LinearPolynomial<V>`。
- 输出：`IntVar`（`resultVar`），通过 `resultPolynomial` 暴露。
- 输入无法求值时，`evaluate` 返回 `null`；否则返回 `floor(p)`。
- 没有除数 `d`：此 API 是 `floor(p)`，不是 `floor(p / d)`。
- `bigM` 作为兼容参数保留但未使用；当前编码不使用 Big-M。

## 数学定义

对于有限实数输入，

$$
\lfloor p\rfloor=k\quad\Longleftrightarrow\quad k\le p<k+1.
$$

solver 使用 `epsilon = NONZERO_TOLERANCE` 将严格上界表示为

$$
p\ge k,\qquad p\le k+1-\varepsilon,
$$

并注册 `resultVar = k`。

## 适用域与边界

数学函数接受任意有限实数，包括负值。严格上界通过容差实现，因此整数边界下方 epsilon 范围内的值可能在 solver 模型中受到影响；远离该范围的普通值不受影响。`kVar` 与 `resultVar` 都是整数变量。

## 当前 API

### Kotlin

源码：[`Floor.kt`（构造、求值与约束）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Floor.kt#L40-L120)

```kotlin
FloorFunction(
    x: LinearPolynomial<V>,
    converter: IntoValue<V>,
    bigM: V? = null,
    name: String,
    displayName: String? = null
)
```

### Rust

源码：[`floor.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/floor.rs)

Rust 接受平展后的 `Linear<V>`，并提供 `FloorFunction::new(id, name, input)`、`FloorFunction::named(name, input)` 与 `FloorFunction::auto(input)`。`input_polynomial()`、`result_variable()` 和 `integer_variable()` 暴露输入及辅助变量。结果变量是与辅助整数变量关联的 `ContinuousVariableItem`；Rust 没有调用方可传入的 `big_m` 或 tolerance 参数，机理层使用固定的 `ROUNDING_EPSILON = 1e-8` 边界。

```rust
FloorFunction::new(id: u64, name: &str, input: Linear<V>) -> Self
FloorFunction::named(name: impl AsRef<str>, input: Linear<V>) -> Self
FloorFunction::auto(input: Linear<V>) -> Self
```

## 辅助变量与注册模型

`helperVariables` 注册 `kVar` 与 `resultVar`。注册会添加 `k <= p`、带 epsilon 的 `p <= k+1-epsilon`，以及等式 `resultVar = k`。当前实现没有 `d` 参数，也没有 Big-M 约束。

## `evaluate` 与 solver 的差异

`evaluate` 通过 `IntoValue` 转换输入并调用数值类型的 `floor`。solver 注册整数变量和带 epsilon 的不等式。差异仅限于严格边界的数值处理；`bigM` 不影响此函数。

## 当前最小示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.FloorFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.eq
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

val x = RealVar("x")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val floor = FloorFunction(
    x = xPoly,
    converter = IntoValue.Identity,
    name = "floor"
)
val value = floor.evaluate(mapOf<Symbol, Flt64>(x to Flt64(1.8)))
check(value != null && (value eq Flt64.one))
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::FloorFunction;
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::token::VecTokenList;

let function = FloorFunction::named("floor", Linear::new(vec![], 1.8));
let value = <FloorFunction as FunctionSymbol>::calculate_value(
    &function,
    &VecTokenList::<f64>::new(),
    false,
);
assert_eq!(value, Some(1.0));
```

:::

完整示例：[`FloorTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/FloorTest.kt)

Core 验证：[`FunctionSymbolDiscreteGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolDiscreteGenericEvaluateTest.kt)

Rust 实现与单元测试：[`floor.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/floor.rs)

## 相关页面

- [`ceiling`](./ceiling)：向上取整对应运算。
- [`rounding`](./rounding)：最近整数编码，半整数规则不同。
- [`mod`](./mod)：使用缩放值的 floor，但要求正除数。
