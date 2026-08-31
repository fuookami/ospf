# BPP3D Packing Context

:us: English | :cn: [简体中文](README_ch.md)

This context maps Kotlin `bpp3d-domain-packing-context`.

## Responsibilities

Packing converts selected layers and known placements into final packed bins, validates geometry, summarizes material usage, and adapts results to render DTOs. It is the final geometry gate after layer assignment.

## File Layout

- `model.rs` defines packed items, packed bins, material summaries, material plans, and package-solution adapters.
- `service.rs` is the public service shim.
- `service/geometry_guard.rs` validates cuboid/cylinder overlap, bounds, and horizontal-cylinder support.
- `service/layer_placement_adapter.rs` and `layer_trace_replay_adapter.rs` convert generated traces into known-coordinate packed bins.
- `service/packer.rs`, `material_packer.rs`, and `renderer_adapter.rs` produce final summaries and render output.

## Public API

- `PackedItem`
- `PackedBin`
- `MaterialSummary`
- `MaterialPackingPlan`
- `PackingGeometryGuard`
- `PackingGeometryContract`
- `Packer`
- `PackingResult`
- `PackingAggregation`
- `MaterialPacker`
- `PackingRendererAdapter`
- `KnownCoordinatePlacement`
- `LayerPlacementAdapter`
- `LayerTraceReplayAdapter`

## Extension Points

Add final validation rules to the geometry guard. Add new trace replay paths in adapters. Keep candidate generation in `layer_generation` and only reject/convert final known-coordinate placements here.

## Lifecycle and Data Flow

Selected layer assignments and known-coordinate placements are replayed into packed items, geometry guards reject infeasible overlap or support states, material packers summarize usage, and render adapters produce DTO-ready loading plans.

## Verification

Use `cargo test -p ospf-rust-framework-bpp3d --lib`, especially packing geometry and render adapter tests.

## Related Directories

- [`../item`](../item/README.md)
- [`../layer_assignment`](../layer_assignment/README.md)
- [`../../infrastructure`](../../infrastructure/README.md)
