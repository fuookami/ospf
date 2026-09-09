# 掩码

`MaskingFunction` 对线性多项式与二进制掩码的乘积建模：

$$
y = p\,z,\qquad z\in\{0,1\}.
$$

本页以当前 `ospf-kotlin-core` 实现作为唯一契约。

## 契约

- 输入：`input: LinearPolynomial<V>` 与 `mask: AbstractVariableItem<*, *>`。
- 掩码的语义是二进制变量（`BinVar`）。
- 输出：`RealVar`，通过 `resultPolynomial` 暴露。
- `evaluate` 在掩码缺失或等于零时返回零，否则求值并返回 `input`（因此传入的非二进制非零值会被视为“开启”）。
- `V` 必须实现 `RealNumber<V>` 与 `NumberField<V>`，并配套 `IntoValue<V>` 转换器。

## 数学定义

对于有效二进制掩码，

$$
y = \begin{cases}p,&z=1,\\0,&z=0.\end{cases}
$$

当存在有限界 $L\le p\le U$ 时，四条标准线性不等式为

$$
y\le Uz,\quad y\ge Lz,\quad y-p\le -L(1-z),\quad y-p\ge -U(1-z).
$$

实现会在可能时从输入多项式取得 $L,U$；否则使用 $-M,M$。

## 适用域与边界

求解器语义要求 `mask` 只能取 0 或 1。非零掩码值虽然会被 `evaluate` 接受，但不是二进制求解器模型的有效赋值。如果输入没有有限界，应显式传入 `bigM`；当前默认回退值为 $10^6$。结果不强制为非负，因此只要 Big-M 有效，也支持负输入范围。

## 当前 API

### Kotlin

源码：[`Masking.kt`（`MaskingFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Masking.kt#L42-L122)

```kotlin
MaskingFunction(
    input: LinearPolynomial<V>,
    mask: AbstractVariableItem<*, *>,
    bigM: V? = null,
    converter: IntoValue<V>,
    name: String,
    displayName: String? = null
)
```

同一源码文件还包含 `MaskingWithPolyMaskFunction` 与 `MaskingRangeFunction`；它们是不同的 API，不应替代本页描述的二进制掩码契约。

### Rust

Rust 提供二值变量对应物 [`MaskingFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/masking.rs)：

```rust
MaskingFunction::new(
    id: u64,
    name: &str,
    input: Linear<V>,
    mask_var: BinaryVariableItem,
) -> MaskingFunction<V>

MaskingFunction::with_big_m(
    id: u64,
    name: &str,
    input: Linear<V>,
    mask_var: BinaryVariableItem,
    big_m: V,
) -> MaskingFunction<V>
```

Rust 要求掩码本身是 `BinaryVariableItem`，而 Kotlin 接收抽象变量项并依赖调用方遵守二值契约。Rust 模块也直接提供两个 Kotlin 变体的对应物：[`MaskingWithPolyMaskFunction::new`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/masking.rs) / `with_big_m` 接收 `Linear<V>` 掩码表达式并创建二值桥接变量；[`MaskingRangeFunction::new`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/masking.rs) 接收线性掩码以及 `lower`、`upper`。`result_variable()`、`mask_variable()`/`mask_bridge_variable()` 和 `big_m()` 暴露 Rust 状态；没有 Kotlin 的 converter 或 `displayName` 参数。

## 辅助变量与注册模型

只创建并注册 `resultVar`。`registerConstraints` 添加四条 Big-M 不等式。实现会在可能时使用输入的有限下界/上界，否则对称地使用传入或默认的 Big-M。

## `evaluate` 与 solver 的差异

`evaluate` 直接查找 `mask`：掩码缺失和精确为零都会返回零，其余值都会求值输入。solver 注册 `resultVar` 并假定掩码变量是二进制，因此只强制实现预期的两种情况。如果调用者用不完整映射或非二进制掩码做求值，这一差异会产生影响。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.MaskingFunction
import fuookami.ospf.kotlin.core.variable.BinVar
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.eq
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

