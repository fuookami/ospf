# Bunch Compilation

:us: English | :cn: [简体中文](README_ch.md)

This directory contains the master-problem side of task bunch compilation. It maps Kotlin `gantt-scheduling-domain-bunch-compilation-context`.

## Responsibilities

- Register task bunch decision variables and aggregation expressions into `MetaModel`.
- Maintain iterative bunch columns for column generation workflows.
- Extract task and bunch scheduling solutions from solver results.
- Provide slot-based compilation and capacity pre-solving helpers.
- Expose shadow-price pipelines used by pricing subproblems.

## Modules

- `model.rs`: bunch compilation, bunch aggregation, entries, solution, and solution summary.
- `context.rs`: basic and iterative bunch compilation contexts.
- `iterative.rs`: iterative bunch compilation state and column operations.
- `slot_based.rs`: slot-based bunch compilation context and capacity pre-solver.
- `service.rs`: solution analysis and service helpers.

## Public API

- `BunchCompilation`
- `BunchAggregation`
- `BunchEntry`
- `BunchSolution`
- `BunchSolutionSummary`
- `IterativeBunchCompilation`
- `BasicBunchCompilationContext`
- `IterativeBunchCompilationContext`
- `BunchShadowPricePipeline`
- `TaskShadowPriceKey`
- `SlotBasedBunchCompilationContext`
- `BasicSlotBasedBunchCompilationContext`
- `SlotBasedCapacityPreSolver`
- `StaticSlotBasedCapacityPreSolver`

## Extension Points

Add master-problem behavior through bunch compilation contexts, iterative bunch-column operations, slot-based capacity pre-solvers, and shadow-price pipelines. Keep pricing rules in `bunch_generation` and task-level variable semantics in `task_compilation`.

## Lifecycle and Data Flow

Compilation contexts register initial bunch columns and aggregation expressions into `MetaModel`, iterative contexts add or refresh columns during column generation, shadow-price pipelines expose dual values to pricing, and solution analyzers extract task and bunch schedules from final solver values.

## Verification

Use `cargo test -p ospf-rust-framework-gantt-scheduling --lib` when changing bunch compilation, shadow-price extraction, slot-based pre-solving, or solution analysis.

## Related Directories

- [`../bunch_generation`](../bunch_generation/README.md)
- [`../task`](../task/README.md)
- [`../../application/algorithm`](../../application/algorithm/README.md)
