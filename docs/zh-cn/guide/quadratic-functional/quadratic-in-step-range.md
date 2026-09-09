# 二次步进区间

`QuadraticInStepRangeFunction` 用闭区间门控二次多项式：当值在区间内返回该多项式，否则返回 zero。

> [!WARNING]
> 尽管名称包含“步进”，当前二次实现不会按步长取整，而是区间门控。

## 契约

- 输入：`x: QuadraticPolynomial<V>` 以及标量边界 `lower`/`upper`。
- 构造器要求 $lower\le upper$。
- 直接求值在 $lower\le x\le upper$ 时返回 `x`，否则返回 zero；符号缺失时返回 `null`。
- solver 辅助变量是二值 `z` 和实数 `y`；公开多项式是辅助结果。
- 泛型值要求 `V : RealNumber<V>, V : Ring<V>, V : NumberField<V>`，并配合 `IntoValue<V>` 转换器。

## 定义与数学模型

对 $p=x$ 及 $z,y$：

$$
z=1\Longleftrightarrow lower\le p\le upper,\qquad
y=\begin{cases}p,&z=1,\\0,&z=0.\end{cases}
$$

当前注册六条 Big-M 约束：

$$
\begin{aligned}
p+M(1-z)&\ge lower,&p-M(1-z)&\le upper,\\
y-p+M(1-z)&\ge0,&y-p-M(1-z)&\le0,\\
y&\le Mz,&y\ge-Mz.
\end{aligned}
$$

## 实现、辅助变量与约束

实现创建名称 `name` 加 `_z` 的二值变量和名称 `name` 加 `_y` 的实数变量，先注册两者，再添加上面的区间门控、开启等式和关闭归零约束。省略 Big-M 时，从二次输入推导范围。

## 当前 API

### Kotlin

源码：[`QuadraticInStepRange.kt`（`QuadraticInStepRangeFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticInStepRange.kt#L48-L375)

```kotlin
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticInStepRangeFunction
import fuookami.ospf.kotlin.core.token.AutoTokenTable
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Quadratic
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val y = RealVar("y")
val polynomial = QuadraticPolynomial(
    monomials = listOf(
        QuadraticMonomial.quadratic(Flt64.one, x, y),
        QuadraticMonomial.linear(Flt64.one, x)
    ),
    constant = Flt64.one
)
val function = QuadraticInStepRangeFunction(
    x = polynomial,
    lower = Flt64.zero,
    upper = Flt64(10.0),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "quadratic_step"
)
val tokens = AutoTokenTable<Flt64>(Quadratic, false)
tokens.add(listOf(x, y))
val value = function.prepare(
    mapOf<Symbol, Flt64>(x to Flt64.one, y to Flt64.two),
    tokens,
    IntoValue.Identity
)
check(value == Flt64(4.0))
tokens.close()
```

### Rust

Rust 的 [`QuadraticInStepRangeFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_in_step_range.rs) 与 Kotlin 的区间门控不是同一个语义，而是步进值函数。公开构造器为：

```rust
QuadraticInStepRangeFunction::new(
    id: u64,
    name: &str,
    input: Quadratic<V>,
    lower: V,
    upper: V,
    step: V,
) -> QuadraticInStepRangeFunction<V>

QuadraticInStepRangeFunction::with_quadratic_bounds(
    id: u64,
    name: &str,
    lower: Quadratic<V>,
    upper: Quadratic<V>,
    step: V,
) -> QuadraticInStepRangeFunction<V>
```

`new` 把 `input` 当作运行时上界，使用标量下界，并把 `upper` 作为硬上限。直接求值返回 `lower + floor((min(upper_value, cap) - lower) / |step|) * |step|`；接近零的 step 返回下界。实现创建名称为 `name + "_in_step_range"` 的结果变量，并通过 `QuadraticLinearFunction` 桥接二次边界。因此它与 Kotlin 闭区间门控没有一一对应的 solver 契约。

## evaluate 与 solver 的差异

直接求值只计算 $p$ 并检查闭区间，不读取辅助 z/y 值。solver 注册创建 z/y 并强制上面的六条约束。缺失或不足的 Big-M 可能使 solver 松弛不准确或不可行，即使直接求值有定义。

## 边界、tolerance 与 Undefined

下界和上界均包含。边界逆序时构造器失败。该函数没有 tolerance 或三值 Undefined 状态；直接求值对刚好在区间外的值返回 zero。控制变量预期为二值，但构造器接受通用变量项，本身不强制其类型。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticInStepRangeFunction
import fuookami.ospf.kotlin.core.token.AutoTokenTable
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Quadratic
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val y = RealVar("y")
val polynomial = QuadraticPolynomial(
    monomials = listOf(
        QuadraticMonomial.quadratic(Flt64.one, x, y),
        QuadraticMonomial.linear(Flt64.one, x)
    ),
    constant = Flt64.one
)
val function = QuadraticInStepRangeFunction(
    x = polynomial,
    lower = Flt64.zero,
    upper = Flt64(10.0),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "quadratic_step"
)
val tokens = AutoTokenTable<Flt64>(Quadratic, false)
tokens.add(listOf(x, y))
val value = function.prepare(
    mapOf<Symbol, Flt64>(x to Flt64.one, y to Flt64.two),
    tokens,
    IntoValue.Identity
)
check(value == Flt64(4.0))
tokens.close()
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticInStepRangeFunction;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
let upper = Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 0.0);
let step = QuadraticInStepRangeFunction::new(11, "qstep", upper, 0.0, 4.0, 2.0);
let mut tokens = VecTokenList::<f64>::new();
let tx = Token::from_generic(x, 0);
tx.set_result(3.0);
tokens.add_token(tx);
assert_eq!(step.calculate_value(&tokens, false), Some(2.0));
```

:::

- Core 求值：[`QuadraticFunctionGenericEvaluationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionGenericEvaluationTest.kt)
- Core 注册：[`FunctionSymbolGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolGenericRegistrationTest.kt)
- 示例目录（当前没有专门的二次步进区间文件）：[quadratic_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/quadratic_function)

- 源码与求值/集成覆盖：[`quadratic_in_step_range.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_in_step_range.rs) 与 [`quadratic_function.rs` 测试](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)

## 相关页面

- [线性步进区间](../linear-functional/in-step-range)
- [二次掩码范围](./quadratic-masking-range)
