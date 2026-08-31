# BPP3D Item Context

:us: English | :cn: [简体中文](README_ch.md)

This context maps Kotlin `bpp3d-domain-item-context`.

## Responsibilities

Item defines the stable domain vocabulary for packages, materials, actual items, patterns, bins, layers, demand keys, cylinder contracts, and continuous-radius cylinder modeling. Other contexts should depend on these models rather than duplicating item semantics.

## File Layout

- `model.rs` is the public model shim. Kotlin-style model fragments live under `model/`, including `package.rs`, `material.rs`, `item.rs`, `pattern.rs`, `bin.rs`, `layer.rs`, and `schema.rs`.
- `model/package_attribute/` contains package type, deformation, hanging, orientation, pair-stacking, placement-stacking, and main `PackageAttribute` logic.
- `model/continuous_radius/` contains continuous cylinder radius prototypes, PWL registration config, solver result extraction, and item shape write-back.
- `service.rs` contains item-domain services such as merging and ordering helpers.

## Public API

- `Package`
- `PackageAttribute`
- `PackageShape`
- `PackageOrientationRule`
- `PackagePairStackingRule`
- `PackagePlacementStackingRule`
- `Material`
- `MaterialKey`
- `ActualItem`
- `Bin`
- `BinLayer`
- `Bpp3dDemandKey`
- `PatternConfig`
- `ContinuousCylinderRadiusSolverPrototype`
- `ContinuousRadiusModelRegistration`

## Extension Points

Extend item rules through `PackageAttribute`, `PackageOrientationRule`, pair/placement stacking rules, `PatternConfig`, and continuous-radius weight functions. Keep business identifiers such as material manufacturer/supplier and cargo attribute keys in this context.

## Lifecycle and Data Flow

Application input is normalized into packages, materials, bins, layers, demand keys, and optional continuous-radius registration plans. Downstream generation, assignment, and packing contexts read these models as immutable domain vocabulary.

## Verification

Use `cargo test -p ospf-rust-framework-bpp3d --lib` and serde tests when touching CSV-facing item fields.

## Related Directories

- [`../layer_generation`](../layer_generation/README.md)
- [`../layer_assignment`](../layer_assignment/README.md)
- [`../packing`](../packing/README.md)
