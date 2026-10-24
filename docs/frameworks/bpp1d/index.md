# 1D Packing (BPP1D): Scope and Learning Path

Pack existing long items into length-limited containers, such as loading cut-to-length profiles. Each container type has length, weight capacity, cost and available stock; adjacent items may require spacing. BPP1D chooses packing compositions and produces physical placements you can inspect.

## Essential Objects

| Business concept | API | Role | Key fields |
| --- | --- | --- | --- |
| Item | `Item` | Input | Stable `id`, `length`, `weight`, exact `amount` |
| Container type | `BinType` | Input | Length, weight capacity, `cost`, optional `inventory` |
| Problem | `PackingProblem` | Input | Items, container types, `spacing`, `objective` |
| Composition | `PackingPattern` | Intermediate | Container type ID and item counts, before coordinate expansion |
| Solve result | `Bpp1dSolution` | Output | Status/report, candidate completeness, column count, `packing` |
| Physical packing | `PackingResult` | Output | `plans`, free/tail intervals, material totals |
| Indicators | `PackingKpi` | Output | Container count, item count, consumed capacity length, tail length, utilization, cost |

Spacing applies only between adjacent items in the same container. There is no extra leading or trailing gap: packing n items introduces n−1 gaps.

::: tip Units and Stock
Length and weight retain physical units. Kotlin converts compatible units through its adapter; the Rust lesson normalizes inputs to meters/kilograms. `inventory = null` (Kotlin) / `None` (Rust) means unlimited; `0` / `Some(0)` means unavailable.
:::

### Find Business Fields in the Result

| What to read | Kotlin | Rust |
| --- | --- | --- |
| Candidate enumeration complete | `solution.candidateComplete` | `solution.candidate_complete` |
| Used containers | `solution.packing.plans` | `plans` after `solution.packing.as_ref()` |
| Container and type | `plan.bin.id`, `plan.bin.type` | `plan.bin_id`, `plan.bin_type_id`; look up `context.problem().bin_types` |
| Item interval | `placement.itemId`, `start`, `start + length` | `placement.item_id`, `start`, `length` |
| Tail/free intervals | `packing.remnants` for tails; derive gaps from adjacent placements | `packing.free_intervals`, including internal gaps and tails |
| Core KPIs | `packing.kpi(Meter)` | `context.kpi(&packing)` |

::: info Language-Specific Results
Kotlin exposes `packing` directly on success; Rust uses an `Option`, so inspect `report` when no feasible solution was returned. Both KPI fields `remnantLength` / `remnant_length` count tails only, excluding spacing. The generator's `complete` becomes the solution's candidate-completeness field; **it does not by itself prove integer optimality**.
:::

## Which Entry Point?

| Entry | Use it for | How to evaluate it |
| --- | --- | --- |
| `Bpp1dMilp` | Initial integration, small instances, or generated pools that fit a direct integer master | Establish correctness and runtime baselines; inspect completeness and column count |
| `Bpp1dColumnGeneration` | Inspecting LP pricing or comparing staged column activation | Compare runtime, column count and reports with the same data, generation budgets and backend |

Both entries currently generate a **bounded finite candidate pool first**. Column generation prices within that pool, and the final integer model uses the union of generated and initial columns. It does not bypass enumeration, so there is no universal item-count threshold for switching. Benchmark representative inputs.

Kotlin injects `ColumnGenerationSolver` and a numeric adapter into the constructor; Rust injects a solver callback into `solve`. See the [second lesson's switching and budget example](./business-scenario#column-generation).

## Scope Boundaries

- This models loading existing items. For material processing, production and over/underproduction, compare [CSP1D](../csp1d/).
- Candidate/node budgets can truncate enumeration; read candidate completeness with the report.
- LP pricing termination and finite-pool optimality have different proof scopes from global integer optimality.
- Tail repacking must revalidate the full plan, including global business rules.

## Learning Path

1. Complete [Setup](../setup).
2. Follow [your first 1D packing](./first-packing) through input, solving, placements and exact counts.
3. Add stock, cost and custom rules in the second lesson.
4. For custom models, consult the [modeling workflow](/guide/modeling-workflow).

For LP, MILP, pricing and other algorithm terms, see the [terminology guide](../#terminology).

## API References

- [Kotlin API and extension interfaces](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp1d/README.md)
- [Rust API and extension interfaces](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp1d/README.md)
- [Understanding solve results](/guide/solving-results)

[Back to scenario selection](../)

## Continue Practicing

Next: [Your First 1D Packing: Items, Gaps and Stock](./first-packing). Then explore [container types, stock and cost](./business-scenario) and the [business integration checks](../integration).
