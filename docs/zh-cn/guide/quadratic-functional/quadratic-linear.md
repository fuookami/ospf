# 二次线性

`QuadraticLinearFunction` 将 `QuadraticPolynomial<V>` 包装为二次中间符号，并根据输入有条件地引入结果变量。

## 契约

- 输入：`polynomial: QuadraticPolynomial<V>`。
- 直接求值返回封装多项式的值。
- 若多项式没有二次单项式，符号分类为线性，不注册辅助变量或约束。
- 若存在二次单项式，实现创建名称追加 `_y` 的非负实数辅助变量，并注册 $y=polynomial$。
- 泛型值要求 `V : RealNumber<V>, V : Ring<V>, V : NumberField<V>`，并配合 `IntoValue<V>` 转换器。

## 定义与数学模型

对输入多项式 $p(x)$，只有在确实含二次项时才注册

$$
y=p(x)
$$

等式。公开多项式仍是 $p(x)$；辅助变量是 solver 侧的等式目标，不改变数学表达式。

## 实现、辅助变量与约束

实现检查 `monomial.isQuadratic`。纯线性输入分类为 `Linear`，不注册辅助变量。否则创建 `URealVar`，其名称为 `name` + `_y`，用一个二次等式把辅助变量与多项式连接，并通过 token 表对原始多项式求值。

## 当前 API

### Kotlin

源码：[`QuadraticLinear.kt`（`QuadraticLinearFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticLinear.kt#L39-L325)

```kotlin
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticLinearFunction
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
val function = QuadraticLinearFunction(
    polynomial = polynomial,
    converter = IntoValue.Identity,
    name = "quadratic_linear"
)
val tokens = AutoTokenTable<Flt64>(Quadratic, false)
tokens.add(listOf(x, y))
val value = function.prepare(
    mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0)),
    tokens,
    IntoValue.Identity
)
check(value == Flt64(13.0))
tokens.close()
```

### Rust

Rust 的 [`QuadraticLinearFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_linear.rs) 把 `Quadratic<V>` 表达式显式桥接到结果变量：

```rust
QuadraticLinearFunction::new(id: u64, name: &str, input: Quadratic<V>)
    -> QuadraticLinearFunction<V>
```

结果变量名称为 `name + "_lin_y"`。`calculate_value` 直接求值输入二次式；`prepare` 优先使用传入的结果变量值，否则求值输入。输入没有二次单项式时，机理会生成线性等式；否则生成二次等式。与 Kotlin 实现不同，Rust 对每个输入都创建桥接变量，不会因为输入纯线性而省略它。

```rust
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticLinearFunction;

let polynomial = Quadratic::new(
    vec![
        QuadraticMonomial::new_quadratic(1.0, 0, 1),
        QuadraticMonomial::new_linear(1.0, 0),
    ],
    1.0,
);
let bridge = QuadraticLinearFunction::new(12, "quadratic_linear", polynomial);
assert!(bridge.result_variable().name().contains("quadratic_linear_lin_y"));
```

## evaluate 与 solver 的差异

直接求值和 `prepare` 始终计算原始多项式。solver 只对确实含二次项的输入注册等式，因此辅助变量及其非负域可能限制 solver，但直接对同一多项式得到负值仍然是允许的。

## 边界、tolerance 与 Undefined

输入多项式必须可求值且可表示；符号缺失时返回 `null`。该函数没有 tolerance 或三值未定义状态。二次值为负时直接求值合法，但生成的 `URealVar` 辅助变量无法表示它，除非改变模型/变量域。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticLinearFunction
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
val function = QuadraticLinearFunction(
    polynomial = polynomial,
    converter = IntoValue.Identity,
    name = "quadratic_linear"
)
val tokens = AutoTokenTable<Flt64>(Quadratic, false)
tokens.add(listOf(x, y))
val value = function.prepare(
    mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0)),
    tokens,
    IntoValue.Identity
)
check(value == Flt64(13.0))
tokens.close()
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticLinearFunction;
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
let y = ContinuousVariableItem::create(VariableId::standalone(1), "y");
let mut tokens = VecTokenList::<f64>::new();
let tx = Token::from_generic(x, 0);
tx.set_result(2.0);
tokens.add_token(tx);
let ty = Token::from_generic(y, 1);
ty.set_result(5.0);
tokens.add_token(ty);
let bridge = QuadraticLinearFunction::new(
    13,
    "qlinear",
    Quadratic::new(
        vec![
            QuadraticMonomial::new_quadratic(1.0, 0, 1),
            QuadraticMonomial::new_linear(1.0, 0),
        ],
        1.0,
    ),
);
assert_eq!(bridge.calculate_value(&tokens, false), Some(13.0));
```

:::

- Core 求值：[`QuadraticFunctionGenericEvaluationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionGenericEvaluationTest.kt)
- Core 注册：[`FunctionSymbolGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolGenericRegistrationTest.kt)
- 示例目录（当前没有专门的二次线性文件）：[quadratic_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/quadratic_function)

- Rust 实现与测试：[`quadratic_linear.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_linear.rs) 与 [`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)

## 相关页面

- [二次乘积](./product)
- [二次步进区间](./quadratic-in-step-range)
