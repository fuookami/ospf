# 1D Packing · BPP1D

Pack existing long items into length-limited slots. Container types have length, weight capacity, cost and inventory; adjacent items may need spacing. BPP1D selects loading patterns and produces independently checkable placements.

## Minimum Vocabulary

| Concept | API | Input or output |
| --- | --- | --- |
| Item | `Item` | Stable ID, length, weight and exact count |
| Container type | `BinType` | Length, load capacity, cost and optional stock |
| Problem | `PackingProblem` | Items, containers, spacing and objective |
| Loading pattern | `PackingPattern` | Counts of each item in one container |
| Final result | `PackingResult` | Placements, free intervals, materials and KPIs |

Spacing occurs only between adjacent items, not after the last item. Preserve units: `40 cm` cannot be compared directly with an unqualified `1`.

## Entry Points

Kotlin injects `ColumnGenerationSolver` and a numeric adapter into `Bpp1dMilp` / `Bpp1dColumnGeneration`. Rust's similarly named application entries receive actual solver models through callbacks; do not translate constructors and error handling line by line.

You own data, objectives and backend configuration. The framework owns pattern generation, registration, selection and packing validation. Start with ordinary MILP before column generation.

## Boundaries

- This models existing items. Compare [CSP1D](../csp1d/) for material processing, production contributions and production deviations.
- Candidate enumeration can be bounded by nodes and pattern counts. Read `complete`; do not assume it.
- LP pricing termination is not integer optimality; restricted-pool optimality is not automatically global optimality.
- Tail repacking must validate the full plan. Global-only business rules cannot simply become local subproblem rules.

## Learning Path

1. Complete [environment setup](../setup).
2. Follow [your first 1D packing](./first-packing) for input, solving, placements and quantity checks.
3. Add spacing, inventory, multiple types and costs to the same scenario, then inspect extension contracts.
4. Read the [modeling workflow](/guide/modeling-workflow) when custom modeling is needed; do not duplicate domain formulas in the application service.

## Reference

- [Kotlin API and extension contracts](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp1d/README.md)
- [Rust API and extension contracts](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp1d/README.md)
- [Understanding solver results](/guide/solving-results)

[Back to scenario selection](../)

## Continue Practicing

Next: [Container Types, Stock and Cost](./business-scenario). Before adapting the scenario to your application, complete the [business integration checks](../integration).
