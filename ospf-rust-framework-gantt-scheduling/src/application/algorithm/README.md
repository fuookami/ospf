# Application Algorithms

[中文](README_ch.md)

This directory contains application-level solving algorithms. The algorithms coordinate model lifecycle, pricing, branching, and policy decisions, while domain modules still own concrete model registration.

## Modules

- `policy.rs`: reusable column generation policy configuration.
- `task_column_generation.rs`: task-level column generation algorithm.
- `bunch_column_generation.rs`: bunch-level column generation and branch-and-price adapter.
- `branch_and_price.rs`: branch-and-price tree search, branching decisions, node callbacks, cut callbacks, and strong branching extension points.

## Public API

- `ColumnGenerationPolicy`
- `TaskColumnGenerationAlgorithm`
- `BunchBranchAndPriceAlgorithm`
- `BunchCGPolicy`
- `BranchAndPriceTreeSearch`
- `BranchSearchConfig`
- `BranchSearchHooks`
- `StrongBranchingStrategy`
- `BranchCutCallback`
- `BranchNodeCallback`

## Design Notes

Algorithms in this directory should orchestrate existing contexts and services. They may manage lifecycle snapshots, warm-start state, node search order, and callback dispatch, but domain-specific constraints and objectives should remain in `src/domain`.

## Related Directories

- [`../service`](../service/README.md)
- [`../../domain/bunch_compilation`](../../domain/bunch_compilation/README.md)
- [`../../domain/bunch_generation`](../../domain/bunch_generation/README.md)
- [`../../domain/task_compilation`](../../domain/task_compilation/README.md)
