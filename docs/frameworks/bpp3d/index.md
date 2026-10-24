# 3D Packing · BPP3D

Cargo has three dimensions and weight. Choose container types, orientations and positions, then produce a checkable, renderable loading plan. The first lesson uses cuboids rather than cylinders or complex support rules.

## Minimum Vocabulary

| Concept | API | Input or output |
| --- | --- | --- |
| Cargo and package | `Item` / `Package` | Dimensions, weight, materials and allowed orientations |
| Container type | `BinType` | Geometry, load capacity and type identity |
| Candidate container layer | `BinLayer` | Container type, demand coverage and placement traces |
| Application service | `ColumnGenerationApplicationService` | Master solves, generation, final selection and packing analysis |
| Final validation | `PackingGeometryGuard` | Bounds, overlap and supported shape-path checks |

A candidate column represents a selectable loading structure, not a completed final packing. Connect selection, physical placement and reporting.

## Entry Points

Kotlin combines application requests, standard executors and a packing analyzer. Rust provides materialized/CSV paths with injected RMP and final executors. Renaming Kotlin types into snake case is not a Rust implementation.

Start with a fixed container and a small seed set, then add generators, container types and objective configuration.

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

## Reference

- [Kotlin entry points and shape paths](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp3d/README.md)
- [Rust entry points and domain contracts](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp3d/README.md)

[Back to scenario selection](../)

## Continue Practicing

Next: [Bin Types and Objectives](./business-scenario). Before adapting the scenario to your application, complete the [business integration checks](../integration).
