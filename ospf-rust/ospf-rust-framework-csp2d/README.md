# ospf-rust-framework-csp2d

:us: English | :cn: [简体中文](README_ch.md)

## Introduction

`ospf-rust-framework-csp2d` is the Rust migration of Kotlin `ospf-kotlin-framework-csp2d`. It provides a 2D
cutting stock framework that reuses the CSP semantics of
[`ospf-rust-framework-csp1d`](../ospf-rust-framework-csp1d/README.md) (products, demands, patterns, plans,
yield, waste, sheet length assignment, column generation) while adopting the generic layering discipline of
[`ospf-rust-framework-bpp2d`](../ospf-rust-framework-bpp2d/README.md): the domain is generic over `V` and
solver value types appear only at the modelling boundary and in the application layer.

The 2D geometry is shared with BPP2D through
[`ospf-rust-framework-geometry2d`](../ospf-rust-framework-geometry2d/README.md).

## Scope

Two supply modes, one model:

| Mode | Problem class | Material model | Decision variables |
| --- | --- | --- | --- |
| Coil cutting | Strip packing (fixed width, length to be determined) | `width` fixed, `length = None` | consumed length and layout |
| Fixed sheet | 2D bin packing | `width × height` fixed | sheet count and layout |

Cutting processes are configurable and orthogonal to the mode:

| Process | Meaning | Equipment |
| --- | --- | --- |
| `Freeform` | Arbitrary nesting, coordinates only | laser, waterjet, CNC |
| `GuillotineTwoStage` | Strips then pieces, emits a cut tree | shear, panel saw |
| `GuillotineThreeStage` | Strips, blocks, pieces, emits a cut tree | shear, panel saw |

Non-rectangular parts are explicitly out of scope.

## Module structure

| Rust module | Kotlin boundary | Responsibility |
| --- | --- | --- |
| [`src/infrastructure`](src/infrastructure) | `csp2d-infrastructure` | Cutting regions, kerf, guillotine cut lines and trees, cutting processes, errors. |
| [`src/domain/material`](src/domain/material) | `csp2d-domain-material-context` | Dual-mode materials, products and demands, patterns and plans. |
| [`src/domain/pattern_generation`](src/domain/pattern_generation) | `csp2d-domain-pattern-generation-context` | Pattern generation requests and the shelf generator. |
| [`src/domain/produce`](src/domain/produce) | `csp2d-domain-produce-context` | Master problem: pattern usage, demand coverage, material availability, objectives. |
| [`src/domain/yield`](src/domain/yield) | `csp2d-domain-yield-context` | Under- and over-production analysis, over-production bounds. |
| [`src/domain/wasting_minimization`](src/domain/wasting_minimization) | `csp2d-domain-wasting-minimization-context` | Remnant area and ratio analysis, waste penalty terms. |
| [`src/application`](src/application) | `csp2d-application` | Problem definition, the MILP path and the column generation path. |

## Cut tree semantics

A guillotine layout is not a set of coordinates but a **cut tree**: internal nodes are through cuts and leaves
are final pieces. The cut level is the stage number: the root cut is stage 1 and the cuts below it must be
exactly stage 2, and so on. A two-stage process therefore equals "strips then pieces" and a three-stage process
equals "strips, blocks, pieces".

`CutTree2::validate` rejects inconsistent region splits (including the kerf strip), stages beyond the process
limit, and incorrect child stages.

The shelf generator builds a balanced tree whose stage equals its depth. When the required depth exceeds the
process limit no tree is built and the pattern cannot validate — an explicit capability boundary rather than a
silently illegal pattern.

## Column generation and pricing

Column addition runs in two steps: candidates are generated from the residual demand, then filtered by reduced
cost `rc = consumption · price − Σ coefficient · π`, keeping only `rc < -tolerance` in ascending order.

To be explicit: the shadow prices `π` are currently **estimated from the coverage deficit ratio**, not taken
from LP duals. This is therefore an engineering approximation of reduced-cost pricing rather than a strict
pricing certificate; the interface is in place so real LP duals can replace the estimate.

## Generic value boundary

1. The domain layer is generic over `V` and keeps the compile-time unit `U`.
2. Coordinates, sizes and areas use `Quantity<V, U>`; dimensionless values use a bare `V`.
3. Solver value types appear only in `produce` (the modelling boundary) and in the application layer.
4. Generic code must not call `to_f64()`.

## Public API

| API | Responsibility | Stability |
| --- | --- | --- |
| `Region2` / `CutTree2` / `CuttingProcess2` | Cutting geometry, guillotine trees and process capability. | migration |
| `Material2` / `Product2` / `Pattern2` / `CuttingPlan2` | Dual-mode domain model. | migration |
| `ShelfPatternGenerator2` | Constructive pattern generation with kerf pitch. | migration |
| `Csp2dMasterProblem` | Master problem modelling. | migration |
| `Csp2dSolvers` | MILP path and column generation path. | migration |
| `Csp2dPricingConfig` / `improving_columns` | Reduced-cost pricing and improving-column selection. | migration |
| `SheetAssignment` | Sheet-count and coil-segment-height assignment conventions. | migration |
| `CostarFiller` | By-product filling of the remnant. | migration |
| `Csp2dMasterSolver` | Injectable master-problem solver interface with a built-in heuristic. | migration |

## Related documents

- [ospf-rust README](../README.md) lists every crate and its Kotlin boundary.
- [csp1d](../ospf-rust-framework-csp1d/README.md) is the mature 1D reference.
- [geometry2d](../ospf-rust-framework-geometry2d/README.md) provides the shared 2D geometry.
- [csp2d.md](csp2d.md) records the migration handover and known gaps.

## Local verification

```powershell
$env:RUSTC_BOOTSTRAP = "1"
cargo test -p ospf-rust-framework-csp2d
```
