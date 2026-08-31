# 任务编译

[English](README.md)

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

## 相关目录

- [`../task`](../task/README_ch.md)
- [`../capacity_scheduling`](../capacity_scheduling/README_ch.md)
- [`../../application/algorithm`](../../application/algorithm/README_ch.md)
