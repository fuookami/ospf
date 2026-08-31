# OSPF Rust Framework Gantt Scheduling

:us: [English](README.md) | :cn: 简体中文

## 简介

`ospf-rust-framework-gantt-scheduling` 是 Kotlin `ospf-kotlin-framework-gantt-scheduling` 的 Rust 迁移目标。它提供可复用 Gantt scheduling 领域框架基础、task/bunch compilation、resource/produce/capacity context、pricing graph 和 label search、迭代列生命周期、隔离 branch-and-price tree search，以及已有测试覆盖的小型 final-MILP branch-and-price flow。

## 作用范围

本 crate 拥有 task modeling、task compilation、bunch compilation、bunch generation、capacity scheduling、resource constraint、produce/consumption tracking、time infrastructure 和 application-level branch-and-price 编排等可复用排程内核。

明确非目标：

1. 业务专用请求 DTO、租户上下文、公式语言和项目运行时策略。
2. solver backend 安装、许可证管理或 backend plugin 所有权。
3. 并发 tree execution 和 solver-native node callback；本 crate 暴露 hook 给下游集成。

## 模块结构

| Rust 模块或目录 | Kotlin 边界 | 职责 |
| --- | --- | --- |
| [`src/infrastructure`](src/infrastructure/README_ch.md) | `gantt-scheduling-infrastructure` | Time range、window、slot、duration range、working calendar、calendar policy、local date offset 和 render DTO。 |
| [`src/domain/task`](src/domain/task/README_ch.md) | `gantt-scheduling-domain-task-context` | Task、executor、assignment、task plan、task bunch、cost、solver value adapter、task-step graph 和 shadow-price key。 |
| [`src/domain/task_compilation`](src/domain/task_compilation/README_ch.md) | `gantt-scheduling-domain-task-compilation-context` | task-level MILP component、time variable、switch、makespan、solution analysis 和 limit/objective pipeline。 |
| [`src/domain/task_generation`](src/domain/task_generation/README_ch.md) | `gantt-scheduling-domain-task-generation-context` | 从 Kotlin 映射保留的 task-generation 扩展点。 |
| [`src/domain/bunch_compilation`](src/domain/bunch_compilation/README_ch.md) | `gantt-scheduling-domain-bunch-compilation-context` | task bunch 的 column-generation master problem、slot-based compilation、迭代列和 bunch solution。 |
| [`src/domain/bunch_generation`](src/domain/bunch_generation/README_ch.md) | `gantt-scheduling-domain-bunch-generation-context` | pricing graph、label-setting search、feasibility policy 和 slot-based bunch generation。 |
| [`src/domain/capacity_scheduling`](src/domain/capacity_scheduling/README_ch.md) | `gantt-scheduling-domain-capacity-scheduling-context` | capacity action、capacity/order compilation、capacity column、action bound 和 capacity solution。 |
| [`src/domain/resource`](src/domain/resource/README_ch.md) | `gantt-scheduling-domain-resource-context` | execution/storage/connection resource、resource usage、capacity、slack 和 resource limit。 |
| [`src/domain/produce`](src/domain/produce/README_ch.md) | `gantt-scheduling-domain-produce-context` | material demand、reserve、production task、produce usage、consumption usage 和 quantity objective。 |
| [`src/domain/common`](src/domain/common/README_ch.md) | shared Gantt domain helpers | constraint index 和 dynamic model lifecycle 兼容 alias。 |
| [`src/application`](src/application/README_ch.md) | `gantt-scheduling-application` | APS/MPS/LSP marker、column generation、branch-and-price algorithm、service constructor、hook 和 iteration state。 |

## 架构概览

本 crate 遵循 context / aggregation / pipeline 架构：

1. domain context 定义可复用排程实体，并向 `MetaModel` 注册变量、中间值、约束、目标和提取逻辑。
2. bunch compilation 是 column-generation master problem；bunch generation 是 pricing problem。
3. task compilation、capacity scheduling、resource 和 produce context 提供可复用 MILP component 与可选约束/目标族。
4. application algorithm 负责编排 LP/MILP 阶段、列生命周期、branch decision、hook 和状态恢复。

普通 MILP、RMP LP 和 final MILP 尽量共享 context 与 iterative compilation 入口，用于注册、加列、shadow-price 提取和 solution extraction。

## 核心概念

