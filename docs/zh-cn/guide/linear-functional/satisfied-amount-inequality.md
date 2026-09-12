# 满足数量不等式

`SatisfiedAmountInequalityFunction` 统计扁平化线性约束输入的满足数量；给出 amount 范围时，返回计数是否落在该范围内的二值指标。本页也覆盖 `AnyFunction`、`AllFunction`、`AtLeastInequalityFunction`、`NotAllFunction` 和 `NumerableFunction` 变体。

## 契约

- 输入：`List<LinearConstraintInput<V>>`，不是普通 `LinearInequality` 列表。
- 每个 `LinearConstraintInput` 携带扁平化关系、`lhsRange` 和 `rhsConstant`；solver 注册使用该范围构造标志编码。
- `amount = null` 时，输出原始满足数量。
- `amount = [l,u]` 时，计数满足 $l\le count\le u$ 返回 one。
- `epsilon` 控制直接边界判定；`from` 工厂接受 `Flt64` epsilon 并通过 `IntoValue<V>` 转换。

## 定义与数学模型

对每个输入，令其相对于零的扁平化关系满足时 $u_i=1$，否则为 zero。则

$$
c=\sum_{i=0}^{n-1}u_i.
$$

基础结果为

$$
y=\begin{cases}
c,&amount=null,\\
\mathbf{1}[l\le c\le u],&amount=[l,u].
\end{cases}
$$

便捷变体为：

| API | 内部 amount 范围 | 含义 |
| --- | --- | --- |
| `AnyFunction` | $[1,n]$ | 至少一个 |
| `AllFunction` | $[n,n]$ | 全部 |
| `AtLeastInequalityFunction(k)` | $[k,n]$ | 至少 k 个 |
| `NotAllFunction` | $n>1$ 时为 $[1,n-1]$ | 不全 |
| `NumerableFunction(amount)` | 调用方提供 | 计数落在范围内 |

## 求解器数学模型

Kotlin 为每个扁平约束创建 $u_i\in\{0,1\}$，并使用[不等式指标](./inequality)中的两条规范化关系约束进行连接。令 $c=\sum_i u_i$。没有数量范围时，$c$ 就是结果；给定范围 $[l,u]$ 时，另建 $y\in\{0,1\}$ 并传入

$$
c\ge l\,y,
\qquad
c\le u+n(1-y).
$$

因此 $y=1\Rightarrow l\le c\le u$；仅凭这两条约束并不会强制反向蕴含。Rust 包装器接收既有指标，注册 $r-\sum_i u_i=0$，并把请求的数量范围作为硬边界，不创建 Kotlin 的独立 $y$。

> [!WARNING]
> 当前注册循环只有在两个范围端点都存在时才为输入编码。缺少端点的输入可能留下没有对应 solver 约束的标志；请提供有限且有序的 `lhsRange`。

## 当前 API

### Kotlin

