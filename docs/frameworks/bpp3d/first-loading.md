# Your First 3D Loading: From Selection to Placement

Write an application example directly; running repository tests is not a prerequisite. Prepare dependencies and backends using [Setup](../setup).

## 1. Business Scenario

Start with one cuboid instead of combining cylinders, support rules and mixed bin types. Kotlin uses a 1 × 1 × 1 m carton and a 3 × 3 × 3 m bin. Rust CSV uses a 2 × 3 × 4 m item and a 10 × 10 × 10 m bin. These teach the same lifecycle, not a numerical cross-language benchmark.

A candidate layer is not a final loading plan. Kotlin supplies a seed with an actual placement and one empty bin, with no additional-layer search. Rust materializes input through the public CSV loader and connects two real model backends.

## 2. Construct Input and Call the Framework

The Kotlin example defines material, packaging attributes, orientation and coordinates locally and accepts a real solver. Rust requires the domain crate `serde` feature and uses synchronous APIs without `async`. Connect both RMP and final phases; do not replace them with preset objectives or empty executors.

The following example functions include their imports and contain no test harness. The application injects a real Kotlin `ColumnGenerationSolver`; the next section shows the launcher.

::: code-group

<<< ../_examples/bpp3d.kt [Kotlin]

<<< ../_examples/bpp3d.rs [Rust]

:::

## 3. Launch from Your Application

Place the Kotlin example in application sources, retaining its package. This standalone launcher calls the lesson function. Add the domain starter, Gurobi plugin and coroutines, and configure matching native libraries and a license. This is the production plugin, not a test-only delegating wrapper.

```kotlin
import docs.examples.bpp3d.runExample
import kotlinx.coroutines.runBlocking
import fuookami.ospf.kotlin.core.solver.gurobi.GurobiColumnGenerationSolver

fun main() = runBlocking {
    runExample(GurobiColumnGenerationSolver())
}
```

Place the Rust example in `src/bpp3d.rs` and use this launcher in `src/main.rs`. See Setup for dependencies and toolchain; do not enable `async` for these synchronous examples.

```rust
mod bpp3d;
use ospf_rust_core::solver::solvers::scip::SCIPConfig;
use ospf_rust_framework::solver::{ScipColumnGenerationSolver, meta_model_backend::ColumnGenerationSolverMetaModelBackend};

fn main() -> Result<(), String> {
    let config = SCIPConfig::new().with_output(false).with_time_limit(30.0);
    let rmp = ColumnGenerationSolverMetaModelBackend::new(
        ScipColumnGenerationSolver::with_config(config.clone()),
    ).with_lp_subproblem_defaults();
    let final_backend = ColumnGenerationSolverMetaModelBackend::new(
        ScipColumnGenerationSolver::with_config(config),
    );
    bpp3d::run_example(rmp, final_backend)
}
```

At the CLI boundary these examples throw or return errors. A server should translate them at its request boundary and retain diagnostics. Preserve normal no-incumbent reports rather than inventing a technical failure or an empty successful plan.

## 4. Read and Validate the Result

Distinguish candidates, selected layers and actual packed bins. Read Kotlin `packingSnapshot`; for Rust inspect selected layers and diagnostics such as `final_final_diagnostics`. A layer or render schema alone does not establish delivery. Verify exact item counts, orientation, boundaries, overlap and load limits. A finite seed pool limits the scope of any proof.

## 5. Change One Business Condition

When changing demand from one item to two, update empty-bin availability and candidate coverage as well. Add a second bin type next, and objectives last. The empty new-layer generator is not a general loading search algorithm.

Continue to the [next business scenario](./business-scenario) and [integration checks](../integration). Consult [repository verification](../verification) when checking source behavior; test commands are not the application entry point.
