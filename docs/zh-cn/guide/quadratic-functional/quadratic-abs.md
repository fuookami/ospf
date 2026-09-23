# 二次绝对值

`QuadraticAbsFunction` 在有界二次多项式之上组合线性绝对值函数。对输入多项式 $p(x)$：

$$
y=|p(x)|。
$$

真正含二次项的输入先由共享基类 `QuadraticFunctionSymbol<V>` 绑定到桥接变量，随后线性 `AbsFunction` 在该桥接上施加带分支 Big-M 的正负部分解。

## 契约

- 输入：`polynomial: QuadraticPolynomial<V>`（Kotlin）或 `input: Quadratic<V>`（Rust）。
- 若输入含二次项，基类创建一个有界桥接 `RealVar`，名称为 `${name}_input_0`，并注册一条精确二次等式 `${name}_input_0`：input = bridge。桥接范围收紧到输入的有限边界，仅在浮点可表示性需要时放宽一个 ULP。
- 注册时校验每个输入具有有限且未扩大的边界；扩大已捕获的边界会被拒绝，收紧始终安全。
- 仿射（线性/常数）输入直接复用，不引入桥接变量。
- `createFunction` 返回 `AbsFunction`，因此辅助变量为桥接变量加上线性绝对值的辅助变量 `${name}_abs`、`${name}_abs_pos`、`${name}_abs_neg` 以及二值变量 `${name}_abs_sign`（纯仿射输入时为四个辅助变量，无桥接变量）。
- 结果 `polynomial` 是绝对值结果变量提升后的二次多项式，只含一次单项式。
- 泛型值要求 `V : RealNumber<V>, V : NumberField<V>`，并配合 `IntoValue<V>` 转换器。
- 公式可能是非凸 MIQCP（二次输入通过等式约束等于一个变量）；需要求解器支持非凸二次约束。

## 求解器数学模型

### Kotlin

令输入为 $p(x)$，有限值域为 $[L,U]$。基类提交恰好一条二次等式

$$
p(x)-b=0,\qquad b\in[\tilde L,\tilde U],
$$

其中 $b$ 是桥接变量 `${name}_input_0`，$[\tilde L,\tilde U]$ 是捕获的输入值域，仅在边界不可被浮点精确表示时放宽一个 ULP。线性 `AbsFunction` 随后把桥接值分解为非负部分：

$$
b=b^+-b^-,\qquad y=b^++b^-,
$$

并对二值选择变量 $s$ 施加分支约束 $b^+\le M^+s$、$b^-\le M^-(1-s)$。两侧 Big-M 按输入有限值域逐侧推导（$M^+$ 来自上界，$M^-$ 来自下界的相反数），值域未知时回退到库默认值。

### Rust

Rust 组合相同的两个阶段：`QuadraticLinearFunction` 桥接产出恰好一条二次等式 $q-b=0$（行名 `{name}_bridge_quad_eq`），随后线性 `AbsFunction` 的四条分支约束作用于桥接列：

$$
y-b\ge 0,\qquad y+b\ge 0,\qquad y-b+M^+s\le M^+,\qquad y+b-M^-s\le 0。
$$

非对称分支 Big-M 按固定顺序解析：显式 `with_big_m`/`with_branch_big_m` 配置优先并在约束生成阶段校验（非有限或非正值导致约束生成失败），其次从 token 边界推断覆盖 $\max(0,-2L)$ 与 $\max(0,2U)$ 的取值，最后回退到策略默认值。通过 `from_linear` 创建的纯线性输入直接退化：不注册桥接列，也不产生二次约束。输入值域有限时，结果列被收紧到 $0\le y\le\max(|L|,|U|)$；边界只收紧、不放宽。

## 当前 API

### Kotlin

源码：[`QuadraticAbs.kt`（`QuadraticAbsFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticAbs.kt)，组合自基类 [`QuadraticFunctionSymbol.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionSymbol.kt)。

```kotlin
QuadraticAbsFunction(
    polynomial: QuadraticPolynomial<V>,
    bigM: V? = null,
    converter: IntoValue<V>,
    name: String = "quadratic_abs",
    displayName: String? = null
)
```

可选的 `bigM` 透传给线性 `AbsFunction`；缺省时按输入有限值域逐侧推导 Big-M，无法推导时回退到库默认值。

### Rust

Rust 的 [`QuadraticAbsFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs) 先桥接二次输入，再施加线性绝对值的分支约束：

```rust
QuadraticAbsFunction::new(id: u64, name: &str, input: Quadratic<V>) -> Self
QuadraticAbsFunction::with_big_m(id: u64, name: &str, input: Quadratic<V>, big_m: V) -> Self
QuadraticAbsFunction::with_branch_big_m(id: u64, name: &str, input: Quadratic<V>, big_m: AbsBranchBigM) -> Self
QuadraticAbsFunction::from_linear(id: u64, name: &str, input: Linear<V>) -> Self
```

`with_big_m` 对两条分支约束使用同一个取值，`with_branch_big_m` 接受非对称配对 `AbsBranchBigM { positive_branch, negative_branch }`。`result_variable()` 返回非负结果列，`side_variable()` 返回二值分支指示列，`bridge_variable()` 返回桥接列（仅当 `has_quadratic_input()` 成立时才注册进模型）。`big_m()` 在显式配置时返回该配对。

## evaluate 与 solver 的差异

直接求值解析原始输入（包括二次单项式），填入桥接值后委托给线性 `AbsFunction`，因此两条路径描述同一个 $|p(x)|$；符号缺失时返回 `null`。求解器路径还依赖有效的分支 Big-M 覆盖：过小的显式 Big-M 可能把真实值排除在可行域之外，而 `evaluate` 仍然成功。该函数没有 tolerance 或三值未定义状态。

## 最小示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticAbsFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial

val x = RealVar("x").also {
    it.range.geq(Flt64(-2.0))
    it.range.leq(Flt64(2.0))
}
val polynomial = QuadraticPolynomial(
    monomials = listOf(QuadraticMonomial.quadratic(Flt64.one, x, x)),
    constant = Flt64(-1.0)
)
val abs = QuadraticAbsFunction(
    polynomial = polynomial,
    converter = IntoValue.Identity,
    name = "quadratic_abs"
)
val value = abs.evaluate(
    values = mapOf<Symbol, Flt64>(x to Flt64(2.0)),
    tokenTable = null,
    converter = IntoValue.Identity,
    zeroIfNone = false
)
check(value == Flt64(3.0))
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticAbsFunction;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
let mut tokens = VecTokenList::<f64>::new();
let tx = Token::from_generic(x, 0);
tx.set_result(2.0);
tokens.add_token(tx);
let input = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], -1.0);
let abs = QuadraticAbsFunction::new(1, "quadratic_abs", input);
assert_eq!(abs.calculate_value(&tokens, false), Some(3.0));
```

:::

## 测试与参考

- Kotlin 组合、注册与求值覆盖：[`QuadraticFunctionCompositionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionCompositionTest.kt)
- Rust 组合验收覆盖（桥接等式、分支约束与 Big-M 解析）：[`quadratic_composition_acceptance.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/quadratic_composition_acceptance.rs)；目前没有专门的 `function_symbol_quadratic_abs.rs` 文件。

## 相关页面

- [绝对值](../linear-functional/abs)：底层线性分解模型。
- [二次线性](./quadratic-linear)：共享的二次桥接。
- [二次乘积](./product)：显式二次单项式乘积。
