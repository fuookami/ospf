# 二次最大值

`QuadraticMaxFunction` 计算一组二次多项式的最大值：

$$
y=\max(p_1,p_2,\ldots,p_n).
$$

它与 `QuadraticMinFunction` 共享同一组合架构：每个含二次项的输入都通过一条精确二次等式绑定到桥接变量，然后对绑定后的输入应用线性 `MaxFunction` 选择模型。

## 契约

- 输入：`polynomials: List<QuadraticPolynomial<V>>`；注册时拒绝空列表，且每个候选都必须具有可推导的有限范围。
- 输出/辅助变量：每个真正的二次输入都会得到名为 `${name}_input_$index` 的桥接 `RealVar`；结果为内部函数的实数变量 `${name}_max`，并通过 `polynomial` 暴露。
- 直接求值返回最大值；符号缺失或候选列表为空时返回 `null`。
- Kotlin 始终注册精确选择模型；Rust 暴露 `exact` 标志，`exact = false` 只注册 $y\ge p_i$ 约束。
- 泛型值要求 `V : RealNumber<V>, V : NumberField<V>`，并配合 `IntoValue<V>` 转换器。

> [!WARNING]
> 注册后的公式可能是非凸 MIQCP——每个二次候选都被一条变量等式固定——因此需要支持非凸二次约束的求解器。Rust 的 `exact = false` 是上包络松弛：没有最小化目标或其他约束把 y 向下推时，它不一定等于数学最大值。

## 求解器数学模型

### Kotlin

每个含二次项（或二次中间符号）的输入都会先被绑定：创建名为 `${name}_input_$i` 的有符号连续桥接变量 $b_i\in\mathbb R$，并用一条精确二次等式固定它：

$$
p_i(x)-b_i=0.
$$

桥接变量的范围被收紧到输入的有限范围；当捕获的范围不是浮点可表示时，会扩宽到相邻的可表示 double。仿射输入不经过桥接，直接透传。注册时会校验有限且未扩大的范围：范围在首次捕获后被扩宽会被拒绝，收紧则是安全的。

随后对绑定后的输入应用线性最大值模型；完整的约束行讨论见[最大值](/guide/linear-functional/max)。令结果为 $y$——内部函数的 `${name}_max` 变量——选择变量 $s_i\in\{0,1\}$：

$$
y\ge p_i,\qquad
y-p_i+M_i s_i\le M_i,
\qquad
\sum_i s_i=1.
$$

$M_i$ 优先使用显式 `bigM`，否则从候选范围推导，再退回每个候选的默认 Big-M。当所有候选都有有限范围时，结果范围会被收紧到候选范围，因此可以表示负的最大值。

### Rust

Rust 为每个候选创建名为 `{name}_bridge{i}` 的 `QuadraticLinearFunction` 桥接，并对绑定后的输入应用内部 `MaxFunction`。纯线性候选保持表达式形式——桥接只为真正的二次输入注册辅助 token——二次候选则贡献一条桥接等式加上最大值约束行：

$$
y\ge b_i,
$$

`exact = true` 时再提交：

$$
y-b_i+M_i s_i\le M_i,
\qquad
s_i\in\{0,1\},
\qquad
\sum_i s_i=1.
$$

`exact = false` 时只注册 $y\ge b_i$ 约束行。有 token 边界时，`mechanism_constraints_with_tokens` 从原始二次候选推导 Big-M；否则使用内部函数的通用回退策略。

## 当前 API

### Kotlin

