# 条件 If-Then

## 契约

`IfThenFunction<V>` 用线性条件门控一个线性多项式。条件为真时，结果等于 `thenPoly`；条件为假时，结果为零。条件落在间隔内时为 `Undefined`，求值结果为 `null`。

条件与 then 表达式都必须是 `LinearPolynomial<V>`。该函数泛型为 `V : RealNumber<V> & NumberField<V>`；不能直接接收二次多项式。

## 定义与三值条件

令 `d` 为传入的条件差值，`q` 为 `thenPoly`，令 `g` 为 `strictBoundary`：

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

## 边界、tolerance 与 Undefined

`classify(values)` 使用共享的 `TruthValue` 分类器。`evaluate()` 在 `True` 时调用 `thenPoly.evaluateWith(values)`，在 `False` 时返回 converter 的零值；在 `Undefined`、缺少条件输入或求值失败时返回 `null`。

`strictBoundary` 默认取 `tolerance`，而 `tolerance` 默认是 `NONZERO_TOLERANCE = 1e-10`。`delta` 默认取 `strictBoundary`。仅支持 `GT`、`GE`、`LT` 与 `LE`。

注册约束需要条件和 `thenPoly` 都有有限闭区间。可以通过 `conditionBounds`/`bounds` 与 `thenBounds` 传入，也可以从相应多项式推断。旧的 `bigM` 参数不能替代任一范围。

## 当前 API

### Kotlin

源码：[`IfThen.kt`（`IfThenFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/IfThen.kt)

```kotlin
IfThenFunction(
    condition: LinearPolynomial<V>,
    thenPoly: LinearPolynomial<V>,
    converter: IntoValue<V>,
    bigM: V? = null,
    tolerance: V? = null,
    strictBoundary: V? = null,
    name: String = "ifthen",
    displayName: String? = null,
    relation: Comparison = Comparison.GT,
    conditionBounds: ConditionBounds<V>? = null,
    thenBounds: ConditionBounds<V>? = null,
    bounds: ConditionBounds<V>? = null,
    delta: V? = null
)
```

伴随对象的 `invoke` 接受相同参数。`IfThenFunction.from` 接受 `LinearConstraintInput<V>`；它会提取扁平化的条件，并且默认可以使用常量一 `thenPoly`，返回 `LinearFunctionSymbolAdapter<V>`。

### Rust

源码：[`if_then.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/if_then.rs)

Rust 保留了两套不同 API。`ConditionalThenFunction` 最接近 Kotlin 的条件值门控：它接收 `ConditionalIfFunction`、`Linear<V>` then 表达式以及显式有限的 then 范围。旧版 `IfThenFunction` 则建模两个 `LinearInequality<V>` 之间的蕴含并返回二值蕴含结果，不是 Kotlin `thenPoly` 输出的一一对应替代。`ConditionalThenFunction` 的未定义条件保持为 `None`，不会静默当作假分支。

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
IfThenFunction::new(
    id: u64,
    name: &str,
    premise: LinearInequality<V>,
    consequence: LinearInequality<V>,
    big_m: V,
) -> Self
```

## 求解器数学模型

对于名称 `name`，实现会创建二进制条件指示变量 `name_ind` 和实数结果变量 `name_y`。二者都放入 `helperVariables`；`resultPolynomial` 是 `name_y` 的单位系数多项式。

加入辅助变量后，`registerConstraints` 会在条件范围上规范化关系。令规范化条件 $c\in[L_c,U_c]$、真阈值为 $T$、假阈值为 $F$，两条条件约束为

$$
c+(L_c-T)i\ge L_c,
\qquad
c+(F-U_c)i\le F.
$$

再令 then 表达式满足 $L\le q\le U$，四条结果门控约束为：

$$
y\le U\,i,
\qquad
y\ge L\,i,
\qquad
y-q\le-L(1-i),
\qquad
y-q\ge-U(1-i).
$$

这些约束共同保证 $i=0\Rightarrow y=0$、$i=1\Rightarrow y=q$。如果条件范围已证明某个分支必然成立，Kotlin 会折叠指标与结果。Rust 的条件对应物使用相同的指标加门控结构。

## `evaluate()` 与求解器模型的差异

求值器可以返回 then 值、零或 `null`。求解器模型要求两个范围都有限，并且没有条件处于间隔内时的赋值；这种值可能使模型不可行。Undefined 条件不会被静默当作假分支。

## 当前最小示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ConditionBounds
import fuookami.ospf.kotlin.core.symbol.function.IfThenFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.inequality.Comparison
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial

fun main() {
    val x = RealVar("x")
    val condition = LinearPolynomial(
        monomials = listOf(LinearMonomial(Flt64.one, x)),
        constant = Flt64(-2.0)
    )
    val thenPoly = LinearPolynomial<Flt64>(emptyList(), Flt64(5.0))
    val function = IfThenFunction(
        condition = condition,
        thenPoly = thenPoly,
        converter = IntoValue.Identity,
        relation = Comparison.GT,
        strictBoundary = Flt64(0.1),
        conditionBounds = ConditionBounds(Flt64(-2.0), Flt64(3.0)),
        thenBounds = ConditionBounds(Flt64(5.0), Flt64(5.0)),
        name = "ifthen"
    )

    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(3.0))) == Flt64(5.0))
    check(function.evaluate(mapOf<Symbol, Flt64>(x to Flt64(1.0))) == Flt64.zero)
}
```

```rust [Rust]
use ospf_rust_core::symbol::flatten::Linear;
use ospf_rust_core::symbol::function::{
    ConditionBounds, ConditionRelation, ConditionalThenFunction,
};

let function = ConditionalThenFunction::from_parts_with_bounds(
    Linear::new(vec![], 1.0),
    ConditionRelation::GreaterEqual,
    0.1,
    ConditionBounds {
        lower: -1.0,
        upper: 2.0,
    },
    Linear::new(vec![], 5.0),
    ConditionBounds {
        lower: 5.0,
        upper: 5.0,
    },
)
.expect("valid conditional-then function");
let value = function.evaluate(&1.0, &5.0).expect("classifiable condition");
assert_eq!(value, Some(5.0));
```

:::

## 源码与 core 测试

- [实现：`IfThen.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/IfThen.kt)
- [core 条件注册测试：`FunctionSymbolConditionalGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolConditionalGenericRegistrationTest.kt)
- [core 条件回归测试：`ConditionalFunctionRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/ConditionalFunctionRegressionTest.kt)
- [core 约束输入工厂测试：`FunctionSymbolConstraintInputFactoryTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolConstraintInputFactoryTest.kt)
- [完整示例：`ConditionalFunctionSolveRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function/ConditionalFunctionSolveRegressionTest.kt)
- [Rust 实现与单元测试：`if_then.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/if_then.rs)
- [Rust 条件回归：`conditional_function_solver_regression.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/conditional_function_solver_regression.rs)
- [Rust 不等式蕴含 parity：`gurobi_linear_function_kotlin_parity.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_linear_function_kotlin_parity.rs)

## 相关页面

- [条件 IF](/zh-cn/guide/linear-functional/if)
- [条件区间](/zh-cn/guide/linear-functional/if-in)
- [One-of 约束](/zh-cn/guide/linear-functional/one-of)
