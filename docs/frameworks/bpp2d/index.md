# 2D Packing · BPP2D: Scope and Learning Path

Place existing rectangular parts on sheets, validate rotation, spacing, bounds and overlap, then inspect material consumption and remnants. A layout needs demand coverage, sheet-type selection and final validation, not just coordinates.

## Minimum Vocabulary

| Concept | API | Role | Key Fields |
| --- | --- | --- | --- |
| Item | `ActualItem` | Input | Shape, weight and rotation permission |
| Sheet type | `SheetType` | Input | Dimensions, load capacity and type identity |
| Layout candidate | `Layer` | Intermediate | Placements carrying a sheet type |
| Generation request | `Bpp2dLayerGenerationRequest` | Input | Items, sheet types, spacing and candidate limits |
| Application service | `ColumnGenerationApplicationService` | Entry / validation | Generation, assignment, packing and KPI/render output |
| Packing result and metrics | `PackingResult` / packing snapshot | Output | Sheets, placements, free rectangles, area and termination diagnostics |

Rectangles support optional 90-degree rotation. Circles and mixed shapes come later. Interior intersection is overlap; touching is not. Distinguish actual area from bounding area.

## Entry Points

::: info Kotlin and Rust Entry Points
Both languages expose `ColumnGenerationApplicationService`, but Kotlin solver injection differs from Rust's restricted-master and final-integer executors. A Rust run without solvers is not a completed integer optimization.
:::

Understand candidate layers and final geometry validation before connecting restricted-master solves, pricing and final MILP. Do not duplicate the framework's geometry rules in your application workflow.

The application entry already orchestrates column generation; there is no equivalent `Bpp2dMilp` switch. Start with one sheet type and bounded candidates, then expand types and budgets. See the [entry and budget example](./business-scenario#entry-budget).

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

For LP, MILP, pricing and other algorithm terms, see the [terminology guide](../#terminology).

## Reference

- [Kotlin API and shape capabilities](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp2d/README.md)
- [Rust API and executor interfaces](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp2d/README.md)

[Back to scenario selection](../)

## Continue Practicing

Next: [Your First 2D Layout](./first-layout). Then explore [Sheet Types, Rotation and Spacing](./business-scenario). Before adapting the scenario to your application, complete the [business integration checks](../integration).