源码：[`QuadraticMax.kt`（`QuadraticMaxFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticMax.kt)

```kotlin
QuadraticMaxFunction(
    polynomials: List<QuadraticPolynomial<V>>,
    bigM: V? = null,
    converter: IntoValue<V>,
    name: String = "quadratic_max",
    displayName: String? = null
)
```

构造器没有 `exact` 标志；Kotlin 始终注册精确选择模型。`createFunction` 使用相同的 `name`、`bigM`、`converter` 和 `displayName` 委托给 `MaxFunction`。不存在单参数的 `evaluate(values)` 重载；请调用 `evaluate(values, tokenTable, converter, zeroIfNone = false)`：

```kotlin
val value = maximum.evaluate(
    mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0)),
    null,
    IntoValue.Identity,
    zeroIfNone = false
)
```

### Rust

Rust 在 `quadratic_function.rs` 中提供 [`QuadraticMaxFunction<V>`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)：

```rust
QuadraticMaxFunction::new(
    id: u64,
    name: &str,
    inputs: Vec<Quadratic<V>>,
    exact: bool,
) -> QuadraticMaxFunction<V>
```

`result_variable()` 返回内部的 `name + "_max"` 连续变量，`with_declared_dependencies` 保留显式依赖 ID。每个候选都会被名为 `name + "_bridge" + i` 的 `QuadraticLinearFunction` 桥接包装；`exact = true` 创建内部二值选择变量，`exact = false` 只保留下包络约束行。`calculate_value` 始终计算数学最大值。有 token 边界时，`mechanism_constraints_with_tokens` 从原始二次候选推导 Big-M；否则使用通用回退策略。Rust 该类型的构造器没有 `bigM` 参数。

## evaluate 与 solver 的差异

直接求值解析原始输入符号、填充桥接变量并委托给线性 `MaxFunction`；无论 `exact` 如何设置，它都计算精确最大值，符号缺失或候选列表为空时返回 `null`。`prepare(values, tokenTable, converter)` 以 `zeroIfNone = false` 转发到同一求值路径。

solver 注册会添加桥接等式和选择模型，把桥接与结果范围收紧到候选范围，并依赖有效的 Big-M；`bigM` 过小时，solver 可能不可行，而直接求值仍能成功。公开的 `polynomial` 是线性对应仿射结果提升为二次多项式后的结果，`helperVariables` 由桥接变量与内部函数的结果和选择变量组成。

## 最小示例

::: code-group

```kotlin [Kotlin]
import fuookami.ospf.kotlin.core.solver.value.IntoValue
import fuookami.ospf.kotlin.core.symbol.function.QuadraticMaxFunction
import fuookami.ospf.kotlin.core.variable.RealVar
import fuookami.ospf.kotlin.math.algebra.number.Flt64
import fuookami.ospf.kotlin.math.symbol.Symbol
import fuookami.ospf.kotlin.math.symbol.monomial.QuadraticMonomial
import fuookami.ospf.kotlin.math.symbol.polynomial.QuadraticPolynomial

val x = RealVar("x")
val y = RealVar("y")
val first = QuadraticPolynomial(
    listOf(QuadraticMonomial.quadratic(Flt64.one, x, y)), Flt64.one
)
val second = QuadraticPolynomial(
    listOf(QuadraticMonomial.linear(Flt64.one, x)), Flt64.two
)
val maximum = QuadraticMaxFunction(
    polynomials = listOf(first, second),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "quadratic_max"
)
val value = maximum.evaluate(
    mapOf<Symbol, Flt64>(x to Flt64.two, y to Flt64(5.0)),
    null,
    IntoValue.Identity,
    zeroIfNone = false
)
check(value == Flt64(11.0))
```

```rust [Rust]
use ospf_rust_core::symbol::FunctionSymbol;
use ospf_rust_core::symbol::flatten::{Quadratic, QuadraticMonomial};
use ospf_rust_core::symbol::function::QuadraticMaxFunction;
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
let maximum = QuadraticMaxFunction::new(
    17,
    "quadratic_max",
    vec![
        Quadratic::new(vec![QuadraticMonomial::new_quadratic(1.0, 0, 1)], 1.0),
        Quadratic::new(vec![QuadraticMonomial::new_linear(1.0, 0)], 2.0),
    ],
    true,
);
assert_eq!(maximum.calculate_value(&tokens, false), Some(11.0));
```

:::

## QuadraticMaxMinFunction 与 QuadraticMinMaxFunction

$$
MaxMin(p_1,\ldots,p_n)=\min_i p_i,\qquad
MinMax(p_1,\ldots,p_n)=\max_i p_i.
$$

虽然名称容易引起误解，`QuadraticMinMaxFunction` 实际计算的是最大值——min-max 目标的精确极值；`QuadraticMaxMinFunction` 计算的是最小值——max-min 目标的精确极值，适用于最大化最差候选的场景。名称描述的是优化语境下的解释，而不是另一种聚合算法。两者都继承自 `QuadraticFunctionSymbol`，并把桥接、辅助变量和约束注册全部委托给各自的线性对应物：`QuadraticMaxMinFunction` 包装内部 `MaxMinFunction`（它本身是对 `MinFunction` 的包装），`QuadraticMinMaxFunction` 包装内部 `MinMaxFunction`（它本身是对 `MaxFunction` 的包装）。两个包装器都接收相同的 `polynomials`、可选 `bigM`、`converter`、`name` 和可选 `displayName` 参数，默认名称分别为 `quadratic_maxmin` 和 `quadratic_minmax`。两个极值都是精确的，且与目标方向无关。

源码：[`QuadraticMaxMin.kt`（`QuadraticMaxMinFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticMaxMin.kt) 与 [`QuadraticMinMax.kt`（`QuadraticMinMaxFunction`）](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/main/fuookami/ospf/kotlin/core/symbol/function/QuadraticMinMax.kt)

