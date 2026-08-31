# Task Compilation

[中文](README_ch.md)

This directory contains task-compilation model components, context wrappers, and limits pipelines. It maps Kotlin `gantt-scheduling-domain-task-compilation-context`.

## Responsibilities

- Register task assignment variables and switch variables into `MetaModel`.
- Build time variables, makespan, and solution-summary structures.
- Analyze solver solutions back into task scheduling results.
- Provide constraint and objective pipelines for delays, advances, cost, makespan, and switch behavior.

## Modules

- `adapter.rs`: variable array helpers and expression builders.
- `context.rs`: task compilation contexts and aggregation wrappers.
- `iterative.rs`: iterative compilation state.
- `model.rs`: compilation, task time, makespan, switch, and solution models.
- `service/limits.rs`: constraint and objective pipelines.
- `service/mod.rs`: service re-exports and analyzers.

## Public API

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

## Related Directories

- [`../task`](../task/README.md)
- [`../capacity_scheduling`](../capacity_scheduling/README.md)
- [`../../application/algorithm`](../../application/algorithm/README.md)
