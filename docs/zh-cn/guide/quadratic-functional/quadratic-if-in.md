# 二次条件区间

`QuadraticIfInFunction` 是线性条件区间的二次输入版本。输入是有界的 `QuadraticPolynomial<V>`，结果是一个二值变量：输入值落在闭区间 $[\mathrm{lower},\mathrm{upper}]$ 内时为 `1`，安全地位于区间外时为 `0`。该符号通过共享的 `QuadraticFunctionSymbol<V>` 组合基类封装线性 `IfInFunction`：输入先通过精确等式绑定到桥接变量，线性区间约束再作用于桥接后的仿射输入。

针对输入多项式的值 $x=p(t)$ 定义两个差值：

$$
d_\mathrm{lower}=p(t)-\mathrm{lower},
\qquad
d_\mathrm{upper}=\mathrm{upper}-p(t).
$$

两个差值都使用共享的 `GE` 关系分类。给定严格边界 $g$：

| Position of $p(t)$ | Lower side | Upper side | Result |
| --- | --- | --- | --- |
| $p(t)\le\mathrm{lower}-g$ | False | True or Undefined | 0 |
| $\mathrm{lower}-g<p(t)<\mathrm{lower}$ | Undefined | True | Undefined |
| $\mathrm{lower}\le p(t)\le\mathrm{upper}$ | True | True | 1 |
| $\mathrm{upper}<p(t)<\mathrm{upper}+g$ | True | Undefined | Undefined |
| $p(t)\ge\mathrm{upper}+g$ | True or Undefined | False | 0 |

两个端点都属于闭区间比较的真分支。

## 契约

- 输入：`input: QuadraticPolynomial<V>`，以及标量端点 `lower: V` 与 `upper: V`（要求 `lower <= upper`）。
- 含二次项的输入绑定到一个桥接变量 `${name}_input_0`，并注册一条精确二次等式；仿射输入直接透传，不引入桥接变量。
- 桥接变量的范围收紧到输入的有限范围；不可被浮点精确表示的边界只会放宽到下一个可表示的 solver 值（`Math.nextUp`/`nextDown`）。
- 注册时校验每个输入都有有限且未扩大的范围；扩大已捕获的边界会被拒绝，收紧是安全的。
- `helperVariables` = 桥接变量加上 `IfInFunction` 的辅助变量（`${name}_ifin`、`${name}_ge` 与 `${name}_le`，均为二值变量）。
- 结果 `polynomial` 是 `${name}_ifin` 的单位系数多项式提升为二次多项式后的结果。
- 语义继承线性条件区间：通过两个 `GE` 指示量表达闭区间成员关系、`strictBoundary` 默认取 `NONZERO_TOLERANCE = 1e-10`、`delta` 默认取 `strictBoundary`，以及上述三值间隔行为。
- 泛型值要求 `V : RealNumber<V>, V : NumberField<V>`，并配合 `IntoValue<V>` 转换器。

## 求解器数学模型

注册时先校验已捕获的输入范围，然后为二次输入提交一条精确二次等式：

$$
p(t)-\mathrm{bridge}_0=0,
$$

命名为 `${name}_input_0`，桥接变量的范围收紧到输入的有限范围。线性 `IfInFunction` 构造在桥接后的仿射输入之上，形成 $q_l=\mathrm{bridge}_0-\mathrm{lower}$ 与 $q_u=\mathrm{upper}-\mathrm{bridge}_0$。对每一侧 $j\in\{l,u\}$，令 $L_j\le q_j\le U_j$、真阈值为 $T_j$、假阈值为 $F_j$、二值指示量为 $a_j$，实际提交的两侧指示约束为

$$
q_j+(L_j-T_j)a_j\ge L_j,
\qquad
q_j+(F_j-U_j)a_j\le F_j
$$

以及结果 $y$ = `${name}_ifin` 的 AND 约束：

$$
y\ge a_l+a_u-1,
\qquad
y\le a_l,
\qquad
y\le a_u.
$$

