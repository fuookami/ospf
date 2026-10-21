# ospf-rust-framework-bpp2d

:us: English | :cn: [简体中文](README_ch.md)

## Introduction

`ospf-rust-framework-bpp2d` is the Rust migration of Kotlin `ospf-kotlin-framework-bpp2d`. It provides a
reusable 2D packing framework for rectangular and circular items, with column generation, kerf handling,
remnant reporting and weighted objectives across multiple sheet types.

The 2D geometry is shared with the cutting framework through
[`ospf-rust-framework-geometry2d`](../ospf-rust-framework-geometry2d/README.md), so packing and cutting stock
never drift apart on area conventions or overlap semantics.

## Scope

The crate owns the reusable BPP2D kernel: item/sheet/layer models, BLA and block-loading layout, candidate
layer generation, the layer-assignment modelling boundary, final packing with geometry validation, remnant
reporting, and the column-generation application flow.

Explicit non-goals:

1. Business request DTOs, tenant context, formula languages or deployment policy.
2. External renderer implementations.
3. Solver installation and licence management; solver backends stay in `ospf-rust-core` /
   `ospf-rust-framework` behind feature gates.

## Module structure

| Rust module | Kotlin boundary | Responsibility |
| --- | --- | --- |
| [`src/infrastructure`](src/infrastructure) | `bpp2d-infrastructure` | Domain errors and render DTOs; re-exports the shared 2D geometry. |
| [`src/domain/item`](src/domain/item) | `bpp2d-domain-item-context` | Items, sheets, layers, materials, demand statistics and identifier types. |
| [`src/domain/bla`](src/domain/bla) | `bpp2d-domain-bla-context` | Bottom-up left-justified layout; circle items are skipped explicitly. |
| [`src/domain/block_loading`](src/domain/block_loading) | `bpp2d-domain-block-loading-context` | Space model, simple/complex blocks, DFS and multi-row splitting, rotation guard. |
| [`src/domain/layer_generation`](src/domain/layer_generation) | `bpp2d-domain-layer-generation-context` | Candidate layers per sheet type, circle grids, patterns, mixed shapes and historical replay. |
| [`src/domain/layer_assignment`](src/domain/layer_assignment) | `bpp2d-domain-layer-assignment-context` | The modelling boundary: demand coverage, reduced-cost pricing, layered objectives and capacity. |
| [`src/domain/packing`](src/domain/packing) | `bpp2d-domain-packing-context` | Final geometry guard, packer, remnant analysis and render adaptation. |
| [`src/application`](src/application) | `bpp2d-application` | Column-generation orchestration, standard executors, tail sheet optimisation and CSV datasets. |

## Architecture overview

Modelling is anchored on `MetaModel`. The RMP and the final MILP share the same layer-assignment context,
demand coverage convention and objective family. The application layer owns solver selection, the generation
loop, trace/KPI assembly and the packing/rendering boundary; it never accumulates variable or constraint
details of its own.

## Core concepts

1. A **layer** is a layout on one sheet; `SheetLayer` combines layers on the same sheet.
2. **Column identity** includes the sheet type, source, bounding size and placement set: the same layout on a
   different sheet type is a different column.
3. **Demand coverage** is a lower bound that must be satisfied; treating demand as an upper bound only would
   let the minimisation degenerate to "use no sheet at all".
4. **Reduced cost** is `rc_j = area_j · c_area − Σ π_i · a_ij`; only `rc_j < −tolerance` counts as improving.
5. **Objectives** are layered: sheet area consumption is primary (equivalent to minimum waste), the sheet
   amount weight is tiny and only breaks ties, and the per-type cost weight defaults to zero.

## Public API

| API | Responsibility | Stability |
| --- | --- | --- |
| `LayerGenerationContext` | Composite candidate generation per sheet type. | migration |
| `LayerAssignmentContext` | Five-stage column-generation lifecycle. | migration |
| `ColumnGenerationAlgorithm` | RMP → pricing → add columns → final MILP orchestration. | migration |
| `ColumnGenerationApplicationService` | End-to-end entry point. | migration |
| `PackingGeometryGuard` / `Packer` / `FreeSpaceAnalyzer` | Final geometry validation, packing and remnant reporting. | migration |
| `TailSheetOptimizer` | Re-selects the tail column against the residual demand. | migration |
| `CsvDatasetLoader` | `grouped-layer` and `material-width-amount` datasets. | migration |

## Modelling extension points

1. `Bpp2dLayerGenerator` — add a candidate generation strategy without touching the loop.
2. `LayerAssignmentObjectiveConfig` — reweight area, sheet amount and per-type cost, or supply type costs.
3. `PackingRotationRule` — inject the rotation knowledge that geometry alone cannot know.
4. `ColumnGenerationConfig` — iteration limit, per-iteration candidates, time limit and reduced-cost tolerance.

## Generic value boundary

1. The domain layer is generic over `V` and keeps the compile-time unit `U`.
2. `f64` appears only in `layer_assignment` (the modelling boundary) and in the application layer.
3. Generic code must not call `to_f64()`; counting and scaling go through `count_fitting` and
   `scaled_quantity`.

## Physical quantity boundary

Widths, heights, coordinates and areas are `Quantity<V, U>`; utilisation ratios and reduced costs are bare
values. Areas come from `quantity_multiply`, so a squared length is never computed as a cube.

## Solve lifecycle

`register` → RMP solve → `add_columns` → shadow price refresh → pricing → final MILP → `extract_solution`.
Retirement is expressed as removing a column from the aggregation and reporting it to the application layer.

## Output

`ColumnGenerationResult` carries columns, sheets, iteration statistics, per-iteration LP objectives, elapsed
time, the packing result and the render DTO.

## Usage

```toml
[dependencies]
ospf-rust-framework-bpp2d = { path = "../ospf-rust-framework-bpp2d" }
```

## Related documents

- [ospf-rust README](../README.md) lists every crate and its Kotlin boundary.
- [geometry2d](../ospf-rust-framework-geometry2d/README.md) provides the shared 2D geometry.
- [bpp2d.md](bpp2d.md) records the migration handover and known gaps.

## Local verification

```powershell
$env:RUSTC_BOOTSTRAP = "1"
cargo test -p ospf-rust-framework-bpp2d
```
