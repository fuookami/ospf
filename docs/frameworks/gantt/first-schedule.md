# Your First Activity Schedule: Modes, Dependencies and Resources

Work through business input, solving and result extraction with this example. Prepare dependencies and backends using [Setup](../setup).

## 1. Business Scenario

Schedule activities a and b with a finishing before b starts. Activity a offers fast (2 h, machine, cost 3) or slow (3 h, alternate, cost 1). Activity b needs 2 h on machine at cost 0. Modes may start only at hour 0 or 2; both resources have capacity one and activities are mandatory.

Slow finishes no earlier than hour 3, but b has no start at or after 3. A feasible choice is a/fast at 0 and b at 2, costing 3. Both examples use the same relative times; Kotlin has a fixed business date and Rust uses UNIX_EPOCH as a teaching origin.

## 2. Construct Input and Call the Framework

::: info Kotlin and Rust Entry Points
Use the finite-choice single-MILP service without implementing task-bunch column generation. Resource capacities/usages and costs are dimensionless teaching values here; declare business units for production. Kotlin returns an outer `Ret` and a scheduling report; Rust retains a report with an optional business solution.
:::

The examples include imports and error handling. The application supplies Kotlin `ColumnGenerationSolver` to `runExample`; the next section shows the launcher.

::: code-group

<<< ../_examples/gantt.kt [Kotlin]

<<< ../_examples/gantt.rs [Rust]

:::

## 3. Launch from Your Application

Place the Kotlin example in application sources, retaining its package. This standalone launcher calls the lesson function. Add the domain starter, Gurobi plugin and coroutines, and configure matching native libraries and a license.

```kotlin
import docs.examples.gantt.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

Place the Rust example in `src/gantt.rs` and use this launcher in `src/main.rs`. See Setup for dependencies and toolchain; do not enable `async` for these synchronous examples.

```rust
mod gantt;
use ospf_rust_framework::solver::ScipColumnGenerationSolver;

fn main() -> Result<(), String> {
    gantt::run_example(ScipColumnGenerationSolver::new())
}
```

At the CLI boundary these examples throw or return errors. A server should translate them at its request boundary and retain diagnostics. Preserve reports when no feasible solution was found rather than inventing a technical failure or an empty successful plan.

## 4. Read and Validate the Result

| Read | Kotlin | Rust |
| --- | --- | --- |
| Task, mode and time | `schedule.placements → key.task / key.mode / time.start / time.end` | `schedule.placements → key.task / key.mode / time.start / time.end` |
| Actual reservations | `placement.reservations → resource / time / quantity` | `placement.reservations → resource / time / quantity` |
| Cost and retained facts | `cost / cancelled / executions / materialInventory` | `cost / cancelled / executions / material_inventory` |
| Solve status and candidate scope | `outcome.report`, `schedule.candidateScope` | `report`, `schedule.candidate_scope` |

::: info Result Semantics
Candidate scope records declared allowed choices; it is not `complete` or an optimality flag. Retain the solve report when no schedule exists. A returned schedule still needs cancellation, dependency and whole-interval resource checks.
:::

Outer success does not ensure a schedule. Read the report before the optional business solution. Verify a mode, start/end times, precedence, resource usage and recomputed cost. Allowed starts define feasibility; a longer solve budget does not create a start at hour 3.

## 5. Change One Business Condition

Add hour 3 to b allowed starts only. You can now examine a/slow at 0 and b at 3, costing 1. Add machine downtime next, leaving freezing and rolling recovery for a later step.

Continue to the [next business scenario](./business-scenario) and [integration checks](../integration). Consult [repository verification](../verification) when checking source behavior.