```kotlin
val maxMin = QuadraticMaxMinFunction(
    polynomials = listOf(first, second),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "quadratic_maxmin"
)
val minMax = QuadraticMinMaxFunction(
    polynomials = listOf(first, second),
    bigM = Flt64(10.0),
    converter = IntoValue.Identity,
    name = "quadratic_minmax"
)
```

Rust 现已在 [`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs) 中提供同样的两个包装器：

```rust
QuadraticMaxMinFunction::new(
    id: u64,
    name: &str,
    inputs: Vec<Quadratic<V>>,
) -> QuadraticMaxMinFunction<V>
QuadraticMaxMinFunction::with_declared_dependencies(
    self,
    dependency_ids: Vec<u64>,
) -> Self
QuadraticMinMaxFunction::new(
    id: u64,
    name: &str,
    inputs: Vec<Quadratic<V>>,
) -> QuadraticMinMaxFunction<V>
QuadraticMinMaxFunction::with_declared_dependencies(
    self,
    dependency_ids: Vec<u64>,
) -> Self
```

与 Kotlin 构造器不同，Rust 构造器不接受 `bigM`、`converter` 或 `displayName` 参数。每个二次候选都会得到名为 `{name}_bridge{i}` 的 `QuadraticLinearFunction` 桥接（即精确二次等式行），纯线性候选保持表达式形式。线性侧复用线性外壳的精确委托目标：`QuadraticMaxMinFunction` 包装精确的 `MinFunction`，注册名为 `{name}_min_*` 的约束行；`QuadraticMinMaxFunction` 包装精确的 `MaxFunction`，注册名为 `{name}_max_*` 的约束行。Big-M 从原始候选的 token 边界推导，并以最小 Big-M 为下限；没有可用 token 边界时使用内部函数的通用回退策略。直接求值对原始二次候选折叠 `min`/`max`。与 Kotlin 委托链一致，两个极值都是精确的，且与目标方向无关。

线性[最小值](/guide/linear-functional/min)页面的 `MinMaxFunction and MaxMinFunction` 一节在线性侧解释了同样的命名陷阱。这种桥接组合——每个二次输入一条有界实变量加一条精确二次等式——也正是让每个候选保持二次而不被展开为三次或四次项的原因。

## 测试与参考

- 覆盖全部三个符号的组合、求值与注册测试：[`QuadraticFunctionCompositionTest.kt`](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-core/src/test/fuookami/ospf/kotlin/core/symbol/function/QuadraticFunctionCompositionTest.kt)。通用求值与注册测试套件只覆盖 Min 对应物，不覆盖 Max 系列。
- Rust 实现与文件内回归测试（`quadratic_min_max_calculate_value`、`quadratic_max_infers_big_m_from_original_candidate_bounds` 与 `quadratic_max_min_and_min_max_calculate_value`）：[`quadratic_function.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/src/symbol/functions/quadratic_function.rs)
- Rust 专项契约测试：[`function_symbol_quadratic_max.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_max.rs)
- MaxMin/MinMax 包装器的 Rust 专项契约测试：[`function_symbol_quadratic_max_min.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_max_min.rs) 与 [`function_symbol_quadratic_min_max.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/function_symbol_quadratic_min_max.rs)
- MaxMin/MinMax 包装器的 Rust 端到端求解覆盖：[`gurobi_quadratic_model_integration.rs`](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-core/tests/gurobi_quadratic_model_integration.rs)（`gurobi_solves_quadratic_max_min_with_non_linear_input`、`gurobi_solves_quadratic_min_max_with_non_linear_input`）

## 相关页面

- [二次最小值](./quadratic-min)
- [最大值](/guide/linear-functional/max)
- [最小值](/guide/linear-functional/min)
