# BPP3D Infrastructure

[中文](README_ch.md)

This context maps Kotlin `bpp3d-infrastructure`.

## Purpose

Infrastructure contains typed geometry, orientation, packing-shape, PWL approximation, and render DTO adapters shared by domain and application code. It is the boundary where Rust keeps physical quantities and typed geometry while providing scalar adapters for solver or rendering edges.

## File Layout

- `geometry.rs` exposes typed points, vectors, sizes, AABBs, placements, and scalar conversion helpers; fragments live under `geometry/`.
- `orientation.rs` defines orientation categories and rotation semantics.
- `packing_shape.rs` defines cuboid/cylinder packing shapes and axis-aware bounding dimensions.
- `pwl_approximation.rs` contains radius and radius-squared PWL support used by continuous cylinder radius modeling.
- `renderer.rs` contains render DTOs and shape/axis enums for application output.

## Extension Points

Add new geometry adapters here when they are shared across contexts. Keep raw `f64` conversion at solver/render boundaries, and keep domain-facing APIs typed with `Quantity<V, U>` and unit traits.

## Verification

Use regular crate checks plus focused infrastructure tests through `cargo test -p ospf-rust-framework-bpp3d --lib`.
