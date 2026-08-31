# OSPF Rust Framework Gantt Scheduling

:us: English | :cn: [简体中文](README_ch.md)

## Introduction

`ospf-rust-framework-gantt-scheduling` is the Rust migration target for Kotlin `ospf-kotlin-framework-gantt-scheduling`. It provides reusable Gantt scheduling domain framework foundations, task and bunch compilation, resource/produce/capacity contexts, pricing graph and label search, iterative column lifecycle, isolated branch-and-price tree search, and a small final-MILP branch-and-price flow covered by tests.

## Scope

This crate owns reusable scheduling kernels for task modeling, task compilation, bunch compilation, bunch generation, capacity scheduling, resource constraints, produce/consumption tracking, time infrastructure, and application-level branch-and-price orchestration.

Explicit non-goals:

1. Business-specific request DTOs, tenant context, formula languages, and project runtime policies.
2. Solver backend installation, license management, or backend plugin ownership.
3. Concurrent tree execution and solver-native node callbacks; the crate exposes hooks for downstream integration instead.

## Module Structure

| Rust module or directory | Kotlin boundary | Responsibility |
| --- | --- | --- |
| [`src/infrastructure`](src/infrastructure/README.md) | `gantt-scheduling-infrastructure` | Time ranges, windows, slots, duration ranges, working calendars, calendar policies, local date offsets, and render DTOs. |
| [`src/domain/task`](src/domain/task/README.md) | `gantt-scheduling-domain-task-context` | Task, executor, assignment, task plan, task bunch, cost, solver value adapter, task-step graph, and shadow-price keys. |
| [`src/domain/task_compilation`](src/domain/task_compilation/README.md) | `gantt-scheduling-domain-task-compilation-context` | Task-level MILP components, time variables, switches, makespan, solution analysis, and limit/objective pipelines. |
| [`src/domain/task_generation`](src/domain/task_generation/README.md) | `gantt-scheduling-domain-task-generation-context` | Reserved task-generation extension point mapped from Kotlin. |
| [`src/domain/bunch_compilation`](src/domain/bunch_compilation/README.md) | `gantt-scheduling-domain-bunch-compilation-context` | Column-generation master problem for task bunches, slot-based compilation, iterative columns, and bunch solutions. |
| [`src/domain/bunch_generation`](src/domain/bunch_generation/README.md) | `gantt-scheduling-domain-bunch-generation-context` | Pricing graph, label-setting search, feasibility policies, and slot-based bunch generation. |
| [`src/domain/capacity_scheduling`](src/domain/capacity_scheduling/README.md) | `gantt-scheduling-domain-capacity-scheduling-context` | Capacity actions, capacity/order compilation, capacity columns, action bounds, and capacity solutions. |
| [`src/domain/resource`](src/domain/resource/README.md) | `gantt-scheduling-domain-resource-context` | Execution/storage/connection resources, resource usage, capacity, slack, and resource limits. |
| [`src/domain/produce`](src/domain/produce/README.md) | `gantt-scheduling-domain-produce-context` | Material demand, reserve, production task, produce usage, consumption usage, and quantity objectives. |
| [`src/domain/common`](src/domain/common/README.md) | shared Gantt domain helpers | Constraint indexes and dynamic model lifecycle compatibility aliases. |
| [`src/application`](src/application/README.md) | `gantt-scheduling-application` | APS/MPS/LSP markers, column generation, branch-and-price algorithms, service constructors, hooks, and iteration state. |

## Architecture Overview

The crate follows the context / aggregation / pipeline architecture:

1. Domain contexts define reusable scheduling entities and register variables, intermediate values, constraints, objectives, and extraction logic into `MetaModel`.
2. Bunch compilation is the column-generation master problem; bunch generation is the pricing problem.
3. Task compilation, capacity scheduling, resource, and produce contexts provide reusable MILP components and optional constraint/objective families.
4. Application algorithms coordinate LP/MILP stages, column lifecycle, branch decisions, hooks, and state restoration.

Ordinary MILP, RMP LP, and final MILP share context and iterative compilation entry points for registration, column addition, shadow-price extraction, and solution extraction where possible.

## Core Concepts

