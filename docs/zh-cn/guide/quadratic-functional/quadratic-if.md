# 二次条件 IF

`QuadraticIfFunction` 是线性条件 IF 的二次输入版本。条件是有界的 `QuadraticPolynomial<V>`，结果是一个二值变量，按比较关系选择真分支或假分支。该符号通过共享的 `QuadraticFunctionSymbol<V>` 组合基类封装线性 `IfFunction`：条件先通过精确等式绑定到桥接变量，线性指示约束再作用于桥接后的仿射条件。

设条件多项式的值为 $d=p(x)$，严格边界为 $g$，语义与线性版本一致：

| 关系 | 真分支 | 假分支 | Undefined 间隔 |
| --- | --- | --- | --- |
| `GT` | $d\ge g$ | $d\le0$ | $0<d<g$ |
| `GE` | $d\ge0$ | $d\le-g$ | $-g<d<0$ |
| `LT` | $d\le-g$ | $d\ge0$ | $-g<d<0$ |
| `LE` | $d\le0$ | $d\ge g$ | $0<d<g$ |

$$
y = \begin{cases}
1, & \text{true branch} \\
0, & \text{false branch} \\
\text{undefined}, & \text{inside the gap}
\end{cases}
$$

## 契约

- 输入：`condition: QuadraticPolynomial<V>`（单个输入）。
- 含二次项的条件绑定到一个桥接变量 `${name}_input_0`，并注册一条精确二次等式；仿射条件直接透传，不引入桥接变量。
- 桥接变量的范围收紧到条件的有限范围；不可被浮点精确表示的边界只会放宽到下一个可表示的 solver 值（`Math.nextUp`/`nextDown`）。
- 注册时校验每个输入都有有限且未扩大的范围；扩大已捕获的边界会被拒绝，收紧是安全的。
- `helperVariables` = 桥接变量加上 `IfFunction` 的辅助变量（`${name}_if` 与 `${name}_if_nz`，均为二值变量）。
- 结果 `polynomial` 是 `${name}_if` 的单位系数多项式提升为二次多项式后的结果。
- 语义继承线性条件 IF：对 `GT`/`GE`/`LT`/`LE` 的关系指示（`EQ` 和 `NE` 被拒绝）、`strictBoundary` 默认取 `NONZERO_TOLERANCE = 1e-10`、`delta` 默认取 `strictBoundary`，以及三值未定义间隔。
- 泛型值要求 `V : RealNumber<V>, V : NumberField<V>`，并配合 `IntoValue<V>` 转换器。

## 求解器数学模型

注册时先校验已捕获的输入范围，然后为二次条件提交一条精确二次等式：

$$
p(x)-\mathrm{bridge}_0=0,
$$

命名为 `${name}_input_0`，桥接变量的范围收紧到条件的有限范围。线性 `IfFunction` 构造在桥接后的仿射条件之上，其范围驱动的指示约束——规范化条件 $q$、真阈值 $T$、假阈值 $F$、有限范围 $L\le q\le U$、二值指示量 $a$、结果 $y$ = `${name}_if`——为：

$$
q+(L-T)a\ge L,
\qquad
q+(F-U)a\le F,
\qquad
y-a=0
$$

这些约束被提升为同一模型上的二次约束。阈值的推导、声明范围已证明某一分支时的折叠行为以及完整的约束集合，参见[条件 IF](../linear-functional/if)。由于桥接等式是二次的，组合后的模型通常是非凸 MIQCP，需要支持非凸二次约束的求解器。

## 当前 API

### Kotlin

源码：[`QuadraticIf.kt`（`QuadraticIfFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticIf.kt)

```kotlin
QuadraticIfFunction(
    condition: QuadraticPolynomial<V>,
    relation: Comparison = Comparison.GT,
    strictBoundary: V? = null,
    delta: V? = null,
    converter: IntoValue<V>,
    name: String = "quadratic_if",
    displayName: String? = null
)
```

该类继承 `QuadraticFunctionSymbol<V>`，在条件完成绑定后委托给 `IfFunction(condition = inputs[0], ...)`。

### Rust

Rust 现已在 `quadratic_function.rs` 中提供同名包装器：[`QuadraticIfFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)。它先把二次条件桥接为线性表达式——每个真正的二次条件对应一个桥接变量和一条精确二次等式，仿射条件直接透传——再包装下文描述的同一构建块，保留其三值间隔语义与显式范围的 Big-M 策略。

```rust
QuadraticIfFunction::new(
    id: u64,
    name: &str,
    condition: Quadratic<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    condition_bounds: ConditionBounds<V>,
) -> Result<QuadraticIfFunction<V>>
QuadraticIfFunction::with_declared_dependencies(
    self,
    dependency_ids: Vec<u64>,
) -> Self
```

桥接变量名为 `{name}_bridge`；`result_variable()` 返回内部指示器的二值结果变量，`relation()`、`strict_boundary()` 与 `condition_bounds()` 暴露保存的配置。构造阶段即预检范围、边界与条件的有限性。机制 Big-M 只来自显式的 `condition_bounds`——从不读取 token 边界；直接求值对原始二次条件做三值分类：真分支为 `1`，假分支为 `0`，间隔内为 `None`（设置 `zero_if_none` 时折叠为 `0`）。

在内部，包装器先把二次条件桥接为线性表达式（使用 `QuadraticLinearFunction`，它注册 $p(x)-\mathrm{bridge}=0$），再对桥接后的线性条件创建范围驱动的关系指示器：

