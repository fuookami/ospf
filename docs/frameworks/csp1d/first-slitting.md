# Your First Slitting Plan: Demand and Plan Usage

Work through business input, solving and result extraction with this example. Prepare dependencies and backends using [Setup](../setup).

## 1. Business Scenario

Produce 0.8 m wide strips with demand of three rolls from stock with width range 0.5–1.5 m, without a machine configuration. Three is a roll count, not 3 m or three pattern usages. Rust additionally limits available stock batches to three.

Declare roll demand explicitly with `ProductDemand.legacyRoll` / `legacy_roll`. Production is each pattern contribution multiplied by its usage. Three generated candidates do not mean three produced rolls.

## 2. Construct Input and Call the Framework

::: info Kotlin and Rust Entry Points
Kotlin uses an actual injected solver and returns a structured `Csp1dSolution` directly, not `Ret`. The Rust default entry needs no native optimizer but is heuristic. The name `Milp` does not establish native MILP execution; configure the master-solver interface separately when needed.
:::

The examples include imports and error handling. The application supplies Kotlin `ColumnGenerationSolver` to `runExample`; the next section shows the launcher.

::: code-group

<<< ../_examples/csp1d.kt [Kotlin]

<<< ../_examples/csp1d.rs [Rust]

:::

## 3. Launch from Your Application

Place the Kotlin example in application sources, retaining its package. This standalone launcher calls the lesson function. Add the domain starter, Gurobi plugin and coroutines, and configure matching native libraries and a license.

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

At the CLI boundary these examples throw or return errors. A server should translate them at its request boundary and retain diagnostics. Preserve reports when no feasible solution was found rather than inventing a technical failure or an empty successful plan.

## 4. Read and Validate the Result

| Read | Kotlin | Rust |
| --- | --- | --- |
| Plans and usage counts | `solution.produce.cuttingPlans → plan / amount` | `solution.produce.cutting_plans → plan / amount` |
| Material and remaining width | `plan.material.id / slices / restWidth` | `plan.material.id / slices / rest_width()` |
| Contribution per execution | `plan.demandContributions → product.id / quantity` | `plan.demand_contributions → product.id / quantity` |
| Shortfall and status | `produce.unmetDemands`, `status / failureMessage / kpi` | `produce.unmet_demands`, `status / failure_message / kpi` |

::: info Result Semantics
Aggregate production as `Σ contribution per execution × usage`. Plan count is not production. Remaining-width helpers report width, not placement intervals. There is no pool `complete`; inspect status, shortfalls and column-generation traces.
:::

Read status, failure and unmet demand before selected plans and usages. Recalculate P1 production in rolls and verify pattern widths and available batches. A nonempty candidate pool does not establish coverage; partial output is not a complete delivery.

## 5. Change One Business Condition

Change demand from three rolls to four and check stock and contributions. Next try weight demand: changing a label to kg is insufficient; supply unit weight, roll length and conversion data, then update acceptance criteria.

Continue to the [next business scenario](./business-scenario) and [integration checks](../integration). Consult [repository verification](../verification) when checking source behavior.
