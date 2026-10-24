# 2D Packing · BPP2D

Place existing rectangular parts on sheets, validate rotation, spacing, bounds and overlap, then inspect material consumption and remnants. A layout needs demand coverage, sheet-type selection and final validation, not just coordinates.

## Minimum Vocabulary

| Concept | API | Input or output |
| --- | --- | --- |
| Item | `ActualItem` | Shape, weight and rotation permission |
| Sheet type | `SheetType` | Dimensions, load capacity and type identity |
| Layout candidate | `Layer` | Placements carrying a sheet type |
| Generation request | `Bpp2dLayerGenerationRequest` | Items, sheet types, spacing and candidate limits |
| Application service | `ColumnGenerationApplicationService` | Generation, assignment, packing and KPI/render output |

Rectangles support optional 90-degree rotation. Circles and mixed shapes come later. Interior intersection is overlap; touching is not. Distinguish actual area from bounding area.

## Entry Points

Both languages expose `ColumnGenerationApplicationService`, but Kotlin solver injection differs from Rust's RMP/final executors. A Rust run without solvers is not a completed integer optimization.

Understand candidate layers and final geometry validation before connecting restricted-master solves, pricing and final MILP. Do not duplicate the framework's geometry rules in your application workflow.

## Boundaries

- Geometric feasibility is not guillotine manufacturability. See [CSP2D](../csp2d/) for production demand, cut trees and coil consumption.
- Circle utilization uses actual circle area, not its bounding rectangle.
- Area consumption, sheet count and type cost are different objectives. A weighted sum is not automatically strict lexicographic optimization.
- Candidate generation or mock execution alone is neither a native solver run nor a full-problem optimality proof.

## Learning Path

1. Complete [environment setup](../setup).
2. Follow [your first 2D layout](./first-layout) using rectangles and one sheet type.
3. Change rotation and spacing, then add multiple types and costs. Inject new generators instead of copying the solving loop.
4. Decide whether a new rule constrains one candidate or the combination of selected candidates.

## Reference

- [Kotlin API and shape capabilities](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp2d/README.md)
- [Rust API and executor contracts](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp2d/README.md)

[Back to scenario selection](../)

## Continue Practicing

Next: [Sheet Types, Rotation and Spacing](./business-scenario). Before adapting the scenario to your application, complete the [business integration checks](../integration).
