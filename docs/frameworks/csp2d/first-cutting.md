# Your First Cutting Plan: Demand, Patterns and Process

Write an application example directly; running repository tests is not a prerequisite. Prepare dependencies and backends using [Setup](../setup).

## 1. Business Scenario

Cut rectangular products from fixed sheets. Kotlin uses a 100 × 60 m sheet, 40 × 30 m products and demand of three. Rust uses 10 × 8 m sheets, 2 × 2 m products, demand of twenty and stock of one hundred sheets. Start with Freeform and zero kerf. These are hand-checkable teaching dimensions, not production specifications.

Both inputs admit a manually constructed one-sheet layout. Kotlin explicitly declares product demand and process; Rust uses `Csp2dProblem::single`. They are different numerical instances.

## 2. Construct Input and Call the Framework

Kotlin `Csp2dMilp` uses a real injected solver. Default Rust `Csp2dSolvers::new().solve_milp` uses a heuristic master and needs no native backend. Its name establishes neither native execution nor optimality. Use `Csp2dMasterSolver` for a real optimizer integration.

The following example functions include their imports and contain no test harness. The application injects a real Kotlin `ColumnGenerationSolver`; the next section shows the launcher.

::: code-group

<<< ../_examples/csp2d.kt [Kotlin]

<<< ../_examples/csp2d.rs [Rust]

:::

## 3. Launch from Your Application

Place the Kotlin example in application sources, retaining its package. This standalone launcher calls the lesson function. Add the domain starter, Gurobi plugin and coroutines, and configure matching native libraries and a license. This is the production plugin, not a test-only delegating wrapper.

```kotlin
import docs.examples.csp2d.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

Place the Rust example in `src/csp2d.rs` and use this launcher in `src/main.rs`. See Setup for dependencies and toolchain; do not enable `async` for these synchronous examples.

```rust
mod csp2d;

fn main() -> ospf_rust_base::error::Ret<()> {
    csp2d::run_example()
}
```

At the CLI boundary these examples throw or return errors. A server should translate them at its request boundary and retain diagnostics. Preserve normal no-incumbent reports rather than inventing a technical failure or an empty successful plan.

## 4. Read and Validate the Result

Check demand coverage, stock consumption, geometry and manufacturing process separately. Pattern count is not sheet usage. Freeform needs no cut tree; guillotine requires a valid tree with consistent stages, regions and kerf. The Rust hand-check baseline covers twenty products on one sheet with zero net-area waste; independently verify actual output.

## 5. Change One Business Condition

Add positive kerf first: an exactly filled layout can become infeasible. Then select two-stage guillotine and regenerate patterns instead of relabelling old coordinates. Switch to coils last and interpret consumption as length rather than sheet count.

Continue to the [next business scenario](./business-scenario) and [integration checks](../integration). Consult [repository verification](../verification) when checking source behavior; test commands are not the application entry point.
