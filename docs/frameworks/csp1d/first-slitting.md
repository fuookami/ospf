# Your First Slitting Plan: Demand and Plan Usage

Write an application example directly; running repository tests is not a prerequisite. Prepare dependencies and backends using [Setup](../setup).

## 1. Business Scenario

Produce 0.8 m wide strips with demand of three rolls from stock with width range 0.5–1.5 m, without a machine configuration. Three is a roll count, not 3 m or three pattern usages. Rust additionally limits available stock batches to three.

Declare roll demand explicitly with `ProductDemand.legacyRoll` / `legacy_roll`. Production is each pattern contribution multiplied by its usage. Three generated candidates do not mean three produced rolls.

## 2. Construct Input and Call the Framework

Kotlin uses an actual injected solver and returns a structured `Csp1dSolution` directly, not `Ret`. The Rust default entry needs no native optimizer but is heuristic. The name `Milp` does not establish native MILP execution; configure the master-solver interface separately when needed.

The following example functions include their imports and contain no test harness. The application injects a real Kotlin `ColumnGenerationSolver`; the next section shows the launcher.

::: code-group

<<< ../_examples/csp1d.kt [Kotlin]

<<< ../_examples/csp1d.rs [Rust]

:::

## 3. Launch from Your Application

Place the Kotlin example in application sources, retaining its package. This standalone launcher calls the lesson function. Add the domain starter, Gurobi plugin and coroutines, and configure matching native libraries and a license. This is the production plugin, not a test-only delegating wrapper.

```kotlin
import docs.examples.csp1d.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

Place the Rust example in `src/csp1d.rs` and use this launcher in `src/main.rs`. See Setup for dependencies and toolchain; do not enable `async` for these synchronous examples.

```rust
mod csp1d;

fn main() {
    csp1d::run_example();
}
```

At the CLI boundary these examples throw or return errors. A server should translate them at its request boundary and retain diagnostics. Preserve normal no-incumbent reports rather than inventing a technical failure or an empty successful plan.

## 4. Read and Validate the Result

Read status, failure and unmet demand before selected plans and usages. Recalculate P1 production in rolls and verify pattern widths and available batches. A nonempty candidate pool does not establish coverage; partial output is not a complete delivery.

## 5. Change One Business Condition

Change demand from three rolls to four and check stock and contributions. Next try weight demand: changing a label to kg is insufficient; supply unit weight, roll length and conversion data, then update acceptance criteria.

Continue to the [next business scenario](./business-scenario) and [integration checks](../integration). Consult [repository verification](../verification) when checking source behavior; test commands are not the application entry point.
