# 最大值

`MaxFunction` 表示一个或多个线性多项式的最大值：

$$
y=\max(p_1,p_2,\ldots,p_n).
$$

## 契约

- 输入：非空的 `List<LinearPolynomial<V>>`（`n >= 1`）。
- 输出：`resultVar`，其类型为 `URealVar`，并通过 `resultPolynomial` 暴露。
- `evaluate` 求值每个输入并返回最大值；缺少符号值时返回 `null`。
- `V` 必须实现 `RealNumber<V>` 与 `NumberField<V>`，并传入匹配的 `IntoValue<V>` 转换器。

## 数学定义

精确选择模型使用二进制 `selectorVars` $s_i$：

$$
y\ge p_i\quad(i=1,\ldots,n),
$$

$$
y-p_i+M_i s_i\le M_i,
\qquad \sum_{i=1}^{n}s_i=1.
$$

当某个候选对应的选择变量为 1 时，该候选被强制等于结果；其余不等式则强制结果不小于每个候选。

## 适用域与边界

当前结果变量为 `URealVar`。因此求解器模型不能表示负的最大值，即使 `evaluate` 可以返回负值。应确保可行域内至少有一个候选已知为非负，或者在需要负结果时采用其他建模方式。未传入 `bigM` 时，实现会尽可能使用每个候选的有限范围；否则当前 Big-M 回退值为 $10^6$。显式值必须足以覆盖所有候选差距。

## 当前 API

### Kotlin

源码：[`Max.kt`（`MaxFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Max.kt#L43-L142)

```kotlin
MaxFunction(
    polynomials: List<LinearPolynomial<V>>,
    bigM: V? = null,
    converter: IntoValue<V>,
    name: String = "max",
    displayName: String? = null
)
```

伴生工厂还提供 `fromSymbols`，用于 `LinearIntermediateSymbol<V>` 列表。当候选已经是 `LinearPolynomial<V>` 时，普通构造器最直观。

### Rust

Rust 对平展后的 `Linear<V>` 表达式暴露 [`MaxFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/max.rs)：

```rust
MaxFunction::new(
    id: u64,
    name: &str,
    polynomials: Vec<Linear<V>>,
    exact: bool,
) -> MaxFunction<V>
```

`exact = true` 为每个候选创建二值选择器，并注册恰好一个选择器的模型。`exact = false` 时，Rust 只注册 `result >= p_i` 下界；此时需要目标函数或其他上界才能让结果等于最大值。`result_variable()`、`polynomials()` 和 `exact()` 暴露内部状态。与 Kotlin 的 `URealVar` 结果不同，Rust 结果是连续变量；若模型有边界要求，调用方必须提供合适的变量范围。Rust 的 [`MinMaxFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/min_max.rs) 和 [`MaxMinFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/min_max.rs) 是对应的包装符号，但没有 `exact` 参数。

## 辅助变量与注册模型

`helperVariables` 包含 `resultVar`，以及每个候选对应的一个二进制 `selectorVar`。注册会添加 `result >= p_i`、每个候选的一条 Big-M 上界/等值门控约束，以及选择变量和为 1 的等式。可能时 Big-M 会按候选有限界分别推导。

## `evaluate` 与 solver 的差异

`evaluate` 直接折叠所有候选值，不产生 Big-M 或变量域副作用。solver 注册选择模型并施加非负结果域；Big-M 过小或真实最大值为负时，solver 可能不可行，而直接求值仍能成功。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.MaxFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.eq
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

val x = RealVar("x")
val y = RealVar("y")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val yPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, y)), Flt64.zero)
val max = MaxFunction(
    polynomials = listOf(xPoly, yPoly),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "max"
)
val value = max.evaluate(mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0)))
check(value != null && (value eq Flt64(5.0)))
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::MaxFunction;

let first = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let second = Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0);
let max = MaxFunction::new(1, "max", vec![first, second], true);
assert!(max.exact());
let _result = max.result_variable();
```

:::

完整示例：[`MaxTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/MaxTest.kt)

Core 验证：[`MaxAndMaskingFunctionGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/MaxAndMaskingFunctionGenericEvaluateTest.kt)

Rust 源码与 parity 覆盖：[`max.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/max.rs) 和 [`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs)。

## MinMaxFunction 与 MaxMinFunction

$$
MinMax(p_1,\ldots,p_n)=\max_i p_i,\qquad
MaxMin(p_1,\ldots,p_n)=\min_i p_i.
$$

虽然名称容易引起误解，`MinMaxFunction` 实际通过委托给内部 `MaxFunction` 来计算最大值，并转发求值、辅助变量和约束注册。`MaxMinFunction` 通过委托给内部 `MinFunction` 来计算最小值。名称描述的是优化语境下的解释，而不是另一种聚合算法。两个包装器都接收相同的 `polynomials`、可选 `bigM`、`converter`、`name` 和可选 `displayName` 参数。它们的 `fromSymbols` 工厂接收 `List<LinearIntermediateSymbol<V>>`，返回 `LinearFunctionSymbolAdapter`；该适配器仅用于衔接中间符号 API。

源码：[`MinMax.kt`（`MinMaxFunction` 与 `MaxMinFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/MinMax.kt#L40-L196)

```kotlin
val minMax = MinMaxFunction(
    polynomials = listOf(xPoly, yPoly),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "min_max"
)
val maxMin = MaxMinFunction(
    polynomials = listOf(xPoly, yPoly),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "max_min"
)
```

## 相关页面

- [`min`](./min)：对应的最小值运算。
- [`masking`](./masking)：二进制选择一个多项式或零。
