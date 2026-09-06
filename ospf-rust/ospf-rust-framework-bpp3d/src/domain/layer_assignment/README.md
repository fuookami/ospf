# BPP3D Layer Assignment Context

:us: English | :cn: [简体中文](README_ch.md)

This context maps Kotlin `bpp3d-domain-layer-assignment-context`.

## Responsibilities

Layer assignment owns RMP and final MILP assignment components, demand load expressions, capacity expressions, dynamic layer-column lifecycle, constraints, objectives, and shadow-price extraction.

## File Layout

- `model.rs` is the public modeling helper shim; variable arrays, expression arrays, solution extraction, and component traits live under `model/`.
- `service/mod.rs` is the public service shim; value adapters, assignment models, load/capacity models, iterative context, aggregation, and context registration live in sibling files.
- `service/limits.rs` is the public limits shim; each Kotlin-style limit/objective lives under `service/limits/`.

## Public API

- `LayerAssignmentContext`
- `IterativeLayerAssignmentContext`
- `LayerAssignmentAggregation`
- `LayerAggregation`
- `Bpp3dModelComponent`
- `Bpp3dSolverValueAdapter`
- `DemandShadowPriceKey`
- `DemandConstraint`
- `BinCapacityConstraint`
- `BinDepthConstraint`
- `BinAmountMinimization`
- `VolumeMinimization`
- `BetterLayerMaximization`

## Extension Points

Add new constraints or objectives as `Pipeline<MetaModel<f64>>` implementations under `service/limits/`. Keep add/remove/hide/fix/flush column lifecycle behavior in `IterativeLayerAssignmentContext`.

## Lifecycle and Data Flow

Initial layer columns register variables and expressions into `MetaModel`; dynamic column operations add, hide, fix, remove, and flush layer columns; RMP solutions refresh shadow prices; final MILP solutions are extracted into selected layer assignments.

## Verification

Use `cargo test -p ospf-rust-framework-bpp3d --lib`; for solver integration, also run serde feature checks and backend `--no-run` builds.

## Related Directories

- [`../item`](../item/README.md)
- [`../layer_generation`](../layer_generation/README.md)
- [`../packing`](../packing/README.md)