所有约束都被提升为同一模型上的二次约束。阈值的推导、某一侧可由声明范围判定时的折叠行为以及完整的约束集合，参见[条件区间](../linear-functional/if-in)。由于桥接等式是二次的，组合后的模型通常是非凸 MIQCP，需要支持非凸二次约束的求解器。

## 当前 API

### Kotlin

源码：[`QuadraticIfIn.kt`（`QuadraticIfInFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticIfIn.kt)

```kotlin
QuadraticIfInFunction(
    input: QuadraticPolynomial<V>,
    lower: V,
    upper: V,
    strictBoundary: V? = null,
    delta: V? = null,
    converter: IntoValue<V>,
    name: String = "quadratic_ifin",
    displayName: String? = null
)
```

该类继承 `QuadraticFunctionSymbol<V>`，在输入完成绑定后委托给 `IfInFunction(x = inputs[0], lower, upper, ...)`。

### Rust

Rust 现已在 `quadratic_function.rs` 中提供同名包装器：[`QuadraticIfInFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)。它先把二次输入桥接为线性表达式——真正的二次输入对应一个桥接变量和一条精确二次等式，仿射输入直接透传——再包装下文描述的同一构建块，保留其三值间隔语义与显式范围的 Big-M 策略。

```rust
QuadraticIfInFunction::new(
    id: u64,
    name: &str,
    input: Quadratic<V>,
    lower: V,
    upper: V,
    strict_boundary: V,
    input_bounds: ConditionBounds<V>,
) -> Result<QuadraticIfInFunction<V>>
QuadraticIfInFunction::with_declared_dependencies(
    self,
    dependency_ids: Vec<u64>,
) -> Self
```

桥接变量名为 `{name}_bridge`；包装器内部封装了一个 `RegisterableIfInRangeFunction`，其两侧分别为 $x-\mathrm{lower}\ge0$ 与 $\mathrm{upper}-x\ge0$（均为 `GreaterEqual`），因此当且仅当 $\mathrm{lower}\le x\le\mathrm{upper}$ 时结果为 `1`。区间有序性（`lower <= upper`）与单变量侧条件的校验在构造阶段完成。`result_variable()` 返回内部的二值结果变量，`lower()`、`upper()`、`strict_boundary()` 与 `input_bounds()` 暴露保存的配置；Big-M 只来自显式的 `input_bounds`——从不读取 token 边界。

在内部，包装器先把二次输入桥接为线性表达式（使用 `QuadraticLinearFunction`，它注册 $p(t)-\mathrm{bridge}=0$），再用两个 `ConditionalIfFunction` 描述闭区间两侧，并注册为一对范围驱动的指示量加 AND 结果：

源码：[`if_in.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/if_in.rs)、[`conditional.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/conditional.rs)

```rust
ConditionalIfFunction::new(
    condition: Linear<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    bounds: ConditionBounds<V>,
) -> Result<ConditionalIfFunction<V>>
IfInRangeFunction::new(
    lower: ConditionalIfFunction<V>,
    upper: ConditionalIfFunction<V>,
) -> Result<IfInRangeFunction<V>>
IfInRangeFunction::registerable(
    self,
    id: u64,
    name: impl AsRef<str>,
) -> Result<RegisterableIfInRangeFunction<V>>
```

区间校验要求两侧使用 `GreaterEqual`、一元变量条件系数符号相反、端点有序且范围有限。`RegisterableIfInRangeFunction` 为每一侧各创建一个 `ConditionalIndicatorFunction`，再用三条线性 AND 约束合并。

## evaluate 与 solver 的差异

直接求值器解析原始输入，把计算出的值写入桥接变量的槽位，然后委托给线性 `IfInFunction` 的分类/求值。三值区间语义与线性页面完全一致：两侧都为真映射为 `1`，任一侧为假映射为 `0`，落在边界带内的值映射为 `null`。Rust 的 `QuadraticIfInFunction` 跳过桥接槽位的写回，直接对原始二次输入的两个区间差值做分类，语义同样是三值的（边界带内为 `None`，设置 `zero_if_none` 时为 `0`）。

