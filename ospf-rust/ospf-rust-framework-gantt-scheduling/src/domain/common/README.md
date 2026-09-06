# Common Domain Helpers

:us: English | :cn: [简体中文](README_ch.md)

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

## Extension Points

Use `ConstraintIndexMap` when a context needs stable access to generated constraint tokens. Use the dynamic model lifecycle re-exports when iterative contexts need snapshots, restoration, or state facades shared across Gantt subdomains.

## Lifecycle and Data Flow

Domain contexts populate constraint indexes during model registration, iterative algorithms read those indexes for shadow prices or diagnostics, and dynamic lifecycle helpers keep model state transitions reusable across task, bunch, and capacity contexts.

## Verification

Use `cargo test -p ospf-rust-framework-gantt-scheduling --lib` when changing constraint index behavior or dynamic model lifecycle integration.

## Related Directories

- [`../task`](../task/README.md)
- [`../task_compilation`](../task_compilation/README.md)
- [`../../application`](../../application/README.md)
