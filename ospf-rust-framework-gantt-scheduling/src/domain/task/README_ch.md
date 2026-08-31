# 任务领域

[English](README.md)

本目录包含任务侧核心领域模型，对应 Kotlin `gantt-scheduling-domain-task-context`。

## 职责

- 建模执行器、任务、任务计划、任务束、分配和成本。
- 提供多步骤任务流程的 task-step graph 结构。
- 为下游编译层暴露求解器值适配器和影子价格 key。

## 模块

- `assignment.rs`：分配策略抽象。
- `cost.rs`：成本表达和可变成本累积。
- `cost_policy.rs`：任务束成本策略和函数式成本策略。
- `executor.rs`：执行器 trait 和基础执行器类型。
- `scheduling_solver_value_adapter.rs`：求解器值适配器抽象。
- `shadow_price.rs`：影子价格参数和 key 类型。
- `task_bunch.rs`：任务束模型。
- `task_plan.rs`：任务计划抽象和状态。
- `task_step_graph.rs`：任务步骤图和步骤关系。
- `task_trait.rs`：任务 trait 定义。

## 公共 API

- `ExecutorTrait`
- `BasicExecutor`
- `AssignmentPolicyTrait`
- `BasicAssignmentPolicy`
- `TaskTrait`
- `TaskPlanTrait`
- `TaskBunch`
- `Cost`
- `MutableCost`
- `BunchCostPolicy`
- `DefaultBunchCostPolicy`
- `FunctionalBunchCostPolicy`
- `SchedulingSolverValueAdapter`
- `SolverValueAdapter`
- `F64SolverValueAdapter`
- `TaskStepGraph`
- `TaskStepGraphBuilder`

## 相关目录

- [`../task_compilation`](../task_compilation/README_ch.md)
- [`../bunch_generation`](../bunch_generation/README_ch.md)
- [`../produce`](../produce/README_ch.md)