注意求值入口：这些类上不存在单参数映射的 `evaluate(values)` 重载。可用重载是 `evaluate(values, tokenTable, converter, zeroIfNone)`，例如 `f.evaluate(mapOf(t to Flt64(1.0)), null, IntoValue.Identity, false)`。`prepare(values, tokenTable, converter)` 以 `zeroIfNone = false` 委托到同一路径。

求解器使用范围驱动的指示约束；如果声明的输入范围穿过不可分类的边界带，生成的模型可能不可行。求值器只分类一个给定值，可以返回 `null`；它不需要范围，而注册需要。

## 最小示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticIfInFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial

val t = RealVar("t").also {
    it.range.geq(Flt64(-2.0))
    it.range.leq(Flt64(2.0))
}
val input = QuadraticPolynomial(
    monomials = listOf(QuadraticMonomial.quadratic(Flt64.one, t, t)),
    constant = Flt64(-1.0)
)
val function = QuadraticIfInFunction(
    input = input,
    lower = Flt64.zero,
    upper = Flt64(2.0),
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

check(value(mapOf(t to Flt64.one)) == Flt64.one)    // t^2 - 1 = 0, inside [0, 2]
check(value(mapOf(t to Flt64(2.0))) == Flt64.zero)  // t^2 - 1 = 3, outside (>= 2.5)
check(value(mapOf(t to Flt64(0.9))) == null)        // t^2 - 1 = -0.19, boundary band
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::{ConditionBounds, QuadraticIfInFunction};
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

// Input x^2 with x in [0, 2] => input range [0, 4], closed interval [1, 4]
let input = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], 0.0);
let qifin = QuadraticIfInFunction::new(
    1,
    "qifin",
    input,
    1.0,
    4.0,
    0.5,
    ConditionBounds { lower: 0.0, upper: 4.0 },
)
.expect("valid quadratic input");

let tokens_for = |value: f64| {
    let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
    let mut tokens = VecTokenList::<f64>::new();
    let tx = Token::from_generic(x, 0);
    tx.set_result(value);
    tokens.add_token(tx);
    tokens
};

assert_eq!(
    <QuadraticIfInFunction as FunctionSymbol>::calculate_value(&qifin, &tokens_for(1.5), false),
    Some(1.0) // x^2 = 2.25, inside [1, 4]
);
assert_eq!(
    <QuadraticIfInFunction as FunctionSymbol>::calculate_value(&qifin, &tokens_for(2.0), false),
    Some(1.0) // x^2 = 4, the closed upper endpoint
);
assert_eq!(
    <QuadraticIfInFunction as FunctionSymbol>::calculate_value(&qifin, &tokens_for(0.5), false),
    Some(0.0) // x^2 = 0.25 < 1, outside
);
```

:::

## 测试与参考

- Kotlin 实现：[`QuadraticIfIn.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticIfIn.kt)
- Kotlin 组合、间隔语义与机制模型测试（覆盖 `QuadraticIfFunction`、`QuadraticIfInFunction` 和 `QuadraticIfThenFunction`）：[`QuadraticFunctionCompositionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionCompositionTest.kt)
- 记录二次组合契约的函数符号 README：[`function/README.md`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/README.md)
- Rust 构建块：[`if_in.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/if_in.rs) 与 [`conditional.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/conditional.rs)
- Rust 条件回归测试（覆盖区间组合每侧的构建块 `ConditionalIfFunction` 与 `ConditionalIndicatorFunction`）：[`conditional_function_solver_regression.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/conditional_function_solver_regression.rs)
- Rust 包装器实现与文件内回归测试（`quadratic_if_in_classifies_closed_interval` 与 `quadratic_if_in_registers_interval_rows_over_the_bridge_column`）：[`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)
- Rust 专项契约测试：[`function_symbol_quadratic_if_in.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_if_in.rs)；端到端求解覆盖：[`gurobi_quadratic_model_integration.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_quadratic_model_integration.rs)（`gurobi_solves_quadratic_if_in_with_non_linear_input`）。

## 相关页面

- [条件区间](../linear-functional/if-in)
- [二次条件 IF](./quadratic-if)
- [二次条件 If-Then](./quadratic-if-then)
- [二次线性](./quadratic-linear)
