# Capacity Scheduling

:us: English | :cn: [简体中文](README_ch.md)

This directory contains capacity scheduling model components and pipelines. It maps the Kotlin `gantt-scheduling-domain-capacity-scheduling-context` module.

## Responsibilities

- Model production actions and executor-slot allocations.
- Register unordered and ordered capacity compilation variables.
- Extract capacity scheduling solutions from solver-order values.
- Maintain capacity columns for iterative and column-generation workflows.
- Provide capacity constraints and capacity-cost objectives.

## Modules

- `model.rs`: production actions, capacity compilation, ordered capacity compilation, capacity columns, aggregation, and solution extraction.
- `service/limits.rs`: executor capacity constraints, order constraints, and capacity cost minimization.
- `service/mod.rs`: service-level re-exports.

## Public API

- `ProductionActionTrait`
- `BasicProductionAction`
- `CapacityCompilation`
- `CapacityOrderCompilation`
- `CapacityColumn`
- `CapacityColumnAggregation`
- `CapacitySchedulingSolution`
- `ActionAllocation`
- `ExecutorCapacityResult`
- `ExecutorCapacityConstraint`
- `OrderConstraint`
- `CapacityCostMinimization`
- `CapacitySchedulingAggregation`
- `CapacitySchedulingContext`

## Extension Points

Extend capacity scheduling through `ProductionActionTrait`, capacity columns, ordered compilation, and limit pipelines such as executor capacity, order constraints, and capacity-cost objectives. Keep resource vocabulary in `resource` and material flow vocabulary in `produce`.

## Lifecycle and Data Flow

Production actions and executor-slot candidates are compiled into capacity variables, optional ordered compilation adds sequence-sensitive variables, limit pipelines register capacity constraints and cost objectives, and solution extraction produces action allocations plus executor capacity results.

## Verification

Use `cargo test -p ospf-rust-framework-gantt-scheduling --lib` when changing capacity columns, ordered compilation, capacity limits, or solution extraction.

## Related Directories

- [`../produce`](../produce/README.md)
- [`../resource`](../resource/README.md)
- [`../bunch_generation`](../bunch_generation/README.md)