1. A task is a schedulable work unit with executor, time, duration, status, and optional multi-step dependency graph.
2. A task bunch is an ordered route/column assigned to an executor.
3. Bunch compilation selects columns in the master problem.
4. Bunch generation produces improving columns through pricing graph and label search.
5. Dynamic model lifecycle tracks warm-start state, column ranges, hidden/fixed/removed columns, and solver solutions.

## Public API

| API | Responsibility | Stability |
| --- | --- | --- |
| `domain::task::{TaskTrait, ExecutorTrait, AssignmentPolicyTrait, TaskPlanTrait}` | Core task and assignment abstractions. | migration |
| `domain::common::{GanttId, ExecutorIdTrait, TaskIdTrait, TaskPlanIdTrait}` | Strongly typed business ID contracts with default string newtypes. | migration |
| `domain::task::{TaskStepGraph, TaskStepTrait, BasicTaskStep, StepRelation}` | Multi-step task dependency model. | migration |
| `domain::task::{Cost, BunchCostPolicy, CostBreakdown, DefaultBunchCostPolicy}` | Cost and reduced-cost policy surface. | migration |
| `domain::task::{SolverValueAdapter, F64SolverValueAdapter}` | Generic solver value conversion and `f64` boundary. | migration |
| `domain::task_compilation::{BasicTaskCompilationContext, IterativeTaskCompilationContext, Switch, SwitchCostMinimization, SwitchTimeMinimization}` | Task compilation contexts and switch objective pipelines. | migration |
| `domain::capacity_scheduling::{CapacityCompilation, CapacityOrderCompilation, CapacityColumn, CapacityColumnAggregation, CapacitySchedulingSolution}` | Capacity scheduling registration and extraction. | migration |
| `domain::bunch_compilation::{BasicBunchCompilationContext, IterativeBunchCompilationContext, BasicSlotBasedBunchCompilationContext, SlotBasedBunchCompilationContext, SlotBasedCapacityPreSolver, BunchEntry, BunchSolution}` | Bunch master problem and slot-based column lifecycle. | migration |
| `domain::bunch_generation::{SlotBasedBunchGenerator, BunchFeasibilityPolicy, BunchTaskCandidate, CapacityIntermediateValues}` | Pricing and feasibility extension surface. | migration |
| `application::service::{create_bunch_branch_and_price, create_slot_bunch_branch_and_price, search_bunch_branch_and_price_with_fresh_model, search_bunch_branch_and_price_with_hooks}` | Application helper constructors, slot capacity pre-solving, and search entries. | migration |
| `application::algorithm::{BranchAndPriceTreeSearch, StrongBranchingStrategy, BranchCutCallback, BranchNodeCallback}` | Isolated branch-and-price tree search hooks. | migration |
| `domain::common::GanttDynamicModelLifecycle` | Compatibility alias over shared framework dynamic lifecycle. | migration |
| `infrastructure::{CalendarPolicy, CompositeCalendarPolicy}` | Calendar extension policy surface. | migration |

## Modeling Extensions

Extension points include:

1. `domain::bunch_generation::BunchFeasibilityPolicy` for task, resource, produce, capacity, and business feasibility rules.
2. `domain::task::BunchCostPolicy` for task, executor, connection, capacity, and soft-constraint cost formulas.
3. `infrastructure::CalendarPolicy` for complex shift rules, unavailable ranges, connection windows, and breaks.
4. `domain::task::TaskStepGraph` for validated multi-step task dependencies.
5. `domain::task_compilation::Switch` objective pipelines for static-time and optional dynamic `TaskTime` switch paths.
6. `domain::capacity_scheduling::CapacityCompilation` and `CapacityOrderCompilation` for unordered/ordered capacity extraction.
7. `domain::bunch_compilation::SlotBasedBunchCompilationContext` for capacity pre-solving, slot constraint lookup, slot-wise column addition, and slot-wise bunch queries.
8. `domain::common::ConstraintIndexMap` for stable shadow-price extraction.
9. `application::algorithm::BranchAndPriceTreeSearch` hooks for strong branching, cuts, and node tracing.

