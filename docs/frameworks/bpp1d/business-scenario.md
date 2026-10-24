# Container Types, Stock and Cost

After [your first 1D packing](./first-packing), offer two container types for the same items. Use this **new shared input** to replace the Kotlin and Rust first-lesson inputs.

## 1. Start with Hand-Checkable Data

Items:

```csv
id,length,weight,amount
a,6,1,2
b,4,1,2
```

Containers:

```csv
id,length,max_weight,cost,inventory
small,10,100,3,2
large,20,100,10,1
```

Explicitly declare meters and kilograms; start with zero spacing. Net length is 20 m. One large or two small containers can carry everything without exceeding weight capacity.

| Objective | An optimal choice | Containers | Total cost |
| --- | --- | ---: | ---: |
| Minimum count | One large | 1 | 10 |
| Minimum cost | Two small, each carrying a+b | 2 | 6 |
| Minimum consumed length | Both choices consume 20 m | Not unique | Not decided by this objective |

These are hand calculations for the complete tiny problem. If generation budgets omit a composition, preserve the restricted-model scope of program conclusions.

## 2. Map to Actual Entry Points

Kotlin uses `Bpp1dCsvDatasetLoader`; Rust uses `load_packing_csv`. Both require explicit units and numeric parsing: the CSV contains no units. Alternatively, construct `Item`, `BinType` and `PackingProblem` using the first lesson's factories.

These contextual fragments switch the objective **after `problem` has been built**; they are not standalone programs:

::: code-group

```kotlin [Kotlin]
val costProblem = problem.copy(objective = PackingObjective.Cost)
```

```rust [Rust]
let mut cost_problem = problem.clone();
cost_problem.objective = PackingObjective::Cost;
```

:::

See the [Kotlin input model](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp1d/bpp1d-domain-item-context/src/main/fuookami/ospf/kotlin/framework/bpp1d/domain/item/model/PackingProblem.kt) and [Rust input model](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp1d/src/domain/model.rs). Retain the working solver and outcome handling from the first lesson; an objective change does not wire a backend.

## 3. Change One Condition at a Time

1. Reduce small stock to 1, keeping zero spacing. Minimum cost becomes 10 with one large; do not silently procure a second small.
2. Restore small stock to 2 and set large stock to zero. Two small still work, costing 6.
3. Also reduce small stock to 1. Total capacity is insufficient: no complete feasible plan exists.
4. Restore original stock and set spacing to 1 m. One large cannot carry all four items, and a+b no longer fits a small. A large carrying a+a+b plus a small carrying b costs 13. Validate this new problem instead of retaining old assertions.

Blank inventory means unlimited; zero means unavailable. Do not map both to the same adapter default.

## 4. Recompute Before Delivery

Count exactly two a and two b from actual placements; check stock by type, then length, weight and inter-piece gaps per container. Recompute cost from actual container use. Equal consumed length does not imply equal count or cost.

Use `PatternPolicy` for a single-composition prohibition. For cross-container quantities or budgets, investigate `PatternModelingExtension` and verify consistency across MILP, LP, final and final-domain checks. Do not merely remove offending containers from a report.

Replace the first-lesson item and container lists directly, retaining units, stock and objective fields. Solve again and independently recompute the result.

Move the data, invariants and failure cases into your application using the [integration guide](../integration).

Repository regression commands live in [Verification](../verification). Complete the business input, call and result checks before using them.
