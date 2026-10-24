# 2D Cutting · CSP2D

Cut rectangular products from fixed sheets or coils while satisfying orders and process requirements. Start with fixed sheets and Freeform before coils and two-/three-stage guillotine processes.

## Minimum Vocabulary

| Concept | API | Input or output |
| --- | --- | --- |
| Supply material | `Material2` | Fixed width/height for sheets; no fixed height for coils |
| Product and demand | `Product2` / `ProductDemand2` | Dimensions and demand modes such as count |
| Pattern | `Pattern2` | Placements, coverage and optional cut tree |
| Plan | `CuttingPlan2` | Material consumption and pattern use |
| Problem | `Csp2dProblem` | Materials, products, demands, kerf, process and configuration |

Freeform requires valid geometry; guillotine processes also need a valid cut tree. Sheet count and consumed coil length are different objective measures.

## Entry Points

Kotlin uses `Csp2dMilp` / `Csp2dColumnGeneration`. Rust uses `Csp2dSolvers`, with an internal heuristic by default and an injectable `Csp2dMasterSolver`. A method named MILP does not establish that the default Rust path invokes a native MILP solver.

Current implementations and configuration are not fully symmetric. Start from each language's executable case rather than treating parallel snippets as a promise of identical capabilities.

## Boundaries

- Current scope is rectangular products, not arbitrary polygon nesting.
- Valid coordinates do not establish process feasibility; guillotine patterns need cut-tree validation.
- Residual-demand generation or estimated-price filtering is not an LP-dual pricing certificate.
- Coverage obtained by a default heuristic is not a global optimality proof.

## Learning Path

1. Complete [environment setup](../setup).
2. Follow [your first cutting plan](./first-cutting), starting with demand counts and plan usage.
3. Add kerf, then switch material mode or cutting process and check consumption and manufacturability separately.
4. Compare [BPP2D](../bpp2d/) if you only need placement of existing items.

## Reference

- [Kotlin modules and process semantics](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-csp2d/README.md)
- [Rust API, master and pricing boundaries](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-csp2d/README.md)

Historical phase labels in the Kotlin module README may lag behind application source. These tutorials use public application APIs; source contracts belong in the verification reference, not capability claims inferred from phase labels.

[Back to scenario selection](../)

## Continue Practicing

Next: [Kerf, Cutting Process and Coils](./business-scenario). Before adapting the scenario to your application, complete the [business integration checks](../integration).
