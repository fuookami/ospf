# OSPF Rust Framework BPP3D

:us: English | :cn: [简体中文](README_ch.md)

## Introduction

`ospf-rust-framework-bpp3d` is the Rust migration target for Kotlin `ospf-kotlin-framework-bpp3d`. It provides reusable three-dimensional bin-packing domain capabilities, including BPP3D infrastructure, item modeling, BLA/block-loading/layer-generation contexts, layer-assignment pipelines, packing, CSV protocol adapters, renderer DTOs, and application-level column-generation orchestration.

## Scope

This crate owns the reusable BPP3D framework kernel. It keeps packing geometry, candidate generation, layer assignment, final packing, fixture protocols, and renderer DTO boundaries inside the shared domain crate.

Explicit non-goals:

1. Business-specific request DTOs, tenant runtime context, formula languages, or project deployment policies.
2. External renderer source code.
3. Solver installation, license management, or non-BPP3D backend plugin ownership.

## Module Structure

| Rust module or directory | Kotlin boundary | Responsibility |
| --- | --- | --- |
| [`src/infrastructure`](src/infrastructure/README.md) | `bpp3d-infrastructure` | Geometry primitives, orientation, packing shapes, PWL approximation helpers, and renderer DTOs. |
| [`src/domain/item`](src/domain/item/README.md) | `bpp3d-domain-item-context` | Items, packages, materials, bins, layers, patterns, demand coverage, shape metadata, and continuous-radius model components. |
| [`src/domain/bla`](src/domain/bla/README.md) | `bpp3d-domain-bla-context` | Bottom-up-left-justified placement and projection candidate construction. |
| [`src/domain/block_loading`](src/domain/block_loading/README.md) | `bpp3d-domain-block-loading-context` | Simple/complex block generation, DFS space splitting, MLHS search, and block-loading diagnostics. |
| [`src/domain/layer_generation`](src/domain/layer_generation/README.md) | `bpp3d-domain-layer-generation-context` | Block, BL, circle-packing, Pattern, Pile, and Historical layer candidate generation. |
| [`src/domain/layer_assignment`](src/domain/layer_assignment/README.md) | `bpp3d-domain-layer-assignment-context` | RMP/final MILP variables, dynamic columns, constraints, objectives, demand/depth/activation limits, and shadow prices. |
| [`src/domain/packing`](src/domain/packing/README.md) | `bpp3d-domain-packing-context` | Final packed-bin conversion, geometry guard, placement/block trace replay, material summaries, and render adaptation. |
| [`src/application`](src/application/README.md) | `bpp3d-application` and layer-selection flow | Solver backend selection, CSV materialization, column-generation lifecycle, fixture reports, KPI/trace diagnostics, and renderer output. |

## Architecture Overview

The modeling path is centered on `MetaModel`: RMP and final MILP assembly share `LayerAssignmentContext`, `LayerAssignmentAggregation`, variable components, demand/depth/activation limits, and objective pipelines. Application services coordinate solver backends, CSV materialization, trace/KPI diagnostics, and renderer adaptation. Placement and block trace replay remain in the packing domain.

Kotlin `bpp3d-domain-layer-selection-context` is mapped into Rust application orchestration, because Rust keeps layer selection as a solver lifecycle around reusable domain contexts.

## Core Concepts

1. `BinLayer` is the selected-column unit in layer assignment.
2. Layer generation supplies candidate layers from BLA, block loading, circle packing, Pattern, Pile, and Historical sources.
3. Layer assignment registers RMP/final MILP variables and constraint/objective pipelines through `MetaModel`.
4. Packing converts selected layers into packed bins and validates shape-aware geometry before renderer output.
5. CSV fixture suites provide Kotlin comparison baselines and solver-backend diagnostics.

## Public API

| API | Responsibility | Stability |
| --- | --- | --- |
| `application::ColumnGenerationApplicationService` | Application-facing column-generation orchestration. | migration |
| `application::MetaModelSolverBackend` | Pluggable solver backend boundary for RMP/final executors. | migration |
| `application::ColumnGenerationStandardExecutors` | Standard RMP/final executor wiring. | migration |
| `application::CsvDatasetLoader` | CSV dataset loading when `serde` is enabled. | migration |
| `application::Bpp3dRunReport` | Structured run and comparison reporting. | migration |
| `domain::layer_assignment::*` | Layer-assignment context, aggregation, limits, objectives, and dynamic columns. | migration |
| `domain::layer_generation::*` | Layer candidate generation traits, requests, diagnostics, and strategy implementations. | migration |
| `domain::packing::*` | Packing conversion, geometry guard, packed-bin solution, and render adaptation. | migration |
| `infrastructure::*` | Shared geometry, orientation, shape, PWL, and renderer DTO types. | stable within migration |

