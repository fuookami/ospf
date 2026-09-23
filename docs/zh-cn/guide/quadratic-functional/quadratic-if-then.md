# 二次条件 If-Then

`QuadraticIfThenFunction` 是线性 If-Then 的二次输入版本。条件与被门控的值都是有界的 `QuadraticPolynomial<V>`：条件成立时结果等于 then 多项式，条件不成立时结果为零。该符号通过共享的 `QuadraticFunctionSymbol<V>` 组合基类封装线性 `IfThenFunction`：两个输入先分别通过精确等式绑定到桥接变量，线性条件值约束再作用于桥接后的仿射输入。

设条件多项式的值为 $d=p(x)$、then 多项式为 $q=r(x)$、严格边界为 $g$，条件语义与线性版本一致：

| 关系 | 真分支 | 假分支 | Undefined 间隔 |
| --- | --- | --- | --- |
| `GT` | $d\ge g$ | $d\le0$ | $0<d<g$ |
| `GE` | $d\ge0$ | $d\le-g$ | $-g<d<0$ |
| `LT` | $d\le-g$ | $d\ge0$ | $-g<d<0$ |
| `LE` | $d\le0$ | $d\ge g$ | $0<d<g$ |

门控结果为：

$$
y = \begin{cases}
q, & \text{true branch} \\
0, & \text{false branch} \\
\text{undefined}, & \text{inside the gap}
\end{cases}
$$

## 契约

- 输入：`condition: QuadraticPolynomial<V>` 与 `thenPoly: QuadraticPolynomial<V>`（两个输入；含二次项时各自拥有独立的桥接变量）。
- 每个二次输入绑定到桥接变量 `${name}_input_$index`，并注册一条精确二次等式；仿射输入直接透传，不引入桥接变量。
- 桥接变量的范围收紧到各输入的有限范围；不可被浮点精确表示的边界只会放宽到下一个可表示的 solver 值（`Math.nextUp`/`nextDown`）。
- 注册时校验每个输入都有有限且未扩大的范围；扩大已捕获的边界会被拒绝，收紧是安全的。
- `helperVariables` = 桥接变量加上 `IfThenFunction` 的辅助变量（`${name}_ind`，二值条件指示量；`${name}_y`，实数结果变量，范围为 then 范围与零的并集）。
- 结果 `polynomial` 是 `${name}_y` 的单位系数多项式提升为二次多项式后的结果。
- 语义继承线性 If-Then：假分支为零的条件值、对 `GT`/`GE`/`LT`/`LE` 的关系指示、`strictBoundary` 默认取 `NONZERO_TOLERANCE = 1e-10`、`delta` 默认取 `strictBoundary`，以及三值未定义间隔。
- 泛型值要求 `V : RealNumber<V>, V : NumberField<V>`，并配合 `IntoValue<V>` 转换器。

## 求解器数学模型

注册时先校验已捕获的输入范围，然后为每个二次输入提交一条精确二次等式：

$$
p(x)-\mathrm{bridge}_0=0,
\qquad
r(x)-\mathrm{bridge}_1=0,
$$

分别命名为 `${name}_input_0` 与 `${name}_input_1`，桥接变量的范围各自收紧到对应输入的有限范围。线性 `IfThenFunction` 构造在桥接后的仿射条件 $c=\mathrm{bridge}_0$ 与 then 多项式 $q=\mathrm{bridge}_1$ 之上。对规范化条件 $c\in[L_c,U_c]$、真阈值 $T$、假阈值 $F$、二值指示量 $i$ = `${name}_ind`，提交两条条件约束

$$
c+(L_c-T)i\ge L_c,
\qquad
c+(F-U_c)i\le F
$$

以及，对 then 范围 $L\le q\le U$ 与结果 $y$ = `${name}_y`，四条门控约束

$$
y\le U\,i,
\qquad
y\ge L\,i,
\qquad
y-q\le-L(1-i),
\qquad
y-q\ge-U(1-i).
$$

所有约束都被提升为同一模型上的二次约束。它们共同保证 $i=0\Rightarrow y=0$、$i=1\Rightarrow y=q$；当声明的条件范围已证明某一分支时，指示量与结果折叠为固定值。完整推导参见[If-Then](../linear-functional/if-then)。由于桥接等式是二次的，组合后的模型通常是非凸 MIQCP，需要支持非凸二次约束的求解器。

