# Your First 1D Packing: Items, Gaps and Stock

Start with a small batch of bars and work through input, solving and business result extraction. Prepare dependencies and backends using [Setup](../setup).

## 1. Business Scenario

Pack three bars, each 40 cm long and 2 kg. One small container is 1 m long with a 5 kg limit and cost 1; one large container is 2 m long with a 10 kg limit and cost 3. Leave 20 cm between adjacent items and minimize container count.

All three together require $3 \times 40\text{ cm} + 2 \times 20\text{ cm} = 160\text{ cm} = 1.6\text{ m}$ and weigh 6 kg. Both length and weight exceed small's limits; large can carry them in one container.

::: tip Counts, Units and Zero Stock
Item `amount` is exact demand. Kotlin keeps mixed cm/m inputs and converts through its adapter; Rust normalizes to m/kg. Null stock (`null` / `None`) means unlimited; zero means unavailable.
:::

## 2. Construct Input and Call the Framework

Kotlin calls `Bpp1dMilp.solve`. Rust creates a validated `PackingContext` and forwards the actual model, remaining time and cancellation handle to an injected `LinearSolver`.

The examples include imports and error handling. The application supplies a solver to `runExample` / `run_example`; the next section shows launchers.

::: code-group

<<< ../_examples/bpp1d.kt [Kotlin]

<<< ../_examples/bpp1d.rs [Rust]

:::

## 3. Launch from Your Application

Place the Kotlin example in application sources, retaining its package. This standalone launcher calls the lesson function. Add the domain starter, Gurobi plugin and coroutines, and configure matching native libraries and a license.

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

At the CLI boundary these examples throw or return errors. A server should translate them at its request boundary and retain diagnostics. Preserve reports when no feasible solution was found rather than inventing a technical failure or an empty successful plan.

## 4. Read and Validate the Result

The key extraction below is already included in section 2. Kotlin runs it in the `Ok` branch; Rust runs it in `Some(packing)` and retains the original `context` for type lookup and KPIs.

::: code-group

<<< ../_examples/bpp1d.kt#result [Kotlin]

<<< ../_examples/bpp1d.rs#result [Rust]

:::

A one-container plan uses large. With coordinates normalized to meters, inspect:

| Field | Expected value |
| --- | --- |
| Item intervals | a: `[0, 0.4)`, `[0.6, 1.0)`, `[1.2, 1.6)` |
| Internal gaps | `[0.4, 0.6)`, `[1.0, 1.2)` |
| Tail | `[1.6, 2.0)`, length 0.4 m |
| KPIs | 1 container, 3 items, consumed container length 2 m, cost 3, utilization 0.6 |

::: info Free Intervals and Completeness
Kotlin's `remnants` contains tails only; the full example derives internal gaps with `placements.zipWithNext()`. Rust's `free_intervals` includes both. Required gaps are reserved space, not freely reusable capacity. `candidateComplete` / `candidate_complete` describes enumeration; combine it with status/report to interpret optimality.
:::

The full example also groups placements by item ID and compares every count with input `amount`. Preserve container type IDs, item intervals, KPIs and solve status in your application DTO for execution and traceability.

## 5. Change One Business Condition

Set large inventory to zero. The remaining small container cannot cover three items; report no complete feasible packing rather than reuse the previous result. Restore stock, switch to `Cost`, and compare cost with container count.

Continue to the [next business scenario](./business-scenario) and [integration checks](../integration). Consult [repository verification](../verification) when checking source behavior.
