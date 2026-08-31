# Common Domain Helpers

[中文](README_ch.md)

This directory holds shared helpers used across Gantt subdomains.

## Responsibilities

- Provide shared constraint index types.
- Re-export dynamic model lifecycle helpers from `ospf_rust_framework::model`.
- Keep cross-domain state helpers close to the domain layer without duplicating them.

## Modules

- `constraint_index.rs`: constraint key, entry, and map helpers.

## Public API

- `ConstraintIndexEntry`
- `ConstraintIndexKey`
- `ConstraintIndexMap`
- `GanttDynamicModelLifecycle`
- `GanttDynamicModelSnapshot`
- `GanttModelStateFacade`

## Related Directories

- [`../task`](../task/README.md)
- [`../task_compilation`](../task_compilation/README.md)
- [`../../application`](../../application/README.md)
