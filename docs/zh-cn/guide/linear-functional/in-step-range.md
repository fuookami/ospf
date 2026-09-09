# 步进区间

`InStepRangeFunction` 将两个线性表达式之差向下取整到给定步长的倍数，再加回下界表达式。它返回步进数值，不是布尔成员资格测试。

## 契约

- 输入：`lb: LinearPolynomial<V>`、`ub: LinearPolynomial<V>` 和标量 `step: V`。
- 结果：$lb+\lfloor(ub-lb)/step\rfloor step$，以 `result` 暴露。
- `m` 是传给委托 `FloorFunction` 的可选 Big-M。
- 泛型值使用 `V : RealNumber<V>, V : NumberField<V>`，并配合 `IntoValue<V>` 转换器。
- 当前构造器不会校验 `step` 为正；调用方必须保证步长非零且为正。

## 定义与数学模型

令 $d=ub-lb$，实现计算：

$$
q=\left\lfloor\frac{d}{step}\right\rfloor,\qquad
y=lb+q\cdot step.
$$

例如 $lb=1$、$ub=4$、$step=2$ 时，结果为 $3$。“区间”只描述形成差值时使用的端点；该函数不会返回某个值是否属于集合的布尔值。

## 实现、辅助变量与约束

实现对 $ub-lb$ 构造私有 `FloorFunction`，把 `m` 作为其 Big-M，再将 floor 结果乘以 `step` 并加回 `lb`。辅助变量和约束完全来自委托的向下取整函数，没有独立的成员资格指标。

## 当前 API

### Kotlin

源码：[`InStepRange.kt`（`InStepRangeFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/InStepRange.kt#L45-L140)

```kotlin
InStepRangeFunction(
    lb: LinearPolynomial<V>,
    ub: LinearPolynomial<V>,
    step: V,
    m: V? = null,
    converter: IntoValue<V>,
    name: String = "inStepRange",
    displayName: String? = null
)
```

### Rust

Rust 暴露 [`InStepRangeFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/in_step_range.rs)，但它与上面的 Kotlin 数值步进公式不是同一契约。Rust 判断一个输入是否位于 `[lower, upper]` 且落在 `lower + k * abs(step)` 网格上，并返回二值结果：

```rust
InStepRangeFunction::new(
    id: u64,
    name: &str,
    input: Linear<V>,
    lower: V,
    upper: V,
    step: V,
) -> InStepRangeFunction<V>
```

`result_variable()` 是 `BinaryVariableItem`；`input_polynomial()`、`lower_bound()`、`upper_bound()` 和 `step()` 暴露保存的输入。求值器使用约 `1e-8` 容差，注册时展开步进点，超过 4096 个点会拒绝。因此 Rust 没有 Kotlin `lb + floor((ub - lb) / step) * step` 数值结果的直接对应物；若需要该结果，应组合 Rust [`FloorFunction::new`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/floor.rs) 与调用方构造的线性表达式。

## evaluate 与 solver 的差异

直接 `evaluate` 使用转换后的 Flt64 算术计算 floor。solver 注册委托 `FloorFunction`，因此使用相同的 floor 建模及其 tolerance/Big-M 行为。零或负步长可能造成非法算术，或产生不代表预期步进语义的模型；这是调用方前置条件，不是构造器错误。

## 边界、tolerance 与 Undefined

缺少 `lb` 或 `ub` 符号时返回 `null`。该函数没有 `TruthValue.Undefined` 状态。若 $ub<lb$，实现仍计算数学 floor 表达式（结果可能低于 `lb`），不会截断或拒绝端点顺序。Big-M 推导必须覆盖 $ub-lb$。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.InStepRangeFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val lb = RealVar("lb")
val ub = RealVar("ub")
val lbPoly = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, lb)), Flt64.zero
)
val ubPoly = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, ub)), Flt64.zero
)
val stepped = InStepRangeFunction(
    lb = lbPoly,
    ub = ubPoly,
    step = Flt64.two,
    converter = IntoValue.Identity,
    name = "step"
)
val value = stepped.evaluate(
    mapOf<Symbol, Flt64>(lb to Flt64.one, ub to Flt64(4.0))
)
check(value == Flt64(3.0))
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::InStepRangeFunction;

let input = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let indicator = InStepRangeFunction::new(
    1,
    "in_step_range",
    input,
    0.0_f64,
    4.0_f64,
    2.0_f64,
);
assert_eq!(indicator.step(), &2.0);
let _result = indicator.result_variable();
```

:::

- Core 测试：[`FunctionSymbolDiscreteGenericEvaluateTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolDiscreteGenericEvaluateTest.kt)
- 示例目录（当前没有专门的步进区间文件）：[linear_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function)

Rust 源码：[`in_step_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/in_step_range.rs)。

## 相关页面

- [向下取整](./floor)
- [取模](./mod)
- [松弛（范围）](./slack-range)
