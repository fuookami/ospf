# Capacity Scheduling

:us: English | :cn: [简体中文](README_ch.md)

This directory contains capacity scheduling model components and pipelines. It maps the Kotlin `gantt-scheduling-domain-capacity-scheduling-context` module.

## Responsibilities

- Model production actions and executor-slot allocations.
- Register unordered and ordered capacity compilation variables.
- Extract capacity scheduling solutions from solver-order values.
- Maintain capacity columns for iterative and column-generation workflows.
- Rebuild active column terms after columns are added or removed.
- Provide capacity constraints and capacity-cost objectives.

## Modules

- `model.rs`: production actions, capacity compilation, ordered capacity compilation, capacity columns, aggregation, and solution extraction.
- `iterative.rs`: stable per-column variables, active-column snapshots, add/remove lifecycle, and solution extraction for column generation.
- `service/limits.rs`: executor capacity constraints, active capacity-column selection constraints, order constraints, and capacity cost minimization.
- `service/mod.rs`: service-level re-exports.

## Public API

- `ProductionActionTrait`
- `BasicProductionAction`
- `CapacityCompilation`
- `CapacityOrderCompilation`
- `CapacityColumn`
- `CapacityColumnAggregation`
- `IterativeCapacityColumn`
- `IterativeCapacityCompilation`
- `CapacitySchedulingSolution`
- `ActionAllocation`
- `ExecutorCapacityResult`
- `ExecutorCapacityConstraint`
- `CapacityColumnSelectionConstraint`
- `OrderConstraint`
- `CapacityCostMinimization`
- `CapacitySchedulingAggregation`
- `CapacitySchedulingContext`

## Extension Points

Extend capacity scheduling through `ProductionActionTrait`, capacity columns, ordered compilation, and limit pipelines such as executor capacity, order constraints, and capacity-cost objectives. Keep resource vocabulary in `resource` and material flow vocabulary in `produce`.

## Lifecycle and Data Flow

Production actions and executor-slot candidates are compiled into capacity variables, optional ordered compilation adds sequence-sensitive variables, and iterative compilation registers a stable variable for every generated column. After `add_columns` or `remove_columns`, consume `active_column_variables` or construct `CapacityColumnSelectionConstraint::from_iterative_compilation` to rebuild active terms. Limit pipelines register capacity constraints and cost objectives, and solution extraction produces action allocations plus executor capacity results.

## Verification

Use `cargo test -p ospf-rust-framework-gantt-scheduling --lib` when changing capacity columns, ordered compilation, capacity limits, or solution extraction.

## Related Directories

- [`../produce`](../produce/README.md)
- [`../resource`](../resource/README.md)
- [`../bunch_generation`](../bunch_generation/README.md)
