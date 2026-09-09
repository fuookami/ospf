# 满足数量

`SatisfiedAmountFunction` 统计满足的线性不等式数量，也可以判断数量是否达到阈值。

## 契约

- 输入：`List<LinearInequality<V>>`，并由调用方提供 `epsilon`。
- `amount = null` 时，输出 `result` 是原始数量 $0,\ldots,n$。
- 设置 `amount` 时，输出为二值：数量至少为 `amount` 时为 one。
- 直接求值识别 LE/LT/GE/GT/EQ/NE；当前 solver 注册只支持 LE/GE/EQ，遇到 LT/GT/NE 会失败。
- 每个不等式使用一个满足二值标志；EQ 另外使用一个 side 标志。
- Big-M 尽可能从每个不等式的差值多项式推导。

## 定义与数学模型

对不等式 $i$，令

$$
u_i=\mathbf{1}[\text{inequality }i\text{ is satisfied}],\qquad
c=\sum_{i=0}^{n-1}u_i.
$$

结果为

$$
y=\begin{cases}
c,&\text{if amount is null},\\
\mathbf{1}[c\ge amount],&\text{otherwise}.
\end{cases}
$$

阈值模式是“至少达到”，不是精确数量等式。

## 实现、辅助变量与约束

实现创建在 `name` 后追加 `_u_` 和索引的标志；对 EQ 输入创建在 `name` 后追加 `_eq_side_` 和索引的标志。注册为 LE/GE 标志添加两条 Big-M 约束，EQ 委托零指标约束；请求阈值时添加 $\sum u_i\ge amount$。原始计数形式不引入结果变量，`result` 就是标志之和。

## 当前 API

### Kotlin

源码：[`SatisfiedAmount.kt`（`SatisfiedAmountFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/SatisfiedAmount.kt#L42-L224)

伴随工厂使用相同的公开参数。使用 `amount = null` 可保留计数，而不是阈值指标。

```kotlin
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.SatisfiedAmountFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.inequality.LinearInequality
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val one = LinearPolynomial<Flt64>(emptyList(), Flt64.one)
val inequality = LinearInequality(xPoly, one, Comparison.LE, "x_le_1")
val satisfied = SatisfiedAmountFunction(
    inequalities = listOf(inequality),
    amount = UInt64.one,
    epsilon = Flt64(1e-6),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "satisfied"
)
val value = satisfied.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero))
check(value == Flt64.one)
```

### Rust

Rust 当前没有一个直接接收 `Vec<LinearInequality<V>>`、epsilon 和 Kotlin 风格阈值结果的对应 API。[`SatisfiedAmountFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/satisfied_amount.rs) 统计已经创建好的二值指标变量：

```rust
SatisfiedAmountFunction::new(
    id: u64,
    name: &str,
    indicators: Vec<BinaryVariableItem>,
) -> Self

pub fn with_amount_range(
    mut self,
    lower: Option<usize>,
    upper: Option<usize>,
) -> Self
```

便捷构造器包括 `any`、`all`、`at_least`、`not_all` 和 `numerable`。`result_variable()` 是等于指标之和的连续计数变量；`with_amount_range` 添加计数硬边界，不会创建 Kotlin 那样独立的二值阈值结果。应先用 Rust `InequalityFunction` 等函数为每个不等式创建指标，再将两个符号都注册到模型中。

## evaluate 与 solver 的差异

直接求值对每种比较应用 epsilon，可以统计六种比较。solver 注册只使用支持的编码；遇到 LT、GT 或 NE 时在写入模型前返回失败 Result。非正或不足的 Big-M、缺失输入，或无法推导差值范围，都可能使注册失败，即使直接计数有定义。

## 边界、tolerance 与 Undefined

直接 `evaluate` 缺少输入值时返回 `null`。当前实现把 `amount` 转为 Int 大小的计数；调用方应将其保持在输入列表大小和宿主 Int 范围内。该函数没有三值 Undefined 结果；不支持的关系类型会明确导致注册失败。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.SatisfiedAmountFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.inequality.LinearInequality
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val one = LinearPolynomial<Flt64>(emptyList(), Flt64.one)
val inequality = LinearInequality(xPoly, one, Comparison.LE, "x_le_1")
val satisfied = SatisfiedAmountFunction(
    inequalities = listOf(inequality),
    amount = UInt64.one,
    epsilon = Flt64(1e-6),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "satisfied"
)
val value = satisfied.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero))
check(value == Flt64.one)
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{
    InequalityFunction, SatisfiedAmountFunction,
};

let x = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let le = InequalityFunction::less_equal(1, "x_le_1", x, 1.0_f64, 10.0_f64);
let satisfied = SatisfiedAmountFunction::<f64>::new(
    2,
    "satisfied",
    vec![le.result_variable().clone()],
)
.with_amount_range(Some(1), None);
let _count = satisfied.result_variable();
```

:::

- Core 测试：[`SatisfiedAmountFunctionsGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/SatisfiedAmountFunctionsGenericEvaluateTest.kt)
- 注册测试：[`FunctionSymbolSameAsGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolSameAsGenericRegistrationTest.kt)
- 示例目录（当前没有专门的满足数量文件）：[linear_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function)

将 `le` 和 `satisfied` 都注册到模型后，才会同时施加指标与计数约束。Rust 源码：[`satisfied_amount.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/satisfied_amount.rs) 和 [`inequality.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/inequality.rs)。

## 相关页面

- [不等式指示函数](./inequality)
- [满足数量不等式](./satisfied-amount-inequality)
- [同状态](./same-as)
