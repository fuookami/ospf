# 二次掩码

`QuadraticMaskingFunction` 用二值掩码门控有界二次多项式。对输入多项式 $p(x)$ 与二值掩码 $z$：

$$
y = \begin{cases}p(x), & z=1,\\ 0, & z=0,\end{cases}\qquad z\in\{0,1\}。
$$

只有真正含二次项的输入会被共享基类 `QuadraticFunctionSymbol<V>` 绑定到桥接变量；掩码作为第二个纯线性输入传给基类，因此自身不获得桥接。线性 `MaskingFunction` 随后在桥接上施加四条乘积约束。把掩码保持为独立的二值因子，避免了把二次输入与掩码直接相乘所需要的三次或四次展开。

## 契约

- 输入：`input: QuadraticPolynomial<V>` 与 `mask: BinVar`。
- 掩码作为第二个（线性）输入传入，基类不为它创建桥接变量；只有二次输入注册桥接变量 `${name}_input_0` 及一条精确二次等式。
- `createFunction` 返回 `MaskingFunction`，因此求解器侧结果是有符号 `RealVar` `${name}_masking`，四条乘积约束依赖线性侧的 Big-M（默认按输入有限值域推导，失败时回退到库默认值）。
- 直接求值在掩码缺失或为零时返回零，否则返回输入值，因此掩码开启时负的 $p(x)$ 保持为负。
- 泛型值要求 `V : RealNumber<V>, V : NumberField<V>`，并配合 `IntoValue<V>` 转换器。
- 公式可能是非凸 MIQCP（二次输入通过等式约束等于一个变量）；需要求解器支持非凸二次约束。

## 求解器数学模型

### Kotlin

令输入为 $p(x)$，有限值域为 $[L,U]$。基类提交恰好一条二次等式

$$
p(x)-b=0,\qquad b\in[\tilde L,\tilde U],
$$

其中 $b$ 是桥接变量 `${name}_input_0`。线性 `MaskingFunction` 随后注册有符号结果变量 `${name}_masking`，并在 $(b,z)$ 上提交四条乘积约束：

$$
y\le Uz,\qquad y\ge Lz,\qquad y-b\le -L(1-z),\qquad y-b\ge -U(1-z)。
$$

缺失的输入边界由线性侧 Big-M 的 $\pm M$ 代替。

### Rust

Rust 把桥接 `QuadraticLinearFunction`（结果列 `{name}_bridge_lin_y`）与建立在该列上的内层 `MaskingFunction` 组合，内层使用调用方传入的 `BinaryVariableItem`。机制路径提交桥接的二次等式以及相同的四条乘积约束；当 token 边界可用时，内层 Big-M 会按原始二次输入的边界重新推导，且不低于策略最小值，否则使用构造时传入的取值。掩码必须是 `BinaryVariableItem`；`result_variable()` 返回有符号结果列，`mask_variable()` 返回掩码变量。

## 当前 API

### Kotlin

源码：[`QuadraticMasking.kt`（`QuadraticMaskingFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticMasking.kt)，组合自基类 [`QuadraticFunctionSymbol.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionSymbol.kt)。

```kotlin
QuadraticMaskingFunction(
    input: QuadraticPolynomial<V>,
    mask: BinVar,
    bigM: V? = null,
    converter: IntoValue<V>,
    name: String = "quadratic_masking",
    displayName: String? = null
)
```

可选的 `bigM` 透传给线性 `MaskingFunction`；缺省时按输入有限值域推导，无法推导时回退到库默认值。

### Rust

Rust 的 [`QuadraticMaskingFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs) 接收平展的 `Quadratic<V>` 与二值掩码变量：

```rust
QuadraticMaskingFunction::new(id: u64, name: &str, input: Quadratic<V>, mask_var: BinaryVariableItem) -> Self
QuadraticMaskingFunction::with_big_m(id: u64, name: &str, input: Quadratic<V>, mask_var: BinaryVariableItem, big_m: V) -> Self
```

`new` 让内层掩码函数从默认 Big-M 出发，`with_big_m` 显式指定取值。`result_variable()` 返回有符号连续结果列，`mask_variable()` 返回二值掩码。

## evaluate 与 solver 的差异

直接求值从给定值中解析原始二次输入与掩码：掩码缺失或为零时返回零，掩码非零时返回 $p(x)$，因此掩码开启时负值可以透传。求解器假设掩码变量是二值的，所以只有 $y=b$ 与 $y=0$ 两种预期情况可行，桥接等式保证 $b=p(x)$ 精确成立。非二值的掩码取值可以被 `evaluate` 接受，但对求解器模型不是合法赋值。

## 最小示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticMaskingFunction
import fuookami.ospf.kotlin.core.variable.BinVar
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial

val x = RealVar("x").also {
    it.range.geq(Flt64(-2.0))
    it.range.leq(Flt64(2.0))
}
val mask = BinVar("mask")
val input = QuadraticPolynomial(
    monomials = listOf(QuadraticMonomial.quadratic(Flt64.one, x, x)),
    constant = Flt64(-1.0)
)
val masking = QuadraticMaskingFunction(
    input = input,
    mask = mask,
    converter = IntoValue.Identity,
    name = "quadratic_masking"
)
val masked = masking.evaluate(
    values = mapOf<Symbol, Flt64>(x to Flt64(2.0), mask to Flt64.one),
    tokenTable = null,
    converter = IntoValue.Identity,
    zeroIfNone = false
)
check(masked == Flt64(3.0))
val gatedOff = masking.evaluate(
    values = mapOf<Symbol, Flt64>(x to Flt64(2.0), mask to Flt64.zero),
    tokenTable = null,
    converter = IntoValue.Identity,
    zeroIfNone = false
)
check(gatedOff == Flt64.zero)
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticMaskingFunction;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{BinaryVariableItem, ContinuousVariableItem, VariableId};

let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
let mask = BinaryVariableItem::create(VariableId::standalone(2), "mask");
let mut tokens = VecTokenList::<f64>::new();
let tx = Token::from_generic(x, 0);
tx.set_result(2.0);
tokens.add_token(tx);
let tm = Token::from_generic(mask.clone(), 2);
tm.set_result(1.0);
tokens.add_token(tm);
let input = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
let masking = QuadraticMaskingFunction::with_big_m(1, "quadratic_masking", input, mask, 10.0);
assert_eq!(masking.calculate_value(&tokens, false), Some(4.0));
```

:::

## 测试与参考

- Kotlin 组合、注册与求值覆盖：[`QuadraticFunctionCompositionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionCompositionTest.kt)，其中包含专门的行可行性用例 `maskingConstraintsRejectIncorrectResultAndIncorrectBridge`。
- Rust 专门契约测试：[`function_symbol_quadratic_masking.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_masking.rs)；端到端求解覆盖：[`gurobi_quadratic_model_integration.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_quadratic_model_integration.rs)（`gurobi_solves_quadratic_masking_with_non_linear_input`）。

## 相关页面

- [掩码](../linear-functional/masking)：底层四行乘积线性化。
- [二次掩码范围](./quadratic-masking-range)：二值门控的范围变体。
- [二次线性](./quadratic-linear)：共享的二次桥接。
