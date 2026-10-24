# Container Types, Stock and Cost

After [your first 1D packing](./first-packing), offer two container types for the same items. Load CSV, compare objectives, then add a mixing policy and generation budgets.

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

Use meters and kilograms, initially with zero spacing. Net item length is 20 m. One large or two small containers can carry everything without exceeding weight capacity.

| Objective | An optimal choice | Containers | Cost |
| --- | --- | ---: | ---: |
| Minimum count | One large | 1 | 10 |
| Minimum cost | Two small, each carrying a+b | 2 | 6 |
| Minimum consumed length | Total enabled container capacity is 20 m in both cases: $1 \times 20\text{ m}$ or $2 \times 10\text{ m}$, fully packed with no tail | Not unique | Not decided by this objective |

Use these hand calculations as a small-instance business reference. Interpret program evidence using candidate completeness and the solve report together.

## 2. Load CSV and Change Objectives

Assign the CSV texts above to `itemsCsv` / `binsCsv` (Rust: `ITEMS_CSV` / `BINS_CSV`). The full source at the end includes imports and numeric parsers `parseNumber` / `parse_number`.

::: code-group

<<< ../_examples/bpp1d-business.kt#csv [Kotlin]

<<< ../_examples/bpp1d-business.rs#csv [Rust]

:::

::: tip Explicit Units and Null Stock
CSV carries no units: bind `Meter` and `Kilogram` and supply spacing when loading. This example parses `Flt64` / `f64`; higher-precision domains should parse directly into their numeric type. An empty `inventory` cell means unlimited stock; `0` means unavailable.
:::

::: info Different Loader Results
Kotlin returns `Ret<PackingProblem<Flt64>>`; the example's `getOrThrow()` handles CLI errors before `copy(objective = ...)` switches objectives. Rust returns a validated `PackingContext`. To switch objectives, clone `context.problem()`, modify `objective`, then validate and rebuild with `PackingContext::new`. The complete source solves each objective in turn.
:::

Reuse the first lesson's backend for `BinCount`, `Cost` and `ConsumedLength`, and extract physical results in the same way.

## 3. Change One Condition at a Time

Use minimum cost throughout; step 4 restores original stock.

| Step | Change | Result and interpretation |
| --- | --- | --- |
| 1 | Reduce small stock from 2 to 1, keeping zero spacing | One large, minimum cost 10 |
| 2 | Restore two small; set large stock to zero | Two small carrying a+b each, minimum cost 6 |
| 3 | From step 2, reduce small stock to one | Insufficient capacity; no complete feasible packing |
| 4 | Restore original stock; set spacing to 1 m | large carries a+a+b and small carries b, costing $10 + 3 = 13$ |

For step 4: all four items require $20 + 3 \times 1 = 23\text{ m} > 20\text{ m}$ in large; a+b requires $6 + 1 + 4 = 11\text{ m} > 10\text{ m}$ in small. The a+a+b composition needs $6 + 1 + 6 + 1 + 4 = 18\text{ m}$ in large; the remaining b (4 m) fits in small.

## 4. Validate Results and Add a Mixing Rule

Independently check the actual placements:

1. **Exact item counts:** group `placements` by ID; both a and b must occur twice.
2. **Container stock:** count `plans` by container type and compare with non-null inventory limits.
3. **Per-container physics:** check intervals, length, weight and adjacent spacing.
4. **Recomputed total cost:** add each used container type's `cost` once; compare with KPIs and the selected objective.

### Per-Container Policy: PatternPolicy

Restore zero spacing and original stock. Add the rule that a and b cannot share a container. Kotlin supplies the policy to the generator, then injects that generator into the solver entry:

::: code-group

<<< ../_examples/bpp1d-business.kt#policy [Kotlin]

<<< ../_examples/bpp1d-business.rs#policy [Rust: implement]

:::

Rust passes the policy explicitly, alongside the no-op model extension:

<<< ../_examples/bpp1d-business.rs#policy-injection [Rust: inject]

`problem` / `context` comes from section 2. `backend(solver)` forwards the model, remaining time and cancellation handle to the backend from the first lesson. With this policy, large can carry a+a and small can carry b+b, giving minimum cost 13. Keep the count and stock checks, and also verify that no container contains both IDs.

::: info Custom-Rule Report Scope
The policy applies to candidate enumeration and initial-candidate generation. Completeness is relative to its accepted compositions. Rust's `solve_with_extension` conservatively leaves `globally_optimal` false for custom rules; interpret the current-model `report`, completeness and business checks together.
:::

### Cross-Container Rules: PatternModelingExtension

For example, limiting total enabled containers to two constrains the sum of pattern usages. This ordinary-MILP example is independent of the mixing exercise:

::: code-group

<<< ../_examples/bpp1d-business.kt#extension [Kotlin]

<<< ../_examples/bpp1d-business.rs#extension [Rust]

:::

::: code-group

<<< ../_examples/bpp1d-business.kt#extension-injection [Kotlin: pass to SolveConfig]

<<< ../_examples/bpp1d-business.rs#extension-injection [Rust: pass to solve_with_extension]

:::

::: warning Before Using Model Extensions with Column Generation
Kotlin's lifecycle includes `register(context): Try`, `refresh(context): Try`, `reducedCost(pattern, context, dual): Flt64` and `extractSolution(context, values): Ret<Map<String, String>>`. Update additional expressions as columns change and include extra rows' dual contributions in pricing. This Kotlin `BinLimit` supports ordinary MILP only and explicitly checks the stage.

Rust rebuilds the complete model when the pool changes. `register(model, packing, aggregation, stage)` receives the current model and columns; optional `extract_solution(values)` extracts business indicators. Pricing uses the complete matrix. These lifecycles differ; consult the overview's [API references](./#api-references) for full signatures and constraint identity handling.
:::

## 5. Switch to Column Generation and Set Budgets {#column-generation}

Keep the section 2 input and backend; change the entry and configure its budgets:

::: code-group

<<< ../_examples/bpp1d-business.kt#cg [Kotlin]

<<< ../_examples/bpp1d-business.rs#cg [Rust]

:::

`maxPatterns` / `max_patterns` bounds generated candidates; `maxNodes` / `max_nodes` bounds enumeration nodes. `iterationLimit` / `iteration_limit` bounds pricing rounds; `columnsPerIteration` / `columns_per_iteration` bounds columns activated per round. Read completeness, `columnCount` / `column_count` and `pricingIterations` / `pricing_iterations` from the solution. These differ from backend time limits; when generation is truncated, retain the finite-pool scope of any optimality claim.

## 6. Run in Your Application

Kotlin: add the complete source and call `runBusinessExample(solver)`. Rust: place it in `src/bpp1d_business.rs` and call `bpp1d_business::run_business_example(&solver)`. Reuse the first lesson's launcher, replacing its import or module name. The example runs objective comparisons, the mixing policy, column generation and the container limit in turn.

:::: details Complete source: CSV, parsers, imports and backend forwarding

::: code-group

<<< ../_examples/bpp1d-business.kt [Kotlin]

<<< ../_examples/bpp1d-business.rs [Rust]

:::

::::

Use [integration guidance](../integration) to migrate your data, result invariants and failure handling. Consult [repository verification](../verification) for source regressions.
