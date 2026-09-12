# Sigmoid

尽管名称如此，`SigmoidFunction` 实际是二值阶跃/条件指标，不是连续 logistic 函数 $1/(1+e^{-x})$。

> [!WARNING]
> 当前实现使用共享的三值离散条件判定器，并在边界间隔返回 `null`；它不会近似平滑的 sigmoid 曲线。

## 契约

- 输入：条件 `LinearPolynomial<V>`，使用 `relation`（默认 `Comparison.GT`）与零比较。
- 输出：包含二值指标的 `resultPolynomial`，指标名称是在 `name` 后追加 `_sig_ind`。
- true/false 值为 one/zero；未定义间隔或输入缺失时求值为 `null`。
- `strictBoundary` 与 `delta` 定义分支间隔和离散转换；未提供 strictBoundary 时使用 `tolerance`。
- solver 注册要求条件范围有限且有序。旧的 `bigM` 不能替代该范围。

## 定义与数学模型

对于默认 `GT` 关系及间隔 $g=\text{strictBoundary}$：

$$
y=\begin{cases}
1,&condition\ge g,\\
0,&condition\le0,\\
\text{undefined},&0<condition<g.
\end{cases}
$$

`LT` 使用反向不等式，`GE` 和 `LE` 使用相应的同一矩阵。

## 求解器数学模型

令规范化条件 $q\in[L,U]$、真阈值为 $T$、假阈值为 $F$，且 $y\in\{0,1\}$。Kotlin 实际传入

$$
q+(L-T)y\ge L,
\qquad
q+(F-U)y\le F.
$$

非恒定情况下只有这两条关系约束：$y=1\Rightarrow q\ge T$，$y=0\Rightarrow q\le F$。Rust `SigmoidStepFunction` 使用同一关系指标模型。Rust `SigmoidFunction::new` 则注册采样的 logistic 分段线性模型，不能解释为这两条约束。

## 当前 API

### Kotlin

源码：[`Sigmoid.kt`（`SigmoidFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/Sigmoid.kt#L45-L300)

公开工厂和构造器使用相同的条件、边界、关系、范围和命名参数；有限 solver 范围使用 `conditionBounds`（或 `bounds` 别名）。

```kotlin
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ConditionBounds
import fuookami.ospf.kotlin.core.symbol.function.SigmoidFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val condition = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, x)), Flt64.zero
)
val sigmoid = SigmoidFunction(
    condition = condition,
    converter = IntoValue.Identity,
    strictBoundary = Flt64(0.1),
    conditionBounds = ConditionBounds(Flt64(-10.0), Flt64(10.0)),
    name = "sigmoid"
)
val value = sigmoid.evaluate(mapOf<Symbol, Flt64>(x to Flt64.one))
check(value == Flt64.one)
```

### Rust

本页 Kotlin 的二值关系阶跃语义对应 Rust 的 [`SigmoidStepFunction`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/sigmoid.rs)，而不是 Rust 中同名的连续分段线性符号：

```rust
SigmoidStepFunction::from_parts(
    condition: Linear<V>,
    relation: ConditionRelation,
    strict_boundary: V,
    bounds: ConditionBounds<V>,
) -> Result<SigmoidStepFunction<V>>

SigmoidFunction::new(
    id: u64,
    name: &str,
    input: Linear<V>,
) -> SigmoidFunction<V>
```

`SigmoidStepFunction` 是最接近的 Rust API：它对关系进行 `True`/`False`/`Undefined` 三值判定，并暴露二值 `result_variable()`。也可以通过 `SigmoidFunction::step`/`relation` 以及别名 `SigmoidRelationFunction`、`ConditionalSigmoidFunction` 使用。Rust 的 `SigmoidFunction::new` 则构造采样的连续 logistic 分段线性函数；其直接求值为 $1/(1+e^{-x})$，因此不是 Kotlin 阶跃指标的一一对应实现。

## evaluate 与 solver 的差异

直接 `evaluate` 调用 `classify`，将 True/False/Undefined 映射为 one/zero/`null`。solver 注册执行预检、规范化条件、折叠常量、注册指标并添加共享关系约束。直接分支为 false 不会绕过 solver 阶段的范围校验。

## 边界、tolerance 与 Undefined

条件值缺失或非有限时判定失败，`evaluate` 返回 `null`。间隔随关系变化；默认 GT 为 $(0,g)$。非正或非有限 `strictBoundary`/`delta`、逆序或 sentinel 范围以及无效 Big-M 会在注册时返回失败 Result。

## 示例与测试

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.ConditionBounds
import fuookami.ospf.kotlin.core.symbol.function.SigmoidFunction
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.LinearMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.LinearPolynomial
import fuookami.ospf.kotlin.core.variable.RealVar

val x = RealVar("x")
val condition = LinearPolynomial(
    listOf(LinearMonomial(Flt64.one, x)), Flt64.zero
)
val sigmoid = SigmoidFunction(
    condition = condition,
    converter = IntoValue.Identity,
    strictBoundary = Flt64(0.1),
    conditionBounds = ConditionBounds(Flt64(-10.0), Flt64(10.0)),
    name = "sigmoid"
)
val value = sigmoid.evaluate(mapOf<Symbol, Flt64>(x to Flt64.one))
check(value == Flt64.one)
```

```rust [Rust]
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{
    ConditionBounds, ConditionRelation, SigmoidFunction, SigmoidStepFunction,
};

let condition = Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0);
let step = SigmoidStepFunction::from_parts(
    condition.clone(),
    ConditionRelation::Greater,
    0.1_f64,
    ConditionBounds { lower: -10.0, upper: 10.0 },
)
.unwrap();
assert_eq!(step.evaluate(&1.0).unwrap(), Some(1.0));

let smooth = SigmoidFunction::new(2, "sigmoid", condition);
let _smooth_result = smooth.result_variable();
```

:::

- Core 回归测试：[`ConditionalFunctionRegressionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/ConditionalFunctionRegressionTest.kt)
- Core 注册测试：[`FunctionSymbolConditionalGenericRegistrationTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/FunctionSymbolConditionalGenericRegistrationTest.kt)
- 示例目录（当前没有专门的 sigmoid 文件）：[linear_function](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/test/fuookami/ospf/kotlin/example/linear_function)

Rust 源码：[`sigmoid.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/sigmoid.rs)、[`conditional.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/conditional.rs) 和 [`conditional_function_solver_regression.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/conditional_function_solver_regression.rs)。

## 相关页面

- [蕴含](./imply)
- [条件 IF](./if)
- [条件 If-Then](./if-then)
