# Bunch Compilation

[中文](README_ch.md)

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

## Related Directories

- [`../bunch_generation`](../bunch_generation/README.md)
- [`../task`](../task/README.md)
- [`../../application/algorithm`](../../application/algorithm/README.md)
