# 任务领域

:us: [English](README.md) | :cn: 简体中文

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

## 扩展点

任务语义通过 executor trait、assignment policy、task trait、task-plan trait、cost policy、solver value adapter 和 task-step graph builder 扩展。模型变量注册保留在 `task_compilation`。

## 生命周期与数据流

task 与 executor 模型定义可排程工作，assignment 和 cost policy 提供业务决策，solver value adapter 标准化 solver 输出，shadow-price key 连接 compilation 与 pricing，task-step graph 描述多步骤流程依赖。

## 验证

修改 task vocabulary、assignment behavior、cost policy、solver value adapter 或 task-step graph construction 时运行 `cargo test -p ospf-rust-framework-gantt-scheduling --lib`。

## 相关目录

- [`../task_compilation`](../task_compilation/README_ch.md)
- [`../bunch_generation`](../bunch_generation/README_ch.md)
- [`../produce`](../produce/README_ch.md)
