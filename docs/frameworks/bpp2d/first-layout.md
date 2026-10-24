# Your First 2D Layout: Start with Rectangles

Write an application example directly; running repository tests is not a prerequisite. Prepare dependencies and backends using [Setup](../setup).

## 1. Business Scenario

Place R1 (4 × 3 m, six pieces) and R2 (2 × 2 m, eight pieces) on 20 × 12 m standard sheets. Rotation is allowed and kerf is zero. Each piece weighs 1 kg and a sheet carries up to 10000 kg.

Net area is 104 m² versus 240 m² per sheet. Sufficient area does not prove geometric feasibility. Keep IDs consistent across the catalogue, demands and sheet types; equal dimensions do not merge separate orders.

## 2. Construct Input and Call the Framework

The Kotlin example constructs items, sheet types and the request before calling the public service. The Rust example is an explicit integration boundary: the caller supplies domain input plus real RMP and final adapters; it is not a standalone optimizer. The current Rust service is fixed to `f64 + Meter` and shares a unit type between weight and length. Check this boundary instead of copying test simplifications that express weight in metres. `StandardRmpSolver` and `StandardFinalSolver` wrap closures; they do not automatically implement a SCIP backend.

The following example functions include their imports and contain no test harness. The application injects a real Kotlin `ColumnGenerationSolver`; the next section shows the launcher.

::: code-group

<<< ../_examples/bpp2d.kt [Kotlin]

<<< ../_examples/bpp2d.rs [Rust: integration boundary]

:::

## 3. Launch from Your Application

Place the Kotlin example in application sources, retaining its package. This standalone launcher calls the lesson function. Add the domain starter, Gurobi plugin and coroutines, and configure matching native libraries and a license. This is the production plugin, not a test-only delegating wrapper.

```kotlin
import docs.examples.bpp2d.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

Use Rust `solve_layout` as the application boundary after implementing input normalization and both RMP/final adapters. This lesson does not provide a standalone native Rust BPP2D adapter; `None, None` or hand-filled prices/usages are not substitutes.

At the CLI boundary these examples throw or return errors. A server should translate them at its request boundary and retain diagnostics. Preserve normal no-incumbent reports rather than inventing a technical failure or an empty successful plan.

## 4. Read and Validate the Result

Confirm both LP and final integer execution, then count exactly R1=6 and R2=8. Check boundaries, pairwise non-overlap, allowed rotation and spacing. The example count check is only the first delivery gate, not geometric verification or a global optimality proof.

## 5. Change One Business Condition

Disable rotation for R1 only and compare candidates and coordinates. Add positive kerf in a separate iteration so you can explain which condition changed the result.

Continue to the [next business scenario](./business-scenario) and [integration checks](../integration). Consult [repository verification](../verification) when checking source behavior; test commands are not the application entry point.
