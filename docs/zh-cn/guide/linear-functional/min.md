# 最小值

`MinFunction` 表示一个或多个线性多项式的最小值：

$$
y=\min(p_1,p_2,\ldots,p_n).
$$

## 契约

- 输入：非空的 `List<LinearPolynomial<V>>`（`n >= 1`）。
- 输出：`resultVar`，其类型为 `URealVar`，并通过 `resultPolynomial` 暴露。
- `evaluate` 求值每个输入并返回最小值；缺少符号值时返回 `null`。
- `V` 必须实现 `RealNumber<V>` 与 `NumberField<V>`，并传入匹配的 `IntoValue<V>` 转换器。

## 数学定义

实现使用二进制 `selectorVars` $s_i$ 和对称的精确选择模型：

$$
y\le p_i\quad(i=1,\ldots,n),
$$

$$
y-p_i-M_i s_i\ge -M_i,
\qquad \sum_{i=1}^{n}s_i=1.
$$

当某个候选对应的选择变量为 0 时，该候选被强制等于结果；其余不等式则强制结果不大于每个候选。

## 适用域与边界

solver 结果是 `URealVar`，所以无法表示负的最小值；`evaluate` 仍可以返回负值。只有在可行模型保证最小值非负时才应使用此函数，或者选择带符号的结果建模。与 `MaxFunction` 一样，推导 Big-M 需要候选有限界；否则当前回退值为 $10^6$，显式 `bigM` 必须覆盖所有候选差距。

## 当前 API

### Kotlin

`MinFunction` 与 `MaxFunction` 声明在同一个源码文件中（不存在独立实现文件）：[`Max.kt`（`MinFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Max.kt#L182-L281)

```kotlin
MinFunction(
    polynomials: List<LinearPolynomial<V>>,
    bigM: V? = null,
    converter: IntoValue<V>,
    name: String = "min",
    displayName: String? = null
)
```

伴生工厂还提供 `fromSymbols`，面向 `LinearIntermediateSymbol<V>` 候选。

### Rust

Rust 在 [`MinFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/max.rs) 中使用 `flatten::Linear<V>`：

```rust
MinFunction::new(id: u64, name: &str, polynomials: Vec<Linear<V>>, exact: bool) -> MinFunction<V>
```

`exact = true` 为每个候选创建一个二值选择器并注册精确选择模型；`exact = false` 只保留不等式包络。与 Kotlin 不同，Rust 构造器没有 `bigM` 或 converter 参数，结果通过 `result_variable()` 暴露。

## 求解器数学模型

令结果为 $y$、候选为 $p_i$、选择变量 $s_i\in\{0,1\}$。Kotlin 与 Rust 的 `exact = true` 实际传入

$$
y-p_i\le0\quad(1\le i\le n),
$$

$$
y-p_i-M_i s_i\ge-M_i\quad(1\le i\le n),
\qquad
\sum_{i=1}^{n}s_i=1.
$$

被选候选的 $s_i=0$。Rust 的 `exact = false` 只保留 $y-p_i\le0$；此时需要最大化或其他下界才能得到最小值。Kotlin 始终注册精确选择模型。

## `evaluate` 与 solver 的差异

`evaluate` 直接折叠候选值，不施加 `URealVar` 的变量域。solver 注册会施加该域并依赖有效的 Big-M。因此全为负候选时，直接求值可以成功，而 solver 模型可能不可行。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.MinFunction
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
val min = MinFunction(
    polynomials = listOf(xPoly, yPoly),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "min"
)
val value = min.evaluate(mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0)))
check(value != null && (value eq Flt64.two))
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::MinFunction;

let x = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let y = Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0);
let min = MinFunction::new(1, "min", vec![x, y], true);
assert!(min.exact());
let _result = min.result_variable();
```

:::

完整示例：[`MinTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/MinTest.kt)

Core 验证：[`MaxAndMaskingFunctionGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/MaxAndMaskingFunctionGenericEvaluateTest.kt)

Rust 源码与跨语言回归覆盖：[`max.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/max.rs) 和 [`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs)。

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

- [`max`](./max)：对应的最大值运算。
- [`masking`](./masking)：二进制选择一个多项式或零。
