# 二次掩码范围

`QuadraticMaskingRangeFunction` 使用调用方控制变量门控二次多项式。

> [!WARNING]
> Kotlin 实现名称中的“范围”不是上下界参数。当 `z=1` 时强制 `y=polynomial`；当 `z=0` 时，`y` 在变量自身边界内自由变化。它不会在关闭分支强制 `y=0`。Rust 有独立的 `QuadraticMaskingRangeFunction` 契约，见下文。

## 契约

- 输入：`polynomial: QuadraticPolynomial<V>` 和控制量 `z: AbstractVariableItem<*, *>`，预期为二值。
- 输出/辅助变量：名称在 `name` 后追加 `_y` 的实数 `resultVar`。
- 直接求值对缺失/零 z 返回 zero，对其他 z 值返回多项式值。
- solver 注册使用两条二次 Big-M 不等式，在 z 为 one 时连接 y 与多项式。
- 泛型值要求 `V : RealNumber<V>, V : Ring<V>, V : NumberField<V>`，并配合 `IntoValue<V>` 转换器。

## 定义与数学模型

令 $p$ 为二次输入，$y$ 为辅助结果：

$$
y-p\le M(1-z),\qquad y-p\ge-M(1-z).
$$

因此 $z=1\Rightarrow y=p$，但 z=0 只会放松关系。直接求值使用更强的过程约定 $z=0\Rightarrow0$，当前 solver 约束没有完全实现该约定。

## 实现、辅助变量与约束

实现创建并注册一个实数 `resultVar`，添加上面的两条二次 Big-M 不等式；没有添加线性掩码函数使用的两个关闭归零边界。Big-M 默认从二次多项式范围推导。

## 当前 API

### Kotlin

源码：[`QuadraticMaskingRange.kt`（`QuadraticMaskingRangeFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticMaskingRange.kt#L41-L345)

```kotlin
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticMaskingRangeFunction
import fuookami.ospf.kotlin.core.token.AutoTokenTable
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Quadratic
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial
import fuookami.ospf.kotlin.core.variable.BinVar
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val y = RealVar("y")
val z = BinVar("z")
val polynomial = QuadraticPolynomial(
    monomials = listOf(
        QuadraticMonomial.quadratic(Flt64.one, x, y),
        QuadraticMonomial.linear(Flt64.one, x)
    ),
    constant = Flt64.one
)
val function = QuadraticMaskingRangeFunction(
    polynomial = polynomial,
    z = z,
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "quadratic_mask"
)
val tokens = AutoTokenTable<Flt64>(Quadratic, false)
tokens.add(listOf(x, y, z))
val value = function.prepare(
    mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0), z to Flt64.one),
    tokens,
    IntoValue.Identity
)
check(value == Flt64(13.0))
tokens.close()
```

### Rust

Rust 提供独立的 [`QuadraticMaskingRangeFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_masking_range.rs)，并不是 Kotlin 的“用 z 掩码多项式”操作。构造器为：

```rust
QuadraticMaskingRangeFunction::new(
    id: u64,
    name: &str,
    mask: Quadratic<V>,
    lower: V,
    upper: V,
) -> QuadraticMaskingRangeFunction<V>

QuadraticMaskingRangeFunction::with_quadratic_bounds(
    id: u64,
    name: &str,
    mask: Quadratic<V>,
    lower: Quadratic<V>,
    upper: Quadratic<V>,
) -> QuadraticMaskingRangeFunction<V>
```

Rust 创建 `name + "_masking_range"` 结果变量，桥接 mask 与两个边界，并注册二次不等式 `result <= upper * mask` 与 `result >= lower * mask`。直接求值在 mask 数值为零时返回 zero；否则把结果 token 截断到 `lower * mask` 和 `upper * mask`（运行时排序）形成的区间。它没有二值 `z` 参数，也没有直接的 `y = polynomial` 分支，因此两种 API 不能互换。

## evaluate 与 solver 的差异

直接求值按过程检查 z，z 恰为 zero 时返回 zero；z 非零时才会因多项式符号缺失返回 `null`。solver 注册只在 z=1 时把 y 与 p 连接；z=0 时 y 可在变量域内取任意值，只受 Big-M 放松约束影响。直接求值也接受非二值的非零 z，但这超出 solver 预期的二值契约。

## 边界、tolerance 与 Undefined

直接求值缺少 z 时返回 zero；z 非零时，多项式符号缺失返回 `null`。Big-M 必须覆盖二次范围。该函数没有 tolerance 或三值 Undefined 状态。`resultVar` 是 `RealVar`，solver 结果服从当前变量域；依赖关闭分支负值前应先检查该域。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticMaskingRangeFunction
import fuookami.ospf.kotlin.core.token.AutoTokenTable
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Quadratic
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial
import fuookami.ospf.kotlin.core.variable.BinVar
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val y = RealVar("y")
val z = BinVar("z")
val polynomial = QuadraticPolynomial(
    monomials = listOf(
        QuadraticMonomial.quadratic(Flt64.one, x, y),
        QuadraticMonomial.linear(Flt64.one, x)
    ),
    constant = Flt64.one
)
val function = QuadraticMaskingRangeFunction(
    polynomial = polynomial,
    z = z,
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "quadratic_mask"
)
val tokens = AutoTokenTable<Flt64>(Quadratic, false)
tokens.add(listOf(x, y, z))
val value = function.prepare(
    mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0), z to Flt64.one),
    tokens,
    IntoValue.Identity
)
check(value == Flt64(13.0))
tokens.close()
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticMaskingRangeFunction;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

let mask_var = ContinuousVariableItem::create(VariableId::standalone(0), "mask");
let mask = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
let range = QuadraticMaskingRangeFunction::new(14, "qmask_range", mask, -2.0, 3.0);
let mut tokens = VecTokenList::<f64>::new();
let tm = Token::from_generic(mask_var, 0);
tm.set_result(1.0);
tokens.add_token(tm);
let ty = Token::from_generic(range.result_variable().clone(), range.result_variable().index());
ty.set_result(2.5);
tokens.add_token(ty);
assert_eq!(range.calculate_value(&tokens, false), Some(2.5));
```

:::

- Core 求值：[`QuadraticFunctionGenericEvaluationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionGenericEvaluationTest.kt)
- Core 注册：[`FunctionSymbolGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolGenericRegistrationTest.kt)
- 示例目录（当前没有专门的二次掩码范围文件）：[quadratic_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/quadratic_function)

- 源码与针对性测试：[`quadratic_masking_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_masking_range.rs) 与 [`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)

## 相关页面

- [线性掩码](../linear-functional/masking)
- [二次步进区间](./quadratic-in-step-range)
