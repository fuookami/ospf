# Your First Network Flow: Balances, Capacity and Cost

Write an application example directly; running repository tests is not a prerequisite. Prepare dependencies and backends using [Setup](../setup).

## 1. Business Scenario

Source supplies two units and sink demands two. A delivery arc connects them with bounds 0–3 and cost 3 per unit. Use a common dimensionless unit first to learn balances, capacity, a real solve call and result handling.

Write supply as +2 and demand as -2 so total balance is zero. Hand calculation gives flow 2 and cost 6. Node, service and commodity IDs are separate; parallel services must not be identified by endpoints alone.

## 2. Construct Input and Call the Framework

Kotlin constructors for nodes, arcs, bounds and commodities return `Ret`; the example handles failures before calling `NetworkFlowApplicationService`. Rust uses `FlowProblem::new` and `FlowApplicationService` with an actual `LinearSolver`. Neither fills in a witness vector or copies model-building internals from a test.

The following example functions include their imports and contain no test harness. The application injects a real Kotlin `ColumnGenerationSolver`; the next section shows the launcher.

::: code-group

<<< ../_examples/network.kt [Kotlin]

<<< ../_examples/network.rs [Rust]

:::

## 3. Launch from Your Application

Place the Kotlin example in application sources, retaining its package. This standalone launcher calls the lesson function. Add the domain starter, Gurobi plugin and coroutines, and configure matching native libraries and a license. This is the production plugin, not a test-only delegating wrapper.

```kotlin
import docs.examples.network.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

Place the Rust example in `src/network.rs` and use this launcher in `src/main.rs`. See Setup for dependencies and toolchain; do not enable `async` for these synchronous examples.

```rust
mod network;
use ospf_rust_core::solver::solvers::scip::SCIPSolver;

fn main() -> Result<(), String> {
    network::run_example(SCIPSolver::new())
}
```

At the CLI boundary these examples throw or return errors. A server should translate them at its request boundary and retain diagnostics. Preserve normal no-incumbent reports rather than inventing a technical failure or an empty successful plan.

## 4. Read and Validate the Result

Read the report and optional business solution. Verify outflow 2 at source, inflow 2 at sink, arc flow within 0–3, and cost 2 × 3 = 6. Solver steps are not physical quantities, and outer success is not demand fulfillment.

## 5. Change One Business Condition

Reduce capacity from three to one. Hard balances should yield no complete feasible flow, not silently discard an order unit. Restore capacity and add a second service with a distinct ID and cost. Shortage requires an explicit bounded and costed relaxation.

Continue to the [next business scenario](./business-scenario) and [integration checks](../integration). Consult [repository verification](../verification) when checking source behavior; test commands are not the application entry point.
