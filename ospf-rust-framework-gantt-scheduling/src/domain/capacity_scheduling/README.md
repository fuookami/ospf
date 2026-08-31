# Capacity Scheduling

[中文](README_ch.md)

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

## Related Directories

- [`../produce`](../produce/README.md)
- [`../resource`](../resource/README.md)
- [`../bunch_generation`](../bunch_generation/README.md)
