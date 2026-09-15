# 同状态

`SameAsFunction` 在所有输入不等式的满足状态相同（要么全部满足，要么全部不满足）时返回 one。

## 契约

- 输入：非空的 `List<LinearInequality<V>>`。
- `constraint = true`（默认）会在 solver 模型中强制所有满足标志相等。
- `constraint = false` 只度量标志是否相等，不强制输入不等式一致。
- 输出：二值 `resultPolynomial`；直接求值在状态一致时为 one，否则为 zero。
- `epsilon` 是比较容差，`m` 是每个输入指标可选的 Big-M。

## 定义与数学模型

令 $u_i\in\{0,1\}$ 表示不等式 $i$ 是否满足，则

$$
y=\mathbf{1}[u_0=u_1=\cdots=u_{n-1}].
$$

当 `constraint` 为 true 时，模型添加

$$
u_0-u_i=0\quad(i=1,\ldots,n-1),\qquad y=u_0.
$$

当 `constraint` 为 false 且 $n>1$ 时，创建差异标志 $d_i=|u_i-u_0|$，并添加

$$
y+\sum_{i=1}^{n-1}d_i=1.
$$

单个输入时，度量模式固定 $y=1$。

## 求解器数学模型

Kotlin 先为每个不等式注册满足指标 $u_i\in\{0,1\}$，关系约束与[不等式指标](./inequality)相同。在约束模式下，随后实际传入

$$
u_0-u_i=0\quad(1\le i<n),\qquad y-u_0=0.
$$

在度量模式下，使用二值绝对差约束创建 $d_i=|u_i-u_0|$，并传入

$$
y+\sum_{i=1}^{n-1}d_i=1.
$$

Rust 较窄的二元函数则对 $p-q$ 应用共享零带 Big-M 编码并公开相等标志；它没有 Kotlin 的列表输入或硬约束模式。

## 当前 API

### Kotlin

源码：[`SameAs.kt`（`SameAsFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/SameAs.kt#L46-L305)

```kotlin
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.SameAsFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.inequality.LinearInequality
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val y = RealVar("y")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val yPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, y)), Flt64.zero)
val zero = LinearPolynomial<Flt64>(emptyList(), Flt64.zero)
val inequalities = listOf(
    LinearInequality(xPoly, zero, Comparison.LE, "x_le_0"),
    LinearInequality(yPoly, zero, Comparison.LE, "y_le_0")
)
val same = SameAsFunction(
    inequalities = inequalities,
    constraint = false,
    epsilon = Flt64(1e-6),
    m = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "same"
)
val value = same.evaluate(
    mapOf<Symbol, Flt64>(x to Flt64.zero, y to Flt64.one)
)
check(value == Flt64.zero)
```

### Rust

Rust 提供 [`SameAsFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/same_as.rs)，但形状比 Kotlin 的不等式列表 API 更窄：

```rust
SameAsFunction::new(
    id: u64,
    name: &str,
    first: Linear<V>,
    second: Linear<V>,
    tolerance: V,
) -> SameAsFunction<V>
```

它在 `tolerance` 内比较两个线性表达式，创建二值 `result_variable()`，没有 Rust `constraint` 开关，也不接收 `LinearInequality` 列表。因此它是最接近的“两表达式相等” API，并不是 Kotlin“所有不等式状态一致”函数的一一对应实现。

## evaluate 与 solver 的差异

直接求值计算每个不等式状态并始终返回度量值 $y$，不论 `constraint` 参数如何。solver 注册不同：`constraint = true` 将状态相等作为硬约束；`constraint = false` 将度量结果连接到状态是否相等。等式或严格边界附近，直接比较使用实现的 epsilon 规则，而指标约束使用 Big-M/tolerance。

## 边界、tolerance 与 Undefined

不等式列表必须非空（`init` 会强制这一点）。缺少符号时直接求值返回 `null`。调用方必须提供足够的有限 Big-M，或允许从每个差值多项式推导。该函数不会返回 `TruthValue.Undefined`；无效注册通过失败 Result 暴露。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.SameAsFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.inequality.LinearInequality
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val y = RealVar("y")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val yPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, y)), Flt64.zero)
val zero = LinearPolynomial<Flt64>(emptyList(), Flt64.zero)
val inequalities = listOf(
    LinearInequality(xPoly, zero, Comparison.LE, "x_le_0"),
    LinearInequality(yPoly, zero, Comparison.LE, "y_le_0")
)
val same = SameAsFunction(
    inequalities = inequalities,
    constraint = false,
    epsilon = Flt64(1e-6),
    m = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "same"
)
val value = same.evaluate(
    mapOf<Symbol, Flt64>(x to Flt64.zero, y to Flt64.one)
)
check(value == Flt64.zero)
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::SameAsFunction;

let first = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let second = Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0);
let same = SameAsFunction::new(1, "same", first, second, 1.0e-6_f64);
let _result = same.result_variable();
```

:::

- Core 测试：[`FunctionSymbolSameAsGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolSameAsGenericRegistrationTest.kt)
- 示例目录（当前没有专门的同状态文件）：[linear_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function)

Rust 源码：[`same_as.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/same_as.rs)。

## 相关页面

- [不等式指示函数](./inequality)
- [满足数量](./satisfied-amount)
- [逻辑异或](./xor)
