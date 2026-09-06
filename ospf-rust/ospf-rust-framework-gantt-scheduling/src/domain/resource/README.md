# Resource Domain

:us: English | :cn: [简体中文](README_ch.md)

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

## Extension Points

Add resource semantics through the resource traits, concrete execution/storage/connection resource models, resource usage components, slack components, and resource quantity limit pipelines. Keep material production/consumption in `produce`.

## Lifecycle and Data Flow

Resource definitions provide capacity and identity, usage components register resource consumption into `MetaModel`, slack components represent controlled violations, and limit pipelines constrain or optimize resource quantities.

## Verification

Use `cargo test -p ospf-rust-framework-gantt-scheduling --lib` when changing resource traits, usage/slack components, capacity constraints, or resource quantity objectives.

## Related Directories

- [`../produce`](../produce/README.md)
- [`../task`](../task/README.md)
- [`../../application`](../../application/README.md)
