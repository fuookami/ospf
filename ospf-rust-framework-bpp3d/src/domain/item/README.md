# BPP3D Item Context

[中文](README_ch.md)

This context maps Kotlin `bpp3d-domain-item-context`.

## Purpose

Item defines the stable domain vocabulary for packages, materials, actual items, patterns, bins, layers, demand keys, cylinder contracts, and continuous-radius cylinder modeling. Other contexts should depend on these models rather than duplicating item semantics.

## File Layout

- `model.rs` is the public model shim. Kotlin-style model fragments live under `model/`, including `package.rs`, `material.rs`, `item.rs`, `pattern.rs`, `bin.rs`, `layer.rs`, and `schema.rs`.
- `model/package_attribute/` contains package type, deformation, hanging, orientation, pair-stacking, placement-stacking, and main `PackageAttribute` logic.
- `model/continuous_radius/` contains continuous cylinder radius prototypes, PWL registration config, solver result extraction, and item shape write-back.
- `service.rs` contains item-domain services such as merging and ordering helpers.

## Extension Points

Extend item rules through `PackageAttribute`, `PackageOrientationRule`, pair/placement stacking rules, `PatternConfig`, and continuous-radius weight functions. Keep business identifiers such as material manufacturer/supplier and cargo attribute keys in this context.

## Verification

Use `cargo test -p ospf-rust-framework-bpp3d --lib` and serde tests when touching CSV-facing item fields.
