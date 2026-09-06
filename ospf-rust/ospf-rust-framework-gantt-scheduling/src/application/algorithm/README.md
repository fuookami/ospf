# Application Algorithms

:us: English | :cn: [简体中文](README_ch.md)

This directory contains application-level solving algorithms. The algorithms coordinate model lifecycle, pricing, branching, and policy decisions, while domain modules still own concrete model registration.

## Responsibilities

- Coordinate task and bunch model lifecycle without owning domain expressions.
- Run column-generation iterations, branch-and-price search, callbacks, and optional strong branching.
- Keep search policy, hooks, and iteration state close to the application boundary.

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

## Extension Points

Algorithms in this directory should orchestrate existing contexts and services. They may manage lifecycle snapshots, warm-start state, node search order, and callback dispatch, but domain-specific constraints and objectives should remain in `src/domain`.

## Lifecycle and Data Flow

Column-generation algorithms build or refresh master models, ask pricing services for new columns, record iteration snapshots, and stop when policy thresholds are met. Branch-and-price wraps the bunch column-generation flow with node selection, branching decisions, cuts, and callback hooks.

## Verification

Use `cargo test -p ospf-rust-framework-gantt-scheduling --lib` and focus on task column generation, bunch column generation, and branch-and-price tests.

## Related Directories

- [`../service`](../service/README.md)
- [`../../domain/bunch_compilation`](../../domain/bunch_compilation/README.md)
- [`../../domain/bunch_generation`](../../domain/bunch_generation/README.md)
- [`../../domain/task_compilation`](../../domain/task_compilation/README.md)
