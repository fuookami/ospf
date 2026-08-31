# Produce Domain

[中文](README_ch.md)

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

## Related Directories

- [`../resource`](../resource/README.md)
- [`../task`](../task/README.md)
- [`../capacity_scheduling`](../capacity_scheduling/README.md)