1. task 是可排程工作单元，包含 executor、time、duration、status 和可选 multi-step dependency graph。
2. task bunch 是分配给同一 executor 的有序路线/列。
3. bunch compilation 在 master problem 中选择列。
4. bunch generation 通过 pricing graph 和 label search 产生改进列。
5. dynamic model lifecycle 跟踪 warm-start state、column range、hidden/fixed/removed column 和 solver solution。

## Public API

| API | 职责 | 稳定性 |
| --- | --- | --- |
| `domain::task::{TaskTrait, ExecutorTrait, AssignmentPolicyTrait, TaskPlanTrait}` | 核心 task 与 assignment 抽象。 | migration |
| `domain::common::{GanttId, ExecutorIdTrait, TaskIdTrait, TaskPlanIdTrait}` | 强类型业务 ID 契约与默认字符串 newtype。 | migration |
| `domain::task::{TaskStepGraph, TaskStepTrait, BasicTaskStep, StepRelation}` | multi-step task dependency model。 | migration |
| `domain::task::{Cost, BunchCostPolicy, CostBreakdown, DefaultBunchCostPolicy}` | cost 与 reduced-cost policy surface。 | migration |
| `domain::task::{SolverValueAdapter, F64SolverValueAdapter}` | 泛型 solver value conversion 和 `f64` 边界。 | migration |
| `domain::task_compilation::{BasicTaskCompilationContext, IterativeTaskCompilationContext, Switch, SwitchCostMinimization, SwitchTimeMinimization}` | task compilation context 和 switch objective pipeline。 | migration |
| `domain::capacity_scheduling::{CapacityCompilation, CapacityOrderCompilation, CapacityColumn, CapacityColumnAggregation, CapacitySchedulingSolution}` | capacity scheduling 注册和提取。 | migration |
| `domain::bunch_compilation::{BasicBunchCompilationContext, IterativeBunchCompilationContext, BasicSlotBasedBunchCompilationContext, SlotBasedBunchCompilationContext, SlotBasedCapacityPreSolver, BunchEntry, BunchSolution}` | bunch master problem 和 slot-based column lifecycle。 | migration |
| `domain::bunch_generation::{SlotBasedBunchGenerator, BunchFeasibilityPolicy, BunchTaskCandidate, CapacityIntermediateValues}` | pricing 与 feasibility 扩展面。 | migration |
| `application::service::{create_bunch_branch_and_price, create_slot_bunch_branch_and_price, search_bunch_branch_and_price_with_fresh_model, search_bunch_branch_and_price_with_hooks}` | application helper constructor、slot capacity pre-solving 和 search 入口。 | migration |
| `application::algorithm::{BranchAndPriceTreeSearch, StrongBranchingStrategy, BranchCutCallback, BranchNodeCallback}` | 隔离 branch-and-price tree search hook。 | migration |
| `domain::common::GanttDynamicModelLifecycle` | shared framework dynamic lifecycle 的兼容 alias。 | migration |
| `infrastructure::{CalendarPolicy, CompositeCalendarPolicy}` | calendar 扩展 policy surface。 | migration |

## 建模扩展点

扩展点包括：

1. `domain::bunch_generation::BunchFeasibilityPolicy` 注入 task、resource、produce、capacity 和业务可行性规则。
2. `domain::task::BunchCostPolicy` 注入 task、executor、connection、capacity 和软约束 cost formula。
3. `infrastructure::CalendarPolicy` 注入复杂班次、额外不可用区间、连接窗口和休息规则。
4. `domain::task::TaskStepGraph` 表达经过校验的 multi-step task dependency。
5. `domain::task_compilation::Switch` objective pipeline 支持 static-time 与可选 dynamic `TaskTime` switch path。
6. `domain::capacity_scheduling::CapacityCompilation` 和 `CapacityOrderCompilation` 支持 unordered/ordered capacity extraction。
7. `domain::bunch_compilation::SlotBasedBunchCompilationContext` 支持 capacity pre-solving、slot constraint lookup、slot-wise column addition 和 slot-wise bunch query。
8. `domain::common::ConstraintIndexMap` 支持稳定 shadow-price extraction。
9. `application::algorithm::BranchAndPriceTreeSearch` 暴露 strong branching、cut 和 node tracing hook。

本次列生成扩展还包括：

