# ospf-rust-framework-gantt-scheduling 迁移完成文档

## 1. 总目标

将 `E:\workspace\ospf\ospf-kotlin\ospf-kotlin-framework-gantt-scheduling` 迁移到 Rust workspace 中的新 crate：

```text
E:\workspace\ospf\ospf-rust\ospf-rust-framework-gantt-scheduling
```

迁移目标不是逐文件翻译 Kotlin，而是用 Rust 风格复刻 Gantt Scheduling 领域框架能力，并严格对齐当前 Rust 项目的 framework 架构规范：

1. 以 `ospf_rust_core::model::MetaModel` 为模型装配轴心。
2. 以 context / aggregation / model component / pipeline 组织领域能力。
3. application 层只负责流程编排、solver 选择、状态映射、trace/KPI/render 组织，不直接堆积变量、约束、目标和 token 解析逻辑。
4. 普通 MILP、列生成 LP、列生成 final MILP 共享同一套 context / aggregation / pipeline 注册路径。
5. public API 保持泛型数值和物理量边界，求解器适配边界内再落到 `f64`。

更新后的总目标为“与 Kotlin 版本具备同等级别的可用特性与高级求解语义”。当前 Rust Gantt crate 已达到该目标在本 crate 范围内的完成态。

## 2. 完成结论

当前已达到更新后的严格 Kotlin 能力对齐目标。

已完成能力包括：

1. Kotlin README 范围内的 Gantt 子模块均有 Rust 实现，或在 README 中明确当前边界。
2. 普通 MILP、列生成 LP、列生成 final MILP 共享 context / aggregation / pipeline 注册路径。
3. branch-and-price 能跑通小规模端到端样例，并具备 fresh-model 多节点树搜索入口。
4. resource / produce / capacity / bunch generation 均有核心测试。
5. public API 无 `Generic` / `Typed` 等迁移期命名后缀。
6. application 层建模职责已下沉到 domain context / lifecycle / policy 扩展点。
7. Kotlin 通用领域能力中已明确出现的 `TaskStepGraph` / 多步任务步骤图具备 Rust 实现。
8. Kotlin `SlotBasedBunchCompilationContext` 的时隙级束编译、产能预求解和按时隙加列语义具备 Rust 实现。
9. 动态模型生命周期已迁移到 `ospf-rust-framework::model::DynamicModelLifecycle`，覆盖 warm start、`setSolution`、`flush`、列范围刷新、列移除和状态恢复。
10. branch-and-price 具备树搜索执行语义，包括节点隔离、分支决策传播、强分支扩展点、cut callback 扩展点和节点回调扩展点。
11. 复杂班次日历、下游自定义成本公式和业务规则注入具备标准 domain/context/pipeline 扩展入口，并有最小测试。
12. capacity scheduling 已补齐 Kotlin 的动作上界注册、无序/带序解提取和 `CapacityColumnAggregation` 加列/去重/移除语义。
13. task compilation `Switch` 已补齐 Kotlin 的静态时间切换、可选 `TaskTime` 动态切换、`SwitchCostMinimization` 和 `SwitchTimeMinimization` 目标 pipeline。

## 3. 关键实现摘要

### 3.1 动态模型生命周期

已将 Gantt 迁移过程中补出的动态生命周期上移到 `ospf-rust-framework::model::DynamicModelLifecycle`，`domain::common::GanttDynamicModelLifecycle` 仅作为兼容别名：

1. core `MetaModel` 已提供按 solver 顺序 `set_solution`、`clear_solution`、`flush` 和变量范围读写接口。
2. 从当前解派生 warm-start 列。
3. 统一处理列隐藏、列固定、列移除和列范围恢复，并可同步写回 `MetaModel`。
4. `flush` 清除临时隐藏、固定和 warm-start 状态，同时保留 removed 列。
5. 提供 snapshot / restore，供 branch-and-price 节点隔离使用。
6. `IterativeTaskCompilationContext` 与 `IterativeBunchCompilationContext` 增加 `flush` / `apply_lifecycle` 入口。

说明：solver 原生 warm start 属于 solver adapter 边界；framework 生命周期已能将缓存解写回 `MetaModel`，Gurobi adapter 会从 token result 映射到原生 `Start`，其他 adapter 可复用同一模型状态接口接入。

### 3.2 Branch-And-Price 高级树搜索

已扩展 `application::algorithm::BranchAndPriceTreeSearch`：

1. `StrongBranchingStrategy`：下游可替换分支目标选择。
2. `BranchCutCallback`：下游可在节点求解后决定继续或剪枝。
3. `BranchNodeCallback`：下游可记录节点开始、结束、incumbent 更新等 trace/KPI。
4. `BranchSearchHooks`：统一挂载强分支、cut 和节点回调。
5. `search_with_hooks`：保持原 `search` 兼容，同时提供高级扩展入口。
6. `search_bunch_branch_and_price_with_hooks`：将 hooks 接入 fresh-model 隔离节点求解。

已测试强分支目标改写、cut 剪枝、节点 trace、fresh-model sibling lifecycle 隔离。

### 3.3 复杂班次日历与自定义成本

已新增标准扩展入口：

1. `infrastructure::CalendarPolicy`：封装 `WorkingCalendar` 上的复杂班次、额外不可用时间、连接窗口和休息规则。
2. `infrastructure::CompositeCalendarPolicy`：提供最小可组合日历策略。
3. `domain::task::BunchCostPolicy`：注入任务成本、执行器成本、连接成本、产能成本和软约束惩罚。
4. `domain::task::FunctionalBunchCostPolicy`：支持用函数闭包快速接入下游业务成本公式。
5. `DefaultBunchGenerationPolicy::with_cost_policy`：无需修改 application solver 主流程即可接入自定义 reduced cost。

