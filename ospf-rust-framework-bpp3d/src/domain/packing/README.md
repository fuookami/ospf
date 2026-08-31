# BPP3D Packing Context

[中文](README_ch.md)

This context maps Kotlin `bpp3d-domain-packing-context`.

## Purpose

Packing converts selected layers and known placements into final packed bins, validates geometry, summarizes material usage, and adapts results to render DTOs. It is the final geometry gate after layer assignment.

## File Layout

- `model.rs` defines packed items, packed bins, material summaries, material plans, and package-solution adapters.
- `service.rs` is the public service shim.
- `service/geometry_guard.rs` validates cuboid/cylinder overlap, bounds, and horizontal-cylinder support.
- `service/layer_placement_adapter.rs` and `layer_trace_replay_adapter.rs` convert generated traces into known-coordinate packed bins.
- `service/packer.rs`, `material_packer.rs`, and `renderer_adapter.rs` produce final summaries and render output.

## Extension Points

Add final validation rules to the geometry guard. Add new trace replay paths in adapters. Keep candidate generation in `layer_generation` and only reject/convert final known-coordinate placements here.

## Verification

Use `cargo test -p ospf-rust-framework-bpp3d --lib`, especially packing geometry and render adapter tests.
