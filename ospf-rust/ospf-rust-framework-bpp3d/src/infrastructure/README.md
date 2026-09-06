# BPP3D Infrastructure

:us: English | :cn: [简体中文](README_ch.md)

This context maps Kotlin `bpp3d-infrastructure`.

## Responsibilities

Infrastructure contains typed geometry, orientation, packing-shape, PWL approximation, and render DTO adapters shared by domain and application code. It is the boundary where Rust keeps physical quantities and typed geometry while providing scalar adapters for solver or rendering edges.

## File Layout

- `geometry.rs` exposes typed points, vectors, sizes, AABBs, placements, and scalar conversion helpers; fragments live under `geometry/`.
- `orientation.rs` defines orientation categories and rotation semantics.
- `packing_shape.rs` defines cuboid/cylinder packing shapes and axis-aware bounding dimensions.
- `pwl_approximation.rs` contains radius and radius-squared PWL support used by continuous cylinder radius modeling. The `ErrorDriven` stopping condition and `max_relative_error` use the true maximum relative error of each segment rather than midpoint sampling. Configure it with `PwlRadiusApproximationConfig` and call `try_from_radius_interval` to receive validation errors; custom breakpoints must be finite, positive, strictly increasing, endpoint-matching (within tolerance), and within `max_segments`. The deprecated `from_radius_interval` compatibility entry may panic on invalid input and is not for model registration.
- `renderer.rs` contains render DTOs and shape/axis enums for application output.

## Public API

- `MetricPoint2`
- `MetricPoint3`
- `MetricSize2`
- `MetricSize3`
- `MetricAabb2`
- `MetricAabb3`
- `MetricPlacement2`
- `MetricPlacement3`
- `Orientation`
- `PackingShape3`
- `PwlBreakpointStrategy`
- `PwlRadiusApproximationConfig`
- `PwlApproximationError`
- `PwlRadiusSquaredApproximation`
- `ConservativeRadiusEnvelope`
- `RenderLoadingPlanDto`
- `SchemaDto`

## Extension Points

Add new geometry adapters here when they are shared across contexts. Keep raw `f64` conversion at solver/render boundaries, and keep domain-facing APIs typed with `Quantity<V, U>` and unit traits.

## Lifecycle and Data Flow

Domain contexts use typed metric geometry and unit-aware shapes, solver-facing edges explicitly convert to scalar coordinates, continuous-radius modeling uses PWL helpers, and application/reporting code serializes render DTOs at the output boundary.

## Verification

Use regular crate checks plus focused infrastructure tests through `cargo test -p ospf-rust-framework-bpp3d --lib`.

## Related Directories

- [`../domain`](../domain/README.md)
- [`../application`](../application/README.md)