## Modeling Extensions

Add solver behavior through `MetaModelSolverBackend` or the RMP/final executor traits. Add new layer candidate sources by implementing the layer-generation traits. Add request-level business rules through domain policies such as package-rule policies instead of embedding arbitrary closures inside serializable models.

New constraints, objectives, and result extraction should be added in domain contexts, aggregations, model components, or pipelines. Application code should compose those extension points rather than duplicating domain modeling logic.

## Generic Numeric Boundaries

Domain-facing APIs keep generic numeric values where they describe reusable geometry, quantities, and demand semantics. Solver-facing RMP/final execution currently uses `MetaModel<f64>` at the registration and adapter boundary. Conversion to raw `f64` should stay in solver, CSV, reporting, or renderer edges.

## Physical Quantity Boundaries

Widths, heights, depths, weights, volumes, radii, material dimensions, package dimensions, demand coverage, and capacity-like quantities should remain typed through `Quantity<V, U>` or infrastructure geometry wrappers. Renderer DTOs and CSV protocol values may expose scalar fields, but those scalar conversions belong at the serialization boundary.

## Solve Lifecycle

The current application flow is:

1. Load or construct a BPP3D request and candidate source set.
2. Generate BLA/block/circle/Pattern/Pile/Historical layer candidates.
3. Register layer-assignment RMP variables, constraints, objectives, and shadow-price metadata.
4. Solve RMP, extract stable demand shadow prices, generate or refresh columns, and register new layer variables.
5. Solve final MILP over the shared layer-assignment context.
6. Extract selected layers and replay placement/block traces through packing.
7. Validate final geometry and emit solution, KPI, diagnostics, and renderer DTOs.

## Outputs

`ColumnGenerationResult` records selected layers, packing analysis, solver status, and lifecycle diagnostics. `Bpp3dRunReport` and fixture-suite reports capture dataset comparison, solver availability, failures, demand coverage, packed-bin output, selected-layer diagnostics, and feature-matrix information. Renderer DTOs include shape metadata and `actualVolume` output for cuboids and cylinders.

## Usage

```rust,ignore
use ospf_rust_framework_bpp3d::application::{
    ColumnGenerationApplicationService, ColumnGenerationConfig,
};

let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
let result = service.solve(request)?;
```

CSV and fixture-suite entry points require the `serde` feature.

## Local Validation

```powershell
cargo check -p ospf-rust-framework-bpp3d
cargo test -p ospf-rust-framework-bpp3d --lib
cargo check -p ospf-rust-framework-bpp3d --features serde
cargo test -p ospf-rust-framework-bpp3d --features serde --lib
```

Real-solver manifest suites are feature-gated and require local solver availability.

## Current Boundaries

Current migration coverage includes:

1. Solver-agnostic `MetaModel` RMP/final executors, pluggable backend wrappers, and a one-round RMP -> shadow-price generation -> column refresh -> final flow.
2. Demand coverage metadata on generated layers, stable demand shadow-price keys, selected-layer extraction, and packed-bin/render replay from placement or block traces.
3. BLA local/global layer candidates, simple-block layer candidates, multi-round axis `ComplexBlockGenerator`, bounded DFS space splitting, MLHS branch/depth candidate ranking, circle-packing grid candidates for fixed/discrete cylinder radii, and request-aware Pattern/Pile/Historical generators.
4. Axis-aware cuboid and cylinder geometry, guarded horizontal-cylinder final validation, PWL continuous-radius metadata/fixtures, and renderer `actualVolume` output.
5. CSV schema guards, Kotlin Gurobi grouped-layer/material-width-amount adapters, manifest and recursive directory fixture loading, backend survey/no-run/fake fallback reports, and feature-matrix diagnostics.
6. PatternedItem conservative demand coverage, PackageAttribute validation/packing diagnostics, and RestAmount/TailBinLoadingRate/BinLoadingOrder `MetaModel` registration hooks.

The manifest mirrors the full Kotlin Gurobi CSV sample set into 22 fixtures, including 19 Kotlin-derived dataset samples plus 3 local regression fixtures. Remaining gaps are tracked in the current-boundaries list above and should be compared with the Kotlin BPP3D README when updating migration coverage.

## Related Modules

- [Root README](../README.md)
- [BPP3D domain README](src/domain/README.md)
- [BPP3D application README](src/application/README.md)
- [Kotlin BPP3D README](../../ospf-kotlin/ospf-kotlin-framework-bpp3d/README.md)
