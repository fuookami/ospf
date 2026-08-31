# OSPF Rust Framework Gantt Scheduling

🇺🇸 [English](README.md) | 🇨🇳 简体中文

本 crate 是 `ospf-kotlin-framework-gantt-scheduling` 的 Rust 迁移目标。
当前状态已包含 Gantt 领域框架基础、任务与任务束编译、资源/产出/产能上下文、
定价图与标签搜索、迭代列生命周期、隔离 branch-and-price 树搜索，以及已由测试覆盖的小规模 final MILP branch-and-price flow。

## Public API

- 任务领域：`domain::task::{TaskTrait, ExecutorTrait, AssignmentPolicyTrait,
  TaskPlanTrait, TaskStepGraph, TaskStepTrait, BasicTaskStep, StepRelation,
  Cost, BunchCostPolicy, CostBreakdown, DefaultBunchCostPolicy,
  SolverValueAdapter, F64SolverValueAdapter}`。
- 任务编译：`domain::task_compilation::{BasicTaskCompilationContext,
  IterativeTaskCompilationContext, Switch, SwitchCostMinimization,
  SwitchTimeMinimization}`。
- 产能排程：`domain::capacity_scheduling::{CapacityCompilation,
  CapacityOrderCompilation, CapacityColumn, CapacityColumnAggregation,
  CapacitySchedulingSolution}`。
- 任务束编译：`domain::bunch_compilation::{BasicBunchCompilationContext,
  IterativeBunchCompilationContext, BasicSlotBasedBunchCompilationContext,
  SlotBasedBunchCompilationContext, SlotBasedCapacityPreSolver, BunchEntry,
  BunchSolution}`。
- 任务束生成：`domain::bunch_generation::{SlotBasedBunchGenerator,
  BunchFeasibilityPolicy, BunchTaskCandidate, CapacityIntermediateValues}`。
- application helper：`application::service::create_bunch_branch_and_price` 和
  `application::service::search_bunch_branch_and_price_with_fresh_model`。
- 动态生命周期：`domain::common::GanttDynamicModelLifecycle`，由
  `ospf_rust_framework::model::DynamicModelLifecycle` 重导出。
- 日历策略：`infrastructure::{CalendarPolicy, CompositeCalendarPolicy}`。

泛型求解器数值转换使用 `SolverValueAdapter`，f64 求解边界使用 `F64SolverValueAdapter`。

## 扩展点

- `domain::bunch_generation::BunchFeasibilityPolicy` 用于向基于时隙的任务束生成注入任务、资源、产出、产能和业务可行性规则。
- `domain::task::BunchCostPolicy` 用于注入任务成本、执行器成本、连接成本、产能成本和软约束惩罚。`DefaultBunchGenerationPolicy::with_cost_policy` 可把这些业务公式接入 reduced cost 计算，而无需修改 application branch-and-price flow。
- `infrastructure::CalendarPolicy` 在 `WorkingCalendar` 外提供复杂班次、额外不可用时间、连接窗口和休息规则的可注入扩展入口。
- `domain::task::TaskStepGraph` 将多步任务依赖建模为经过校验的 DAG，覆盖起始步骤、前向步骤向量和后向步骤向量语义。
- `domain::task_compilation::Switch` 支持与 Kotlin 对齐的静态时间切换路径，以及可选 `TaskTime` 动态切换路径；切换成本和切换时间目标已作为标准 pipeline 暴露，并支持切换时间 threshold slack。
- `domain::capacity_scheduling::CapacityCompilation` 与 `CapacityOrderCompilation` 会注册每个时隙的动作上界并提取无序/带序产能解；`CapacityColumnAggregation` 负责 iteration 列记录、加列去重和 removed 列跟踪。
- `domain::bunch_compilation::SlotBasedBunchCompilationContext` 在迭代任务束编译上下文之上增加产能预求解、时隙约束查询、按时隙加列和按时隙查询任务束。
- `domain::common::ConstraintIndexMap` 将业务约束 key 映射到对偶解索引，用于稳定提取 shadow price。
- `domain::common::GanttDynamicModelLifecycle` 复用 framework 公共动态生命周期，集中承载 warm start、`setSolution`、`flush`、列隐藏/固定/移除和列范围恢复语义。`GanttModelStateFacade` 是共享列可选状态的兼容别名。
- `application::algorithm::BranchAndPriceTreeSearch` 提供与具体模型解耦的多节点 branch-and-price 搜索骨架。
- `application::algorithm::BranchAndPriceTreeSearch` 支持 `StrongBranchingStrategy`、`BranchCutCallback` 和 `BranchNodeCallback` 扩展点。
- `application::algorithm::BunchBranchAndPriceAlgorithm::solve_branch_node` 将分支决策适配为列状态 fallback，并复用当前单节点 branch-and-price 流程。它会在每个节点求解前后快照/恢复 application 状态，避免兄弟节点共享 shadow price、固定/隐藏列、incumbent 或迭代状态。
- `application::algorithm::BunchBranchAndPriceAlgorithm::solve_branch_node_with_fresh_model` 提供更强的节点隔离入口：快照编译上下文，并让每个节点在调用方构建的 fresh `MetaModel` 上求解。
- `application::service::search_bunch_branch_and_price_with_fresh_model` 将 `BranchAndPriceTreeSearch` 接到隔离节点入口，是多节点 branch-and-price 推荐的 application public helper。
- `application::service::search_bunch_branch_and_price_with_hooks` 在 fresh-model 隔离节点搜索上增加强分支、cut 和节点 trace hooks。