10. `domain::bunch_compilation::ExecutorSlotCompilationConstraint` 为每个 `(executor, slot)` 注册恰选一列约束。
11. `ConstraintIndexMap` 支持 executor-slot 对偶值；`BunchPricingRequest` 将 slot 对偶、固定/保留 group 和最小列配额传入定价策略。
12. `BranchGroupTracker` 按 `(executor, slot)` 跟踪分支状态，部分 slot 固定时不会误移除整个 executor。
13. `SlotBunchPricingRequest` 将时隙入口状态和分支限制传入时隙定价策略；`CapacityColumnSelectionConstraint` 支持产能列恰选约束及其对偶提取。

## 泛型数值边界

domain API 通过 `SolverValueAdapter` 使用泛型 solver-value 抽象。`F64SolverValueAdapter` 标记当前 `f64` solver 边界。solver conversion 集中在 context registration、application solver call 和 result extraction，不应散落在 domain logic 中。

## 物理量边界

时间、持续时长、产能、资源数量、产量、消耗量、slack 和需求满足应使用 infrastructure time type、`ospf-rust-quantities` 物理量或明确 domain wrapper。裸 `f64` 限于 solver adapter、registration、extraction 和低层系数边界。

## 求解生命周期

已测试 slot branch-and-price application flow：

1. 预求解 capacity，并把 `CapacityIntermediateValues` 固化到 bunch-generation policy。
2. 为每个 branch node 构造新的 `MetaModel<f64>`。
3. 注册 bunch compilation context。
4. 求解 initial MILP。
5. 求解 RMP LP，并通过 context 提取 shadow price。
6. 通过 `BunchCGPolicy` 生成 slot bunch，并使用 `add_columns` 注册。
7. 求解 final MILP，并通过共享 context 提取 `BunchSolution`。
8. 在进入下一个 tree node 前恢复 application 和 context state。

multi-node tree search 优先使用 `solve_branch_node_with_fresh_model`，它会恢复 application state、dynamic lifecycle 和 compilation context，并在求解后丢弃 node-local `MetaModel`。

## 输出

`BunchSolution` 及相关 bunch scheduling 输出包含 selected bunch、task assignment、canceled task、executor assignment 和 total cost。`CapacitySchedulingSolution` 包含 capacity column 和每个 time slot 的 production action。iteration 与 branch-search 输出记录 LP/IP objective、node status、branch decision、incumbent state 和 trace hook。

## 使用方式

```rust,ignore
use ospf_rust_framework_gantt_scheduling::application::service::{
    create_bunch_branch_and_price,
    search_bunch_branch_and_price_with_fresh_model,
};

let algorithm = create_bunch_branch_and_price(config);
let result = search_bunch_branch_and_price_with_fresh_model(algorithm, input)?;
```

## 本地验证

```powershell
cargo check -p ospf-rust-framework-gantt-scheduling
cargo test -p ospf-rust-framework-gantt-scheduling
cargo check -p ospf-rust-framework-gantt-scheduling --features serde
```

## 当前边界

1. 普通 MILP、RMP LP 和 final MILP 共享 context 与 iterative compilation 入口，用于注册、加列、shadow-price extraction 和 solution extraction。solver conversion 仍在 application 层。
2. multi-step task graph 作为 domain extension model 暴露；既有 single-step task compilation 保持不变。
3. slot-based bunch compilation 当前使用共享 `MetaModel<f64>` solver 边界和可插拔 capacity pre-solver trait。
4. capacity scheduling 与 task switch registration 使用共享 core variable-range、solution、function-symbol 和 objective-pipeline interface，而不是 Gantt-local lifecycle shim。
5. native solver warm start 仍是 adapter 能力。shared lifecycle 把缓存的 solver-order solution 写入 `MetaModel`；Gurobi adapter 已能把 token result 映射到 native `Start`。
6. 并发 tree execution 和 solver-native node callback 仍在本 crate 之外，但 branch-and-price search 暴露 strong-branching、cut 和 node callback trait 给下游集成。
7. complex shift calendar 和 custom cost formula 已通过标准 policy extension point 支持，并有最小测试覆盖。

详细迁移目标、清单和验收标准应与本节当前边界清单以及 Kotlin Gantt Scheduling README 保持一致。

## 相关模块

- [根 README](../README_ch.md)
- [Gantt application README](src/application/README_ch.md)
- [Gantt domain README](src/domain/README_ch.md)
- [Kotlin Gantt Scheduling README](../../ospf-kotlin/ospf-kotlin-framework-gantt-scheduling/README_ch.md)
