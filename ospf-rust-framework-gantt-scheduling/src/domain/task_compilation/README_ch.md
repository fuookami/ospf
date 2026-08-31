# 任务编译

:us: [English](README.md) | :cn: 简体中文

本目录包含任务编译模型组件、context 封装和限制 pipeline，对应 Kotlin `gantt-scheduling-domain-task-compilation-context`。

## 职责

- 向 `MetaModel` 注册任务分配变量和切换变量。
- 构建时间变量、完工时间和解摘要结构。
- 将 solver 解分析回任务排程结果。
- 提供延迟、提前、成本、完工时间和切换行为的约束与目标 pipeline。

## 模块

- `adapter.rs`：变量数组辅助和表达式构造。
- `context.rs`：任务编译上下文和聚合封装。
- `iterative.rs`：迭代编译状态。
- `model.rs`：编译、任务时间、完工时间、切换和解模型。
- `service/limits.rs`：约束和目标 pipeline。
- `service/mod.rs`：服务层重导出和分析器。

## 公共 API

- `Compilation`
- `TaskTime`
- `Makespan`
- `Switch`
- `TaskSolution`
- `TaskSolutionSummary`
- `TaskTimeInfo`
- `SolutionAnalyzer`
- `IterativeTaskCompilation`
- `IterativeTaskCompilationContext`
- `BasicTaskCompilationContext`
- `TaskCompilationConstraint`
- `ExecutorCompilationConstraint`
- `TaskConflictConstraint`
- `TaskTimeConflictConstraint`
- `TaskDelayTimeConstraint`
- `TaskAdvanceTimeConstraint`
- `TaskOverMaxDelayTimeConstraint`
- `TaskOverMaxAdvanceTimeConstraint`
- `TaskDelayLastEndTimeConstraint`
- `TaskAdvanceEarliestEndTimeConstraint`
- `TaskExecutorCostMinimization`
- `TaskCostMinimization`
- `MakespanMinimization`
- `SwitchCostMinimization`
- `SwitchTimeMinimization`
- `TaskDelayTimeMinimization`
- `TaskAdvanceTimeMinimization`
- `ExecutorCostMinimization`
- `ExecutorLeisureMinimization`

## 扩展点

任务编译行为通过 compilation context、iterative compilation state、solution analyzer，以及 assignment、conflict、time、cost、makespan、switch 相关 limit pipeline 扩展。任务词汇保留在 `task`，任务束级列保留在 `bunch_compilation`。

## 生命周期与数据流

compilation context 将 assignment、timing、switch 和 makespan 变量注册到 `MetaModel`；iterative context 维护动态编译状态；limit pipeline 添加约束和目标；analyzer 将 solver value 转换为 task solution 与 summary。

## 验证

修改 task variable registration、iterative state、limit pipeline 或 solution analysis 时运行 `cargo test -p ospf-rust-framework-gantt-scheduling --lib`。

## 相关目录

- [`../task`](../task/README_ch.md)
- [`../capacity_scheduling`](../capacity_scheduling/README_ch.md)
- [`../../application/algorithm`](../../application/algorithm/README_ch.md)