## Branch-And-Price Flow

已测试的 application flow：

1. 为每个分支节点构建 fresh `MetaModel<f64>`。
2. 注册任务束编译 context。
3. 求解初始 MILP。
4. 求解 RMP LP，并通过 context 提取 shadow price。
5. 通过 `BunchCGPolicy` 生成任务束，并用 `add_columns` 注册新列。
6. 求解 final MILP，并通过共享 context 提取 `BunchSolution`。
7. 在进入下一个树节点前恢复 application 与 context 状态。

## 当前动态模型边界

`GanttDynamicModelLifecycle` 现在是 framework 公共动态模型生命周期的 Gantt 兼容别名。它记录 solver solution、派生 warm-start 列、刷新临时状态、恢复列范围，并持久保留 removed 列。core `MetaModel` 已公开 solution、flush 和变量范围接口；生命周期会在 MILP 求解前把缓存解写回 `MetaModel`，供 adapter 将 token result 映射为原生 warm start。多节点树搜索优先使用 `solve_branch_node_with_fresh_model`，它会恢复 application 状态、动态生命周期和编译上下文，并在求解后丢弃节点本地 `MetaModel`。

## 当前边界

- 普通 MILP、RMP LP、final MILP 共享 context 与 iterative compilation 入口，用于注册、加列、shadow price 提取和解提取；solver 转换仍保留在 application 层。
- 多步任务图作为任务领域扩展模型暴露，现有单步任务编译路径保持不变。
- 时隙级任务束编译使用共享 `MetaModel<f64>` 求解边界和可插拔产能预求解器 trait；下游产能排程器可提供更丰富的中间值，而无需修改 application solver flow。
- 产能排程和任务切换注册现在复用 core 的变量范围、solution、函数符号和 objective pipeline 接口，不再依赖 Gantt 本地生命周期补丁。
- solver 原生 warm start 属于 adapter 能力；共享生命周期将缓存解写入 `MetaModel`，Gurobi adapter 已将 token result 映射到原生 `Start`，其他 adapter 可复用同一 solution 状态继续补齐。
- 并发树搜索和 solver 原生节点回调仍不属于本 crate 当前边界，但 branch-and-price search 已提供强分支、cut 和节点回调 trait 供下游集成。
- 复杂班次日历和自定义成本公式已通过标准 policy 扩展点支持，并有最小测试覆盖。

详细迁移目标、清单和验收标准见 [gantt.md](gantt.md)。
