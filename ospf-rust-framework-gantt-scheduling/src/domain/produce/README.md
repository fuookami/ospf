# Produce Domain

:us: English | :cn: [简体中文](README_ch.md)

This directory models material production and consumption, mapped from Kotlin `gantt-scheduling-domain-produce-context`.

## Responsibilities

- Model material traits, products, semi-products, raw materials, demand, and reserves.
- Model production and consumption usages for `MetaModel` registration.
- Provide quantity-based production and consumption objectives and constraints.

## Modules

- `model/`: demand, material, production task, and usage structures.
- `service/limits.rs`: production and consumption quantity constraints and objectives.

## Public API

- `MaterialTrait`
- `Product`
- `SemiProduct`
- `RawMaterial`
- `MaterialDemand`
- `MaterialReserves`
- `ProductionTaskTrait`
- `ProduceUsage`
- `ConsumptionUsage`
- `ProduceQuantityConstraint`
- `ConsumptionQuantityConstraint`
- `ProduceOverQuantityMinimization`
- `ProduceLessQuantityMinimization`
- `ProduceQuantityMaximization`
- `ProduceQuantityMinimization`
- `ConsumptionOverQuantityMinimization`
- `ConsumptionLessQuantityMinimization`
- `ConsumptionQuantityMaximization`
- `ConsumptionQuantityMinimization`

## Extension Points

Extend material flow through `MaterialTrait`, `ProductionTaskTrait`, produce/consumption usage components, and quantity-based limit pipelines. Keep resource-capacity logic in `resource` and executor/time allocation in `capacity_scheduling`.

## Lifecycle and Data Flow

Material demand and reserve models define required and available quantities, production tasks expose produce and consumption usages, limit pipelines register quantity constraints or objectives, and downstream scheduling contexts combine these expressions with capacity and task decisions.

## Verification

Use `cargo test -p ospf-rust-framework-gantt-scheduling --lib` when changing material models, usage components, or produce/consumption quantity limits.

## Related Directories

- [`../resource`](../resource/README.md)
- [`../task`](../task/README.md)
- [`../capacity_scheduling`](../capacity_scheduling/README.md)