val x = RealVar("x")
val mask = BinVar("mask")
val input = LinearPolynomial(
    monomials = listOf(LinearMonomial(Flt64.one, x)),
    constant = Flt64.one
)
val masking = MaskingFunction(
    input = input,
    mask = mask,
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "masking"
)
val value = masking.evaluate(mapOf<Symbol, Flt64>(x to Flt64(5.0), mask to Flt64.one))
check(value != null && (value eq Flt64(6.0)))
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::MaskingFunction;
use ospf_rust_core::variable::{BinaryVariableItem, VariableId};

let mask = BinaryVariableItem::create(VariableId::standalone(2), "mask");
let input = Linear::new(vec![LinearMonomial::new(1.0, 0)], 1.0);
let masking = MaskingFunction::with_big_m(1, "masking", input, mask, 10.0_f64);
assert_eq!(masking.big_m(), &10.0);
let _result = masking.result_variable();
```

:::

完整示例：[`MaskingTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/MaskingTest.kt)

Core 验证：[`MaxAndMaskingFunctionGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/MaxAndMaskingFunctionGenericEvaluateTest.kt)

Rust 源码与 parity 覆盖：[`masking.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/masking.rs) 和 [`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs)。

## 多项式掩码：`MaskingWithPolyMaskFunction`

此变体接收线性掩码表达式，而不是直接接收变量：

$$
m = maskPoly,\qquad y = input\cdot m,\qquad m\in\{0,1\}.
$$

它创建 `maskVar`（`BinVar`）和 `resultVar`（`RealVar`），注册 `maskPoly = maskVar`，然后应用与 `MaskingFunction` 相同的四个 Big-M 约束。直接 `evaluate` 在掩码表达式缺失或恰为零时返回零，否则计算 `input`；求解器依靠二值等式约束保证预期的掩码域。

源码：[`Masking.kt`（`MaskingWithPolyMaskFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Masking.kt#L227-L430)

```kotlin
val mask = BinVar("mask")
val input = LinearPolynomial(
    monomials = listOf(LinearMonomial(Flt64.one, x)),
    constant = Flt64.one
)
val maskPoly = LinearPolynomial(
    monomials = listOf(LinearMonomial(Flt64.one, mask)),
    constant = Flt64.zero
)
val polyMask = MaskingWithPolyMaskFunction(
    input = input,
    maskPoly = maskPoly,
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "poly_mask"
)
val value = polyMask.evaluate(
    mapOf<Symbol, Flt64>(x to Flt64(5.0), mask to Flt64.one)
)
check(value != null && (value eq Flt64(6.0)))
```

当前没有此变体的专用示例或测试；可结合源码和共享掩码测试，并参考实际的[`linear_function` 示例目录](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function)。

## 掩码范围：`MaskingRangeFunction`

$$
lower\cdot m\le y\le upper\cdot m.
$$

构造器要求 `lower <= upper`，创建 `resultVar`（`URealVar`），并且只注册上述两个不等式；掩码表达式应当是二值的，但该类既不创建也不强制掩码为二值。当 `m` 为二值变量时，`m=0` 给出 `y=0`，`m=1` 给出 `lower\le y\le upper`。

源码：[`Masking.kt`（`MaskingRangeFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Masking.kt#L432-L567)

```kotlin
val mask = BinVar("range_mask")
val maskPoly = LinearPolynomial(
    monomials = listOf(LinearMonomial(Flt64.one, mask)),
    constant = Flt64.zero
)
val rangeMask = MaskingRangeFunction(
    mask = maskPoly,
    lower = Flt64(2.0),
    upper = Flt64(5.0),
    converter = IntoValue.Identity,
    name = "range_mask"
)
val value = rangeMask.evaluate(
    mapOf<Symbol, Flt64>(mask to Flt64.one, rangeMask.resultVar to Flt64(4.0))
)
check(value != null && (value eq Flt64(4.0)))
```

直接求值在掩码缺失或为零时返回零；掩码非零时读取 `resultVar`，若该值缺失则返回零，并将结果截断到缩放后的区间（掩码为负时交换端点）。求解器注册约束不会交换负端点，因此模型中应使用二值且非负的掩码。当前没有专用示例或测试；可参考源码和实际的[`linear_function` 示例目录](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function)。

## 相关页面

- [`abs`](./abs)：把数值拆为正部和负部。
- [`max`](./max) 与 [`min`](./min)：聚合候选多项式。
- [`slack`](./slack) 与 [`slack-range`](./slack-range)：其他有界线性变换。