## 当前 API

### Kotlin

源码：[`QuadraticIfThen.kt`（`QuadraticIfThenFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticIfThen.kt)

```kotlin
QuadraticIfThenFunction(
    condition: QuadraticPolynomial<V>,
    thenPoly: QuadraticPolynomial<V>,
    relation: Comparison = Comparison.GT,
    strictBoundary: V? = null,
    delta: V? = null,
    converter: IntoValue<V>,
    name: String = "quadratic_ifthen",
    displayName: String? = null
)
```

该类继承 `QuadraticFunctionSymbol<V>`，在两个输入完成绑定后委托给 `IfThenFunction(condition = inputs[0], thenPoly = inputs[1], ...)`。

### Rust

Rust 现已在 `quadratic_function.rs` 中提供同名包装器：[`QuadraticIfThenFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)。它先把两个二次输入分别桥接为线性表达式——每个真正的二次输入对应一个桥接变量和一条精确二次等式，仿射输入直接透传——再包装下文描述的同一构建块，保留其三值间隔语义与显式范围的 Big-M 策略。

```rust
QuadraticIfThenFunction::new(
    id: u64,
    name: &str,
    condition: Quadratic<V>,
    then_poly: Quadratic<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    condition_bounds: ConditionBounds<V>,
    then_bounds: ConditionBounds<V>,
) -> Result<QuadraticIfThenFunction<V>>
QuadraticIfThenFunction::with_declared_dependencies(
    self,
    dependency_ids: Vec<u64>,
) -> Self
```

包装器创建两个桥接：`{name}_bridge_condition` 与 `{name}_bridge_then`。其内部封装了 `ConditionalThenFunction::from_parts_with_bounds`：条件成立时结果等于 then 值，条件不成立时结果为 `0`，间隔内为 `None`。两个声明的范围都在构造阶段完成校验；`result_variable()` 返回内部的连续结果变量，`relation()`、`strict_boundary()`、`condition_bounds()` 与 `then_bounds()` 暴露保存的配置。Big-M 只来自显式的 `condition_bounds`——从不读取 token 边界。

在内部，包装器先把二次条件与 then 多项式分别桥接为线性表达式（使用 `QuadraticLinearFunction`，它注册 $p(x)-\mathrm{bridge}=0$），再在桥接后的线性表达式上创建条件值门控：