### 3.4 Capacity 与 Switch 复审补齐

重新对照 Kotlin `CapacityCompilation`、`CapacityOrderCompilation`、`CapacityColumnAggregation`、`Switch`、`SwitchCostMinimization` 和 `SwitchTimeMinimization` 后，Rust 侧已补齐：

1. `CapacityCompilation::register` 与 `CapacityOrderCompilation::register` 会把 `ProductionActionTrait::upper_bound_at(slot)` 写入变量上界。
2. 无序与带序 capacity compilation 均可从 solver-order solution 提取 `ActionAllocation` 和 `ExecutorCapacityResult`。
3. `CapacityColumnAggregation` 支持按 iteration 加列、已有列去重、removed 列记录、最近 iteration 查询和清空。
4. `Switch::register_with_compilation` 覆盖 Kotlin 默认静态时间路径：只为相邻静态任务注册 switch，并构建 switch-time 汇总符号。
5. `Switch::register_with_task_time` 覆盖 Kotlin 可选动态时间路径：基于 `front_of`、`between_in`、executor 级 switch 和 masked switch-time 注册动态切换语义。
6. `SwitchCostMinimization` 与 `SwitchTimeMinimization` 已作为标准 objective pipeline 接入 `MetaModel`，其中 switch-time 支持 Kotlin 对应的 threshold slack 语义。

## 4. 验收清单

- [x] 动态模型生命周期抽象
- [x] warm start / set solution / flush 统一入口
- [x] 列范围刷新、列移除、列隐藏、列固定一致性测试
- [x] fake model / fake solver 生命周期测试
- [x] branch-and-price 多节点执行协议
- [x] 强分支策略 trait 与 fake 测试
- [x] cut callback trait 与 fake 测试
- [x] 节点回调 trait 与 trace/KPI 测试
- [x] sibling 节点状态隔离回归测试
- [x] 复杂班次日历扩展入口
- [x] 自定义成本公式 pipeline / policy
- [x] 下游业务扩展示例或最小测试
- [x] Capacity 上界、解提取和列聚合复审补齐
- [x] Switch 静态/动态切换与目标 pipeline 复审补齐
- [x] README / README_ch / gantt.md 收口
- [x] Gantt crate 验收命令通过

## 5. 当前边界

1. solver 原生 warm start 属于 adapter 能力；Gurobi adapter 已读取 token result 并写入原生 `Start`，其他 adapter 可按同一 `MetaModel` solution 状态继续补齐。
2. 并发树执行和 solver 原生节点回调仍属于 solver/core 层能力，不在当前 Gantt crate 内部实现；Gantt crate 已提供 trait 扩展入口。
3. 商业 solver 下的强分支与 cut 注入需要由下游 solver adapter 将本 crate hooks 映射到对应 solver 原生接口。

## 6. 验收命令

```powershell
cargo check -p ospf-rust-framework-gantt-scheduling
cargo test -p ospf-rust-framework-gantt-scheduling --no-run
cargo test -p ospf-rust-framework-gantt-scheduling --lib
cargo check -p ospf-rust-framework-gantt-scheduling --features async
```

已知上游 warning：

1. `ospf-rust-math` 中 `point2_system` unused macro warning。
2. `ospf-rust-quantities` 中 `UnitSystemBuilder::prototype` dead code warning。

这些 warning 来自上游 crate，不是 Gantt crate 新增 warning。

## 7. Kotlin 复审结论

已重新复审 Kotlin 关键实现：

1. `gantt-scheduling-application/.../service/bunch/BranchAndPriceAlgorithm.kt`
2. `gantt-scheduling-domain-bunch-compilation-context/.../SlotBasedBunchCompilationContext.kt`
3. `gantt-scheduling-infrastructure/.../WorkingCalendar.kt`
4. `gantt-scheduling-domain-task-context/.../Cost.kt`
5. `gantt-scheduling-domain-capacity-scheduling-context/.../CapacityCompilation.kt`
6. `gantt-scheduling-domain-capacity-scheduling-context/.../CapacityOrderCompilation.kt`
7. `gantt-scheduling-domain-capacity-scheduling-context/.../CapacityColumnAggregation.kt`
8. `gantt-scheduling-domain-task-compilation-context/.../Switch.kt`
9. `gantt-scheduling-domain-task-compilation-context/.../SwitchCostMinimization.kt`
10. `gantt-scheduling-domain-task-compilation-context/.../SwitchTimeMinimization.kt`

对照结论：

1. Kotlin 的 `model.setSolution()`、`model.flush()`、变量范围刷新和列固定/隐藏/移除语义已在 Rust core/framework 层收敛，Gantt 通过兼容别名复用该实现。
2. Kotlin 的 slot-based bunch compilation、capacity pre-solve、slot constraints、add columns by slot 已在 Rust 侧具备对应 trait/context。
3. Kotlin 的复杂日历组合能力已由 `WorkingCalendar` 与 `CalendarPolicy` 承接。
4. Kotlin 的成本模型与业务公式扩展已由 `Cost` / `MutableCost` 与 `BunchCostPolicy` 承接。
5. branch-and-price 的树搜索、节点隔离、分支决策传播和回调扩展点已具备 Rust 可用语义。
6. Kotlin capacity scheduling 的动作上界、解提取和列聚合语义已在 Rust capacity model 中补齐。
7. Kotlin task compilation switch 的默认静态路径、可选 `TaskTime` 动态路径和 switch objective pipeline 已在 Rust `Switch` 与 limits pipeline 中补齐。

因此，本 crate 的 Gantt Scheduling 迁移总目标已完成。