源码：[`conditional_indicator.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/conditional_indicator.rs)

```rust
ConditionalIndicatorFunction::new(
    id: u64,
    name: &str,
    condition: Linear<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    bounds: ConditionBounds<V>,
) -> Result<ConditionalIndicatorFunction<V>>
ConditionalIndicatorFunction::named(
    name: impl AsRef<str>,
    condition: Linear<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    bounds: ConditionBounds<V>,
) -> Result<Self>
```

`ConditionRelation` 提供 `Greater`、`GreaterEqual`、`Less` 与 `LessEqual`；构造阶段即预检范围、边界和多项式的有限性。

## evaluate 与 solver 的差异

直接求值器解析原始输入，把计算出的输入值写入桥接变量的槽位，然后委托给线性 `IfFunction` 的求值。间隔与边界间隔语义与线性页面完全一致：真分支上的值映射为 `1`，假分支上的值映射为 `0`，间隔内的值映射为 `null`。Rust 的 `QuadraticIfFunction` 跳过桥接槽位的写回，直接对原始二次条件求值，并按同样的三值语义分类（间隔内为 `None`，设置 `zero_if_none` 时为 `0`）。

注意求值入口：这些类上不存在单参数映射的 `evaluate(values)` 重载。可用重载是 `evaluate(values, tokenTable, converter, zeroIfNone)`，例如 `f.evaluate(mapOf(x to Flt64(3.0)), null, IntoValue.Identity, false)`。`prepare(values, tokenTable, converter)` 以 `zeroIfNone = false` 委托到同一路径。

求解器必须表示声明的整个条件范围，因此间隔内的值没有二值分支，可能使模型不可行。求值器只分类一个给定值，可以返回 `null`；它不需要范围，而注册需要。

## 最小示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticIfFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial

val x = RealVar("x").also {
    it.range.geq(Flt64(-2.0))
    it.range.leq(Flt64(2.0))
}
val condition = QuadraticPolynomial(
    monomials = listOf(QuadraticMonomial.quadratic(Flt64.one, x, x)),
    constant = Flt64(-1.0)
)
val function = QuadraticIfFunction(
    condition = condition,
    strictBoundary = Flt64(0.5),
    converter = IntoValue.Identity
)

fun value(values: Map<Symbol, Flt64>): Flt64? =
    function.evaluate(
        values = values,
        tokenTable = null,
        converter = IntoValue.Identity,
        zeroIfNone = false
    )

check(value(mapOf(x to Flt64.zero)) == Flt64.zero)   // x^2 - 1 = -1, false branch
check(value(mapOf(x to Flt64(2.0))) == Flt64.one)    // x^2 - 1 = 3, true branch
check(value(mapOf(x to Flt64(1.1))) == null)         // x^2 - 1 = 0.21, inside (0, 0.5)
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::{ConditionBounds, ConditionRelation, QuadraticIfFunction};
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

// Condition x^2 with x in [0, 2] => condition range [0, 4], gap 0.5
let condition = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
let qif = QuadraticIfFunction::new(
    1,
    "qif",
    condition,
    ConditionRelation::Greater,
    0.5,
    ConditionBounds { lower: 0.0, upper: 4.0 },
)
.expect("valid quadratic condition");

let tokens_for = |value: f64| {
    let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
    let mut tokens = VecTokenList::<f64>::new();
    let tx = Token::from_generic(x, 0);
    tx.set_result(value);
    tokens.add_token(tx);
    tokens
};

assert_eq!(
    <QuadraticIfFunction as FunctionSymbol>::calculate_value(&qif, &tokens_for(2.0), false),
    Some(1.0) // x^2 = 4 >= 0.5, true branch
);
assert_eq!(
    <QuadraticIfFunction as FunctionSymbol>::calculate_value(&qif, &tokens_for(0.0), false),
    Some(0.0) // x^2 = 0 <= 0, false branch
);
assert_eq!(
    <QuadraticIfFunction as FunctionSymbol>::calculate_value(&qif, &tokens_for(0.5), false),
    None // x^2 = 0.25, inside (0, 0.5)
);
```

:::

## 测试与参考

- Kotlin 实现：[`QuadraticIf.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticIf.kt)
- Kotlin 组合、间隔语义与机制模型测试（覆盖 `QuadraticIfFunction`、`QuadraticIfInFunction` 和 `QuadraticIfThenFunction`）：[`QuadraticFunctionCompositionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionCompositionTest.kt)
- 记录二次组合契约的函数符号 README：[`function/README.md`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/README.md)
- Rust 构建块：[`conditional_indicator.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/conditional_indicator.rs)
- Rust 条件回归测试（覆盖线性表达式上的 `ConditionalIndicatorFunction` 与 `ConditionalIfFunction`）：[`conditional_function_solver_regression.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/conditional_function_solver_regression.rs)
- Rust 包装器实现与文件内回归测试（`quadratic_if_classifies_three_valued_condition` 与 `quadratic_if_registers_indicator_rows_over_the_bridge_column`）：[`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)
- Rust 专项契约测试：[`function_symbol_quadratic_if.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_if.rs)；端到端求解覆盖：[`gurobi_quadratic_model_integration.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_quadratic_model_integration.rs)（`gurobi_solves_quadratic_if_with_non_linear_input`）。

## 相关页面

- [条件 IF](../linear-functional/if)
- [二次条件区间](./quadratic-if-in)
- [二次条件 If-Then](./quadratic-if-then)
- [二次线性](./quadratic-linear)