源码：[`if_then.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/if_then.rs)

```rust
ConditionalThenFunction::from_parts_with_bounds(
    condition: Linear<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    condition_bounds: ConditionBounds<V>,
    then_poly: Linear<V>,
    then_bounds: ConditionBounds<V>,
) -> Result<ConditionalThenFunction<V>>
ConditionalThenFunction::named(
    name: impl AsRef<str>,
    condition: ConditionalIfFunction<V>,
    then_poly: Linear<V>,
    then_bounds: ConditionBounds<V>,
) -> Result<Self>
```

`ConditionalThenFunction` 的未定义条件保持为 `None`，不会静默当作假分支。Rust 旧版 `IfThenFunction` 建模的是两个不等式之间的蕴含并返回二值结果，不是条件值门控的替代。

## evaluate 与 solver 的差异

直接求值器解析原始输入，把计算出的值写入桥接变量的槽位，然后委托给线性 `IfThenFunction` 的求值。间隔与边界间隔语义与线性页面完全一致：条件为真映射为 then 值，条件为假映射为零，条件落在间隔内映射为 `null`。Rust 的 `QuadraticIfThenFunction` 跳过桥接槽位的写回，直接对原始二次条件与 then 表达式求值，语义同样是三值的（间隔内为 `None`，设置 `zero_if_none` 时为 `0`）。

注意求值入口：这些类上不存在单参数映射的 `evaluate(values)` 重载。可用重载是 `evaluate(values, tokenTable, converter, zeroIfNone)`，例如 `f.evaluate(mapOf(x to Flt64(2.0)), null, IntoValue.Identity, false)`。`prepare(values, tokenTable, converter)` 以 `zeroIfNone = false` 委托到同一路径。

求解器模型要求两个输入的范围都有限，并且没有条件处于间隔内时的赋值；这种值可能使模型不可行。未定义条件不会被静默当作假分支。

## 最小示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticIfThenFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial

val x = RealVar("x").also {
    it.range.geq(Flt64(-2.0))
    it.range.leq(Flt64(2.0))
}
val square = QuadraticPolynomial(
    monomials = listOf(QuadraticMonomial.quadratic(Flt64.one, x, x)),
    constant = Flt64(-1.0)
)
val function = QuadraticIfThenFunction(
    condition = square,
    thenPoly = square,
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

check(value(mapOf(x to Flt64.zero)) == Flt64.zero)   // condition -1 <= 0, false branch
check(value(mapOf(x to Flt64(2.0))) == Flt64(3.0))   // condition 3 >= 0.5, then value 3
check(value(mapOf(x to Flt64(1.1))) == null)         // condition 0.21, inside (0, 0.5)
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::{
    ConditionBounds, ConditionRelation, QuadraticIfThenFunction,
};
use ospf_rust_core::token::{MutableTokenList, Token, VecTokenList};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId};

// Condition x^2 - 1 (GT, gap 0.5), then = 2x^2; x in [0, 2]
let condition = Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 0)], -1.0);
let then_poly = Quadratic::new(vec![QuadraticMonomial::new_quadratic(2.0, 0, 0)], 0.0);
let qifthen = QuadraticIfThenFunction::new(
    1,
    "qifthen",
    condition,
    then_poly,
    ConditionRelation::Greater,
    0.5,
    ConditionBounds { lower: -1.0, upper: 3.0 },
    ConditionBounds { lower: 0.0, upper: 8.0 },
)
.expect("valid quadratic if-then");

let tokens_for = |value: f64| {
    let x = ContinuousVariableItem::create(VariableId::standalone(0), "x");
    let mut tokens = VecTokenList::<f64>::new();
    let tx = Token::from_generic(x, 0);
    tx.set_result(value);
    tokens.add_token(tx);
    tokens
};

assert_eq!(
    <QuadraticIfThenFunction as FunctionSymbol>::calculate_value(&qifthen, &tokens_for(2.0), false),
    Some(8.0) // condition 3 >= 0.5, then value 2 * 4
);
assert_eq!(
    <QuadraticIfThenFunction as FunctionSymbol>::calculate_value(&qifthen, &tokens_for(1.0), false),
    Some(0.0) // condition 0 <= 0, zero false branch
);
assert_eq!(
    <QuadraticIfThenFunction as FunctionSymbol>::calculate_value(&qifthen, &tokens_for(1.1), false),
    None // condition 0.21, inside (0, 0.5)
);
```

:::

## 测试与参考

- Kotlin 实现：[`QuadraticIfThen.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticIfThen.kt)
- Kotlin 组合、间隔语义与机制模型测试（覆盖 `QuadraticIfFunction`、`QuadraticIfInFunction` 和 `QuadraticIfThenFunction`）：[`QuadraticFunctionCompositionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionCompositionTest.kt)
- 记录二次组合契约的函数符号 README：[`function/README.md`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/README.md)
- Rust 构建块：[`if_then.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/if_then.rs)
- Rust 条件回归测试（覆盖线性表达式上的 `ConditionalIfFunction`、`ConditionalIndicatorFunction` 与 `ConditionalThenFunction`）：[`conditional_function_solver_regression.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/conditional_function_solver_regression.rs)
- Rust 包装器实现与文件内回归测试（`quadratic_if_then_gates_then_value_by_condition` 与 `quadratic_if_then_registers_rows_over_both_bridge_columns`）：[`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)
- Rust 专项契约测试：[`function_symbol_quadratic_if_then.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_if_then.rs)；端到端求解覆盖：[`gurobi_quadratic_model_integration.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_quadratic_model_integration.rs)（`gurobi_solves_quadratic_if_then_with_non_linear_input`）。

## 相关页面

- [If-Then](../linear-functional/if-then)
- [二次条件 IF](./quadratic-if)
- [二次条件区间](./quadratic-if-in)
- [二次线性](./quadratic-linear)