Slot-aware column-generation additions include `ExecutorSlotCompilationConstraint` for
exactly-one selection per `(executor, slot)`, typed executor-slot dual extraction through
`ConstraintIndexMap`, and `BunchPricingRequest` for slot duals, branch groups, and minimum
column quotas. `BranchGroupTracker` keeps an executor active until all known slot groups
are fixed. `SlotBunchPricingRequest` carries slot entry state and branch restrictions into
slot-pricing policies, while `CapacityColumnSelectionConstraint` registers and extracts
exactly-one capacity-column selections.

## Generic Numeric Boundaries

Domain APIs use generic solver-value abstractions through `SolverValueAdapter`. `F64SolverValueAdapter` marks the current `f64` solver boundary. Solver conversion remains concentrated in context registration, application solver calls, and result extraction rather than scattered through domain logic.

## Physical Quantity Boundaries

Time, duration, capacity, resource amount, production amount, consumption amount, slack, and demand satisfaction should use infrastructure time types, `ospf-rust-quantities` quantities, or explicit domain wrappers. Bare `f64` values are limited to solver adapter, registration, extraction, and low-level coefficient boundaries.

## Solve Lifecycle

The tested slot branch-and-price application flow is:

1. Pre-solve capacity and capture `CapacityIntermediateValues` in the bunch-generation policy.
2. Build a fresh `MetaModel<f64>` per branch node.
3. Register the bunch compilation context.
4. Solve the initial MILP.
5. Solve the RMP LP and extract shadow prices through the context.
6. Generate slot bunches through `BunchCGPolicy` and register them with `add_columns`.
7. Solve the final MILP and extract `BunchSolution` through the shared context.
8. Restore application and context state before moving to the next tree node.

For multi-node tree search, prefer `solve_branch_node_with_fresh_model`, which restores application state, the dynamic lifecycle, and the compilation context while discarding the node-local `MetaModel` after solve.

## Outputs

`BunchSolution` and related bunch scheduling outputs contain selected bunches, task assignments, canceled tasks, executor assignments, and total cost. `CapacitySchedulingSolution` contains capacity columns and production actions per time slot. Iteration and branch-search outputs record LP/IP objectives, node status, branch decisions, incumbent state, and trace hooks.

## Usage

```rust,ignore
use ospf_rust_framework_gantt_scheduling::application::service::{
    create_bunch_branch_and_price,
    search_bunch_branch_and_price_with_fresh_model,
};

let algorithm = create_bunch_branch_and_price(config);
let result = search_bunch_branch_and_price_with_fresh_model(algorithm, input)?;
```

## Local Validation

```powershell
cargo check -p ospf-rust-framework-gantt-scheduling
cargo test -p ospf-rust-framework-gantt-scheduling
cargo check -p ospf-rust-framework-gantt-scheduling --features serde
```

## Current Boundaries

1. Ordinary MILP, RMP LP, and final MILP share the context and iterative compilation entry points for registration, column addition, shadow-price extraction, and solution extraction. Solver conversion remains in the application layer.
2. Multi-step task graphs are exposed as a domain extension model. Existing single-step task compilation remains unchanged.
3. Slot-based bunch compilation currently uses the shared `MetaModel<f64>` solver boundary and a pluggable capacity pre-solver trait.
4. Capacity scheduling and task switch registration use shared core variable-range, solution, function-symbol, and objective-pipeline interfaces instead of Gantt-local lifecycle shims.
5. Native solver warm start remains an adapter capability. The shared lifecycle writes cached solver-order solutions into `MetaModel`; Gurobi adapters already map token results to native `Start`.
6. Concurrent tree execution and solver-native node callbacks remain outside this crate, but branch-and-price search exposes strong-branching, cut, and node callback traits for downstream integration.
7. Complex shift calendars and custom cost formulas are supported through standard policy extension points and covered by minimal tests.

The migration target, checklist, and acceptance criteria should stay aligned with the current-boundaries list above and the Kotlin Gantt Scheduling README.

## Related Modules

- [Root README](../README.md)
- [Gantt application README](src/application/README.md)
- [Gantt domain README](src/domain/README.md)
- [Kotlin Gantt Scheduling README](../../ospf-kotlin/ospf-kotlin-framework-gantt-scheduling/README.md)
