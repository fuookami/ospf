# Task Domain

[中文](README_ch.md)

This directory contains the core task-side domain model. It maps Kotlin `gantt-scheduling-domain-task-context`.

## Responsibilities

- Model executors, tasks, task plans, task bunches, assignments, and costs.
- Provide task-step graph structures for multi-step task workflows.
- Expose solver value adapters and shadow-price keys for downstream compilation layers.

## Modules

- `assignment.rs`: assignment policy abstractions.
- `cost.rs`: cost representation and mutable cost accumulation.
- `cost_policy.rs`: bunch cost policies and functional cost policies.
- `executor.rs`: executor traits and base executor types.
- `scheduling_solver_value_adapter.rs`: solver value adapter abstractions.
- `shadow_price.rs`: shadow-price argument and key types.
- `task_bunch.rs`: task bunch model.
- `task_plan.rs`: task plan abstractions and statuses.
- `task_step_graph.rs`: task step graph and step relations.
- `task_trait.rs`: task trait definitions.

## Public API

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

## Related Directories

- [`../task_compilation`](../task_compilation/README.md)
- [`../bunch_generation`](../bunch_generation/README.md)
- [`../produce`](../produce/README.md)
