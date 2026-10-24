# 3D Packing · BPP3D: Scope and Learning Path

Cargo has three dimensions and weight. Choose bin types, orientations and positions, then produce a checkable, renderable loading plan. The first lesson uses cuboids rather than cylinders or complex support rules.

## Minimum Vocabulary

| Concept | API | Role | Key Fields |
| --- | --- | --- | --- |
| Cargo and package | `Item` / `Package` | Input | Dimensions, weight, materials and allowed orientations |
| Container type | `BinType` | Input | Geometry, load capacity and type identity |
| Candidate container layer | `BinLayer` | Intermediate | Container type, demand coverage and placement traces |
| Application service | `ColumnGenerationApplicationService` | Entry / validation | Master solves, generation, final selection and packing analysis |
| Final validation | `PackingGeometryGuard` | Entry / validation | Bounds, overlap and supported shape-path checks |
| Final packing result | `PackingResult` / packing snapshot | Output | Physical bins, item positions, orientations and loading order |

A candidate column represents a selectable loading structure, not a completed final packing. Connect selection, physical placement and reporting.

## Entry Points

::: info Kotlin and Rust Entry Points
Kotlin combines application requests, standard executors and a packing analyzer. Rust provides input-conversion and CSV interfaces with injected restricted-master and final-integer executors. Use the language-specific inputs and adapters shown in each lesson.
:::

Start with a fixed bin and a small set of initial layers, then add generators, bin types and objective configuration.

This lesson uses the application service for restricted-master and final-integer execution, not a `Bpp3dMilp` switch. Start with initial layers with actual placements and physical bins, then expand orientations, types and budgets. See [entry and budget configuration](./business-scenario#entry-budget).

## Boundaries

- Cylinder support depends on axis and generation/support/final-validation paths; it does not imply arbitrary rotation in 3D.
- Utilization must use the appropriate actual shape volume, not automatically its bounding cuboid.
- The external renderer is separate. Producing DTOs does not launch a visualization page.
- Inspect `finalSolved`, packing output and pricing diagnostics together, not just a nonempty candidate list.

## Learning Path

1. Complete [environment setup](../setup).
2. Follow [your first 3D loading](./first-loading), from candidate selection to final placements.
3. Add types, weight and costs, and explain the business meaning of objective weights.
4. See [framework example 2](/examples/framework-example2) for integrated aircraft loading. Industry safety rules are not automatically included by using BPP3D.

For LP, MILP, pricing and other algorithm terms, see the [terminology guide](../#terminology).

## Reference

- [Kotlin entry points and shape paths](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp3d/README.md)
- [Rust entry points and domain rules](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp3d/README.md)

[Back to scenario selection](../)

## Continue Practicing

Next: [Your First 3D Loading](./first-loading). Then explore [Bin Types and Objectives](./business-scenario). Before adapting the scenario to your application, complete the [business integration checks](../integration).
