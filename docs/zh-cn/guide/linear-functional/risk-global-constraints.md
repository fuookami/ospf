# 风险度量与全局约束

## 条件风险价值

给定情景损失 $L_i$、概率 $p_i$ 和置信水平 $\alpha$，离散 CVaR 定义为

$$
\operatorname{CVaR}_{\alpha}(L)=\min_{\eta}\left(\eta+\frac{1}{1-\alpha}\sum_i p_i\max(L_i-\eta,0)\right)
$$

`CvarFunction` 是精确离散形式：枚举每个情景损失作为候选阈值，通过精确正部函数计算超额项，再选取最小候选值。它可用于最小化、最大化或约束，不依赖目标方向。损失需要有限范围；概率必须非负且总和为 1，置信水平满足 $0\le\alpha<1$。

`CvarEpigraphFunction` 是规模较小的 LP 上图形式，其结果表达式始终不小于 CVaR。对该表达式进行最小化时，最优解会收紧至 CVaR。将表达式限制为不超过某个上界，其辅助变量存在的可行性与真实 CVaR 不超过该上界等价，但不会强制每个可行点的表达式都等于 CVaR。不要最大化该表达式，也不要用它施加 CVaR 下界；辅助变量可能使表达式高于真实值。

```kotlin
val exactRisk = CvarFunction(
    losses = scenarioLosses,
    probabilities = probabilities,
    alpha = Flt64(0.95),
    converter = converter,
    name = "exact_tail_risk"
)
val riskEpigraph = CvarEpigraphFunction(
    losses = scenarioLosses,
    probabilities = probabilities,
    alpha = Flt64(0.95),
    converter = converter,
    name = "tail_risk_epigraph"
)
```

```rust
use std::collections::HashMap;
use std::sync::Arc;

use ospf_rust_core::model::BasicModel;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::symbol::function::{CvarEpigraphFunction, CvarFunction};
use ospf_rust_core::variable::{ContinuousVariableItem, VariableId, VariableRange};

let mut model = BasicModel::new("scenario_risk");
let loss_a = ContinuousVariableItem::with_range(
    VariableId::standalone(1), "loss_a", VariableRange::bounded(0.0, 10.0)
);
let loss_a_index = model.register_variable(loss_a)?;
let loss_b = ContinuousVariableItem::with_range(
    VariableId::standalone(2), "loss_b", VariableRange::bounded(0.0, 20.0)
);
let loss_b_index = model.register_variable(loss_b)?;
let scenario_losses = vec![
    Linear::new(vec![LinearMonomial::new(1.0, loss_a_index)], 0.0),
    Linear::new(vec![LinearMonomial::new(1.0, loss_b_index)], 0.0),
];
let probabilities = vec![0.5, 0.5];
let exact_risk = Arc::new(CvarFunction::new(
    20, "exact_tail_risk", scenario_losses.clone(), probabilities.clone(), 0.95
)?);
let risk_epigraph = Arc::new(CvarEpigraphFunction::new(
    21, "tail_risk_epigraph", scenario_losses, probabilities, 0.95
)?);
model.add_symbol(exact_risk.clone())?;
model.add_symbol(risk_epigraph.clone())?;
let symbol_to_index = model.tokens().iter()
    .map(|token| (token.id().unique_id() as usize, token.solver_index))
    .collect::<HashMap<_, _>>();
let exact_expression = exact_risk.result_expression(&symbol_to_index)?;
let epigraph_expression = risk_epigraph.result_expression(&symbol_to_index)?;
```

Rust 符号必须先注册，再请求 `result_expression`；此方法通过模型最终分配的 solver index 解析辅助变量 ID。上图结果由阈值与过量变量组成，没有单独的结果列，因此同样使用该映射接口。

## 约束规划全局约束

全局约束在约束规划（CP）模型中保留其自然结构：

| 约束 | 含义 |
|---|---|
| `allDifferent` / `all_different` | 整数表达式两两取值不同 |
| `noOverlap` / `no_overlap` | 区间互不重叠；可选区间遵循其存在状态 |
| `cumulative` | 任意时刻活动区间的总需求不超过容量 |

Kotlin 与 Rust 的 `GlobalConstraintFunctions` 都创建相应 CP 约束节点。若应用适合使用 CP 全局约束，应优先考虑 CP 求解器。它们出现在 function 包中，并不表示已转换成普通线性函数。

```kotlin
val allDifferent = GlobalConstraintFunctions.allDifferent(integerExpressions)
val noOverlap = GlobalConstraintFunctions.noOverlap(intervals)
val cumulative = GlobalConstraintFunctions.cumulative(intervals, demands, capacity)
```

```rust
let all_different = GlobalConstraintFunctions::all_different(integer_expressions);
let no_overlap = GlobalConstraintFunctions::no_overlap(interval_ids);
let cumulative = GlobalConstraintFunctions::cumulative(tasks, capacity)?;
```

Kotlin 的 cumulative AST 接受 CP 表达式作为需求与容量。Rust 便捷工厂使用每个任务的固定整数需求和整数容量。MIP lowering 中 cumulative 默认关闭；显式启用后会按有限整数时域展开，且需求与容量必须属于支持的常量形式，展开规模受 slot/work 预算限制。`AllDifferent` 和 `NoOverlap` 的 lowering 也要求支持的有限整数域和区间界。仅在模型明确启用受支持的 lowering 时才使用相应策略。

`Cumulative` 的 MIP lowering 要求 start、end 和 duration 具有有限整数界；可选区间会通过 presence literal 一并处理，而需求和容量必须为非负常量。默认 MIP lowering policy 不启用此功能。Kotlin 构造 `ConstraintProgrammingToLinearModelLowerer` 时，可通过 `ConstraintProgrammingLoweringPolicy(allowCumulative = true)` 显式开启；`MipBackedConstraintProgrammingSolver` 默认将其报告为 `Unsupported`，开启后报告 `ExactLowering`。Kotlin 当前会拒绝被 implication 或 reification 包裹的 `Cumulative`。Rust 使用对应的 `MipLoweringOptions` 字段，并且 `lower_constraint_programming_with_options` 同样要求显式开启；其按真值进行的降阶会保留 implication 与等价 reification 语义。每条约束的默认上限为 4,096 个整数时隙和 65,536 个 interval-slot 工作单元：

```kotlin
import fuookami.ospf.kotlin.core.solver.constraint_programming.MipBackedConstraintProgrammingSolver
import fuookami.ospf.kotlin.core.solver.constraint_programming.lowering.ConstraintProgrammingLoweringPolicy
import fuookami.ospf.kotlin.core.solver.constraint_programming.lowering.ConstraintProgrammingToLinearModelLowerer

val lowerer = ConstraintProgrammingToLinearModelLowerer(
    ConstraintProgrammingLoweringPolicy(
        allowCumulative = true,
        maxCumulativeTimeSlots = 4096,
        maxCumulativeWork = 65_536
    )
)
val mipBackedSolver = MipBackedConstraintProgrammingSolver(linearSolver, lowerer)
```

```rust
use ospf_rust_core::solver::constraint_programming::{
    lower_constraint_programming_with_options, MipLoweringOptions,
};

let options = MipLoweringOptions {
    allow_cumulative: true,
    max_cumulative_time_slots: 4_096,
    max_cumulative_work: 65_536,
    ..MipLoweringOptions::default()
};
let lowered = lower_constraint_programming_with_options(&snapshot, options)?;
```

较长时域、很宽的变量域或更复杂的 CP 表达式可能超出预算或当前支持范围；遇到这类模型时，应使用原生处理这些约束的求解器。
