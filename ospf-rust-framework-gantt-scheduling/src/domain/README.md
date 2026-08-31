# Domain Layer

:us: English | :cn: [简体中文](README_ch.md)

This directory contains the domain modeling layer for the Gantt scheduling framework. It maps Kotlin Gantt Scheduling domain submodules into Rust modules organized around contexts, aggregations, model components, and pipelines.

## Responsibilities

- Define task, executor, resource, material, capacity, and bunch abstractions.
- Register variables, intermediate symbols, constraints, and objectives into `MetaModel`.
- Provide reusable model components for MILP, column generation LP, and final MILP workflows.
- Expose extension points for cost policies, feasibility policies, shadow prices, and dynamic model lifecycle.

## Modules

- [`task`](task/README.md): task, executor, assignment, task plans, task bunches, costs, solver value adapters, and shadow-price keys.
- [`task_compilation`](task_compilation/README.md): task compilation model components, time variables, switch variables, makespan, solution analysis, and limits.
- [`task_generation`](task_generation/README.md): reserved task-generation extension point mapped from Kotlin.
- [`bunch_generation`](bunch_generation/README.md): pricing graph, label-setting algorithm, and bunch generators.
- [`bunch_compilation`](bunch_compilation/README.md): column-generation master problem components for task bunches.
- [`capacity_scheduling`](capacity_scheduling/README.md): capacity actions, capacity compilation, ordered capacity compilation, capacity columns, and limits.
- [`produce`](produce/README.md): material demand, reserves, production tasks, produce usage, consumption usage, and quantity objectives.
- [`resource`](resource/README.md): execution, storage, connection resources, resource usage, capacity, slack, and resource limits.
- [`common`](common/README.md): shared constraint indexes and dynamic model lifecycle re-exports.

## Public API

- `task`
- `task_compilation`
- `task_generation`
- `bunch_generation`
- `bunch_compilation`
- `capacity_scheduling`
- `produce`
- `resource`
- `common`

## Extension Points

Domain modules should own optimization semantics. Application code may compose them, but should not duplicate variable registration, constraint construction, or solution extraction logic.

## Lifecycle and Data Flow

Task, resource, and produce models define shared vocabulary; compilation contexts register master-model variables and expressions; generation contexts create pricing columns; capacity and bunch contexts connect task flows to solver iterations; common helpers keep dynamic model state and constraint indexes reusable.

## Verification

Use `cargo test -p ospf-rust-framework-gantt-scheduling --lib` for domain coverage. Add focused tests when changing context registration, pipeline constraints, or solution extraction.

## Related Directories

- [`../application`](../application/README.md)
- [`../infrastructure`](../infrastructure/README.md)