源码：[`SatisfiedAmountInequality.kt`（`SatisfiedAmountInequalityFunction` 与变体）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/SatisfiedAmountInequality.kt#L56-L526)

使用 `SatisfiedAmountInequalityFunction.from` 可得到原始计数或自定义 amount 范围；便捷类使用各自的 `from` 工厂。所有变体共享同一套标志和 amount 指标，是同一个基础实现的包装。

```kotlin
import fuookami.ospf.kotlin.core.model.mechanism.LinearConstraintInput
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.AnyFunction
import fuookami.ospf.kotlin.core.symbol.function.SatisfiedAmountInequalityFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.value_range.Interval
import fuookami.ospf.kotlin.math.algebra.value_range.ValueRange
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.inequality.LinearInequality
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val one = LinearPolynomial<Flt64>(emptyList(), Flt64.one)
val lhsRange = ValueRange(
    lb = Flt64(-1000.0),
    ub = Flt64(1000.0),
    lbInterval = Interval.Closed,
    ubInterval = Interval.Closed,
    constants = Flt64.zero.constants
).value!!
val input = LinearConstraintInput.from(
    relation = LinearInequality(xPoly, one, Comparison.LE, "x_le_1"),
    lhsRange = lhsRange,
    rhsConstant = Flt64.one
).value!!
val any = AnyFunction.from(
    inputs = listOf(input),
    converter = IntoValue.Identity,
    name = "any"
)
val value = any.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero))
check(value == Flt64.one)
```

### Rust

Rust 没有直接对应的 `SatisfiedAmountInequalityFunction`，不能直接接收 Kotlin 的 `LinearConstraintInput`、`lhsRange`、`rhsConstant` 与 epsilon。Rust 模块提供基于 [`SatisfiedAmountFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/satisfied_amount.rs) 的指标计数薄包装器：

```rust
AnyFunction::new(id: u64, name: &str, indicators: Vec<BinaryVariableItem>) -> Self
AllFunction::new(id: u64, name: &str, indicators: Vec<BinaryVariableItem>) -> Self
AtLeastInequalityFunction::new(
    id: u64,
    name: &str,
    indicators: Vec<BinaryVariableItem>,
    amount: usize,
) -> Self
NotAllFunction::new(id: u64, name: &str, indicators: Vec<BinaryVariableItem>) -> Self
NumerableFunction::new(
    id: u64,
    name: &str,
    indicators: Vec<BinaryVariableItem>,
    lower: usize,
    upper: usize,
) -> Self
```

这五个 Rust API 都接收已经创建的二值指标，并通过 `result_variable()` 暴露连续计数；amount 范围是硬边界，不是 Kotlin 那样独立的二值 amount 指标。若要从不等式组合，应先创建 `InequalityFunction` 指标，再把其 `result_variable().clone()` 传给上述包装器。有限范围和 Big-M 仍由该指标函数负责。

## evaluate 与 solver 的差异

直接求值计算扁平化输入值，并使用 `epsilon` 与零比较。solver 注册使用声明的有限范围和 Big-M 约束，不会自动从普通关系推导缺失范围。给出 amount 范围时，直接求值检查闭区间；solver 使用二值指标和放松的计数上下界。

## 边界、tolerance 与 Undefined

缺少符号时直接求值返回 `null`。空输入列表不是有用的计数模型；单输入的 `NotAllFunction` 特意使用 `amount = null`，因此直接结果是原始计数，而不是“非全满足”的布尔值。当前各构造器的校验机制并不统一：`AtLeastInequalityFunction` 通过断言要求 $0<k\le n$。该函数没有 `TruthValue.Undefined` 输出。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.model.mechanism.LinearConstraintInput
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.AnyFunction
import fuookami.ospf.kotlin.core.symbol.function.SatisfiedAmountInequalityFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.algebra.value_range.Interval
import fuookami.ospf.kotlin.math.algebra.value_range.ValueRange
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.inequality.LinearInequality
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val xPoly = LinearPolynomial(listOf(LinearMonomial(Flt64.one, x)), Flt64.zero)
val one = LinearPolynomial<Flt64>(emptyList(), Flt64.one)
val lhsRange = ValueRange(
    lb = Flt64(-1000.0),
    ub = Flt64(1000.0),
    lbInterval = Interval.Closed,
    ubInterval = Interval.Closed,
    constants = Flt64.zero.constants
).value!!
val input = LinearConstraintInput.from(
    relation = LinearInequality(xPoly, one, Comparison.LE, "x_le_1"),
    lhsRange = lhsRange,
    rhsConstant = Flt64.one
).value!!
val any = AnyFunction.from(
    inputs = listOf(input),
    converter = IntoValue.Identity,
    name = "any"
)
val value = any.evaluate(mapOf<Symbol, Flt64>(x to Flt64.zero))
check(value == Flt64.one)
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{AnyFunction, InequalityFunction};

let x = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let le = InequalityFunction::less_equal(1, "x_le_1", x, 1.0_f64, 10.0_f64);
let any = AnyFunction::<f64>::new(
    2,
    "any",
    vec![le.result_variable().clone()],
);
assert_eq!(any.amount_range(), (Some(1), None));
let _count = any.result_variable();
```

:::

- Core 求值测试：[`SatisfiedAmountFunctionsGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/SatisfiedAmountFunctionsGenericEvaluateTest.kt)
- Core 注册测试：[`FunctionSymbolSatisfiedAmountInequalityGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolSatisfiedAmountInequalityGenericRegistrationTest.kt)
- 示例目录（当前没有专门的满足数量不等式文件）：[linear_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function)

将 `le` 和 `any` 一起注册到模型中。Rust 源码：[`satisfied_amount_inequality.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/satisfied_amount_inequality.rs) 和 [`inequality.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/inequality.rs)。

## 相关页面

- [不等式指示函数](./inequality)
- [满足数量](./satisfied-amount)
- [同状态](./same-as)
