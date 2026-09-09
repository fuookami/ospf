# 条件 IF

## 契约

`IfFunction<V>` 将线性条件与零比较，并暴露二值结果。对于条件差值 $d$，关系为真时结果为 `1`，关系为假时结果为 `0`。支持的关系是 `GT`、`GE`、`LT` 和 `LE`；共享分类器会拒绝 `EQ` 和 `NE`。

条件是 `LinearPolynomial<V>`，不是预先构造的布尔表达式。`IfFunction` 对 `V : RealNumber<V> & NumberField<V>` 泛型化。

## 定义与真值表

条件多项式解释为 $d = \mathrm{lhs}-\mathrm{rhs}$，或直接作为传入的差值多项式。令 $g$ 为 `strictBoundary`：

| Relation | True branch | False branch | Undefined gap |
| --- | --- | --- | --- |
| `GT` | $d\ge g$ | $d\le0$ | $0<d<g$ |
| `GE` | $d\ge0$ | $d\le-g$ | $-g<d<0$ |
| `LT` | $d\le-g$ | $d\ge0$ | $-g<d<0$ |
| `LE` | $d\le0$ | $d\ge g$ | $0<d<g$ |

结果为：

$$
y = \begin{cases}
1, & \text{true branch} \\
0, & \text{false branch} \\
\text{undefined}, & \text{inside the gap}
\end{cases}
$$

## 边界、tolerance 与 Undefined

`classify` 返回 `TruthValue.True`、`TruthValue.False` 或 `TruthValue.Undefined`。`evaluate()` 将前两者映射为 `1` 和 `0`，把 `Undefined`、缺少输入或分类失败映射为 `null`。

`strictBoundary` 默认取兼容参数 `tolerance`，而 `tolerance` 默认取 `NONZERO_TOLERANCE = 1e-10`。`delta` 默认取 `strictBoundary`，用于把离散条件规范化为约束。两者都必须有限、可表示且为正。

注册要求有限闭区间 `ConditionBounds(lower, upper)`：可以通过 `conditionBounds`/`bounds` 显式提供，也可以从 `condition.finiteBounds(converter)` 推导。旧版 `bigM` 参数会做兼容性校验，但不能替代这些范围。如果给定范围只覆盖一个分支，实现会把指示量和结果折叠为固定值。

## 当前 API

### Kotlin

源码：[`If.kt`（`IfFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/If.kt)

```kotlin
IfFunction(
    condition: LinearPolynomial<V>,
    converter: IntoValue<V>,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    name: String = "if",
    displayName: String? = null,
    relation: Comparison = Comparison.GT,
    bounds: ConditionBounds<V>? = null,
    conditionBounds: ConditionBounds<V>? = null,
    delta: V? = null
)
```

伴生 `invoke` 具有相同的条件参数。`IfFunction.from` 接受 `LinearConstraintInput<V>`，提取其展平后的差值多项式，保留比较关系，并返回 `LinearFunctionSymbolAdapter<V>`。

### Rust

源码：[`if_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/if_function.rs)

Rust 有同名辅助函数，但不是 Kotlin 关系分类器的一一对应替代。Rust `IfFunction` 是三元值选择器：测试 `condition` 是否非零，然后返回 `then_expr` 或 `else_expr`；它没有 `Comparison`、`strictBoundary`、`ConditionBounds` 或 `Undefined` 间隔。选择器使用内部的 `16 * f64::EPSILON` 零值判定。若要实现 Kotlin 风格的 `0/1` 关系指示器，应使用带 `ConditionRelation`、正严格边界和有限 `ConditionBounds` 的 `ConditionalIndicatorFunction::new`，再用其结果变量组合所需表达式。

```rust
IfFunction::new(
    id: u64,
    name: &str,
    condition: Linear<V>,
    then_expr: Linear<V>,
    else_expr: Linear<V>,
) -> Self
IfFunction::named(
    name: impl AsRef<str>,
    condition: Linear<V>,
    then_expr: Linear<V>,
    else_expr: Linear<V>,
) -> Self
IfFunction::condition_indicator_variable(&self) -> &BinaryVariableItem
IfFunction::result_variable(&self) -> &ContinuousVariableItem
```

## 辅助变量与注册模型

对于 `name`，实现创建 `name_if` 作为结果变量，创建 `name_if_nz` 作为条件指示量。两者都在 `helperVariables` 中；`resultPolynomial` 是结果变量的单位系数多项式。

`registerAuxiliaryTokens` 校验条件、边界、`strictBoundary` 和 `delta`，然后添加两个变量。`registerConstraints` 在有限范围上规范化声明的关系，必要时折叠单一分支，否则添加两条范围驱动的指示不等式以及 `resultVar = indicatorVar` 等式。实现没有统一的 `bin(max(x) <= rhs)` 越界规则。

## `evaluate()` 与求解器模型的差异

直接求值器分类一个给定值；值在间隔内时可能返回 `null`。求解器必须表示声明的整个范围，因此间隔内的值没有二值分支，可能使模型不可行。旧页面对所有越界情况使用输入最大值的公式，不是当前实现。

## 当前最小示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ConditionBounds
import fuookami.ospf.kotlin.core.symbol.function.IfFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

fun main() {
    val x = RealVar("x")
    val condition = LinearPolynomial(
        monomials = listOf(LinearMonomial(Flt64.one, x)),
        constant = Flt64(-2.0)
    )
    val function = IfFunction(
        condition = condition,
        converter = IntoValue.Identity,
        relation = Comparison.GT,
        strictBoundary = Flt64(0.1),
        conditionBounds = ConditionBounds(Flt64(-2.0), Flt64(3.0)),
        name = "if"
    )

    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(3.0))) == Flt64.one)
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(1.0))) == Flt64.zero)
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::IfFunction;
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::token::VecTokenList;

let function = IfFunction::named(
    "if",
    Linear::new(vec![], 1.0),
    Linear::new(vec![], 7.0),
    Linear::new(vec![], 0.0),
);
let value = <IfFunction as FunctionSymbol>::calculate_value(
    &function,
    &VecTokenList::<f64>::new(),
    false,
);
assert_eq!(value, Some(7.0));
```

:::

## Source and core tests

- [Implementation: `If.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/If.kt)
- [Core conditional regression test: `ConditionalFunctionRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/ConditionalFunctionRegressionTest.kt)
- [Core generic registration test: `FunctionSymbolGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolGenericRegistrationTest.kt)
- [Complete example: `ConditionalFunctionSolveTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/ConditionalFunctionSolveTest.kt)
- [Rust 实现与单元测试：`if_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/if_function.rs)
- [Rust 范围驱动条件回归：`conditional_function_solver_regression.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/conditional_function_solver_regression.rs)

## 相关页面

- [区间条件](/guide/linear-functional/if-in)
- [If-Then](/guide/linear-functional/if-then)
- [二值化](/guide/linear-functional/bin)
- [选一约束](/guide/linear-functional/one-of)
