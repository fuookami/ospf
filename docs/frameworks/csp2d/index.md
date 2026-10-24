# 2D Cutting · CSP2D: Scope and Learning Path

Cut rectangular products from fixed sheets or coils while satisfying orders and process requirements. Start with fixed sheets and Freeform before coils and two-/three-stage guillotine processes.

## Minimum Vocabulary

| Concept | API | Role | Key Fields |
| --- | --- | --- | --- |
| Supply material | `Material2` | Input | Fixed width/height for sheets; no fixed height for coils |
| Product and demand | `Product2` / `ProductDemand2` | Input | Dimensions and demand modes such as count |
| Pattern | `Pattern2` | Intermediate | Placements, coverage and optional cut tree |
| Plan | `CuttingPlan2` | Intermediate | Material consumption and pattern use |
| Problem | `Csp2dProblem` | Input | Materials, products, demands, kerf, process and configuration |
| Cutting result | `Csp2dSolution` | Output | Pattern usage, coverage, consumed length, waste and termination |

Freeform requires valid geometry; guillotine processes also need a valid cut tree. Sheet count and consumed coil length are different objective measures.

::: tip Missing Height and Cut Trees
A missing material height denotes a coil, not a zero-height sheet; each pattern still needs a segment height. A missing Freeform cut tree is valid, while guillotine patterns require a tree consistent with placements.
:::

## Entry Points

::: info Kotlin and Rust Entry Points
Kotlin uses `Csp2dMilp` / `Csp2dColumnGeneration`. Rust uses `Csp2dSolvers`, with an internal heuristic by default and an injectable `Csp2dMasterSolver`. A method named MILP does not establish that the default Rust path invokes a native MILP solver.
:::

Current implementations and configuration are not fully symmetric. Start from each language's executable case rather than treating parallel snippets as a promise of identical capabilities.

Use `Csp2dMilp` / `solve_milp` to inspect a once-generated pool; use `Csp2dColumnGeneration` / `solve_column_generation` to append patterns against residual demand. This is not certified exact dual pricing; see the [configuration and switch](./business-scenario#entry-budget).

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

For LP, MILP, pricing and other algorithm terms, see the [terminology guide](../#terminology).

## Reference

- [Kotlin modules and process semantics](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-csp2d/README.md)
- [Rust API, master and pricing boundaries](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-csp2d/README.md)

Historical phase labels in the Kotlin module README may lag behind application source. These tutorials use public application APIs; implemented APIs and behavior belong in the verification reference, not capability claims inferred from phase labels.

[Back to scenario selection](../)

## Continue Practicing

Next: [Your First Cutting Plan](./first-cutting). Then explore [Kerf, Cutting Process and Coils](./business-scenario). Before adapting the scenario to your application, complete the [business integration checks](../integration).
