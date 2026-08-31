# Resource Domain

[中文](README_ch.md)

This directory models execution, storage, and connection resources. It maps Kotlin `gantt-scheduling-domain-resource-context`.

## Responsibilities

- Define resource traits and concrete resource types.
- Define resource usage, slack, and capacity model components.
- Provide resource capacity constraints and quantity objectives.

## Modules

- `model/`: resource traits, capacities, usages, and slack components.
- `service/limits.rs`: resource capacity constraints and quantity objectives.

## Public API

- `ResourceCapacity`
- `ResourceSlack`
- `ResourceTrait`
- `ExecutionResourceTrait`
- `StorageResourceTrait`
- `ConnectionResourceTrait`
- `BasicExecutionResource`
- `BasicStorageResource`
- `BasicConnectionResource`
- `ResourceUsage`
- `StorageResourceUsage`
- `ConnectionResourceUsage`
- `ResourceCapacityConstraint`
- `ResourceOverQuantityMinimization`
- `ResourceLessQuantityMinimization`

## Related Directories

- [`../produce`](../produce/README.md)
- [`../task`](../task/README.md)
- [`../../application`](../../application/README.md)
