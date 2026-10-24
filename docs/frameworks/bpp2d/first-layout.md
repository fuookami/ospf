# Your First 2D Layout: Start with Rectangles

Work through business input, solving and result extraction with this example. Prepare dependencies and backends using [Setup](../setup).

## 1. Business Scenario

Place R1 (4 × 3 m, six pieces) and R2 (2 × 2 m, eight pieces) on 20 × 12 m standard sheets. Rotation is allowed and kerf is zero. Each piece weighs 1 kg and a sheet carries up to 10000 kg.

Net area is 104 m² versus 240 m² per sheet. Sufficient area does not prove geometric feasibility. Keep IDs consistent across the catalogue, demands and sheet types; equal dimensions do not merge separate orders.

## 2. Construct Input and Call the Framework

::: info Kotlin and Rust Entry Points
The Kotlin example constructs items, sheet types and the request before calling the public service. The Rust example is an explicit integration boundary: the caller supplies domain input plus real restricted-master and final-integer adapters; it is not a standalone optimizer. The current Rust service is fixed to `f64 + Meter` and shares a unit type between weight and length. Handle this unit limitation explicitly in the application adapter. `StandardRmpSolver` and `StandardFinalSolver` wrap closures; they do not automatically implement a SCIP backend.
:::

The examples include imports and error handling. The application supplies Kotlin `ColumnGenerationSolver` to `runExample`; the next section shows the launcher.

::: code-group

<<< ../_examples/bpp2d.kt [Kotlin]

<<< ../_examples/bpp2d.rs [Rust: integration boundary]

:::

## 3. Launch from Your Application

Place the Kotlin example in application sources, retaining its package. This standalone launcher calls the lesson function. Add the domain starter, Gurobi plugin and coroutines, and configure matching native libraries and a license.

```kotlin
import docs.examples.bpp2d.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

Use Rust `solve_layout` as the application boundary after implementing input normalization and both restricted-master and final-integer adapters. This lesson does not provide a standalone native Rust BPP2D adapter; `None, None` or hand-filled prices/usages are not substitutes.

At the CLI boundary these examples throw or return errors. A server should translate them at its request boundary and retain diagnostics. Preserve reports when no feasible solution was found rather than inventing a technical failure or an empty successful plan.

## 4. Read and Validate the Result

| Read | Kotlin | Rust |
| --- | --- | --- |
| Enabled sheets and items | `packing.aggregation.bins → name / type / items` | `packing.sheets → name / sheet_type / items` |
| Coordinates and rotation | `placement.x / y / maxX() / maxY() / rotated` | `placement.x() / y() / max_x() / max_y() / rotated` |
| Remnants and KPIs | `packing.remnants`, `response.kpi` | `sheet.remnants`, `packing.item_amounts() / occupied_area() / remnant_area()` |
| Termination scope | `response.result.finalSolved / terminatedByIterationLimit / terminatedByTimeLimit` | `result.final_solved / terminated_by_iteration_limit / terminated_by_time_limit` |

::: info Result Semantics
This entry has no `complete` field. Final execution and budget flags describe the run; validate actual geometry and exact demand separately. Remnants are free rectangles associated with sheet names, not stocked inventory.
:::

Confirm both LP and final integer execution, then count exactly R1=6 and R2=8. Check boundaries, pairwise non-overlap, allowed rotation and spacing. The example count check is only the first delivery gate, not geometric verification or a global optimality proof.

## 5. Change One Business Condition

Disable rotation for R1 only and compare candidates and coordinates. Add positive kerf in a separate iteration so you can explain which condition changed the result.

Continue to the [next business scenario](./business-scenario) and [integration checks](../integration). Consult [repository verification](../verification) when checking source behavior.
