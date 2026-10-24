# Your First 1D Packing: Items, Gaps and Stock

Write an application example directly; running repository tests is not a prerequisite. Prepare dependencies and backends using [Setup](../setup).

## 1. Business Scenario

Pack three bars, each 40 cm long and 2 kg. One small container is 1 m long with a 5 kg limit; one large container is 2 m long with a 10 kg limit. Leave 20 cm between adjacent items and minimize container count.

Both versions model the same instance. Kotlin keeps mixed cm/m inputs and converts through an adapter; Rust explicitly uses m/kg. Item amount is an exact demand. Zero inventory is not unlimited stock.

## 2. Construct Input and Call the Framework

Kotlin calls `Bpp1dMilp.solve`. Rust creates a validated `PackingContext` and forwards the actual model, remaining time and cancellation handle to an injected `LinearSolver`. Neither example uses a fixture solver or a preset solution.

The following example functions include their imports and contain no test harness. The application injects a real Kotlin `ColumnGenerationSolver`; the next section shows the launcher.

::: code-group

<<< ../_examples/bpp1d.kt [Kotlin]

<<< ../_examples/bpp1d.rs [Rust]

:::

## 3. Launch from Your Application

Place the Kotlin example in application sources, retaining its package. This standalone launcher calls the lesson function. Add the domain starter, Gurobi plugin and coroutines, and configure matching native libraries and a license. This is the production plugin, not a test-only delegating wrapper.

```kotlin
import docs.examples.bpp1d.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

Place the Rust example in `src/bpp1d.rs` and use this launcher in `src/main.rs`. See Setup for dependencies and toolchain; do not enable `async` for these synchronous examples.

```rust
mod bpp1d;
use ospf_rust_core::solver::solvers::scip::SCIPSolver;

fn main() -> Result<(), String> {
    bpp1d::run_example(&SCIPSolver::new())
}
```

At the CLI boundary these examples throw or return errors. A server should translate them at its request boundary and retain diagnostics. Preserve normal no-incumbent reports rather than inventing a technical failure or an empty successful plan.

## 4. Read and Validate the Result

A feasible one-container plan uses large, with starts at 0, 60 and 120 cm, ending at 160 cm, and a total weight of 6 kg. There is no trailing gap. Read the status/report, count every item ID and verify dimensions, weight and stock rather than checking objective 1 alone.

## 5. Change One Business Condition

Set large inventory to zero. The remaining small container cannot cover three items; report no complete feasible packing rather than reuse the previous result. Restore stock, switch to `Cost`, and compare cost with container count.

Continue to the [next business scenario](./business-scenario) and [integration checks](../integration). Consult [repository verification](../verification) when checking source behavior; test commands are not the application entry point.
