# OSPF Rust Framework

:us: English | :cn: [简体中文](README_ch.md)

## Introduction

`ospf-rust-framework` is the shared framework layer for OSPF Rust domain frameworks. It provides high-level solver abstractions, pipeline modeling, shadow-price utilities, dynamic model lifecycle helpers, persistence expression contracts, remote solver client infrastructure, network helpers, and running heartbeat data structures.

## Scope

This crate covers:

1. Solver abstractions for column generation, Benders decomposition, linear/quadratic solving, and solver combinators.
2. Pipeline modeling for constraints, column generation, heuristic analysis, shadow prices, and dynamic model lifecycle.
3. Persistence expression repository contracts and feature-gated backend translators.
4. Remote solver domain models, ports, clients, HTTP transport, object storage, and model serialization.
5. Network utilities, running heartbeat structures, and framework solve options.

Explicit non-goals:

1. Domain-specific modeling such as cutting plans, packing layers, or scheduling tasks.
2. Low-level variable/token/symbol/model standard-form ownership; those belong in `ospf-rust-core`.
3. Solver installation and license management beyond backend setup notes.

## Module Structure

| Rust module | Kotlin boundary | Responsibility |
| --- | --- | --- |
| `model` | `framework/model` | `Pipeline`, `CGPipeline`, `HAPipeline`, shadow-price maps, dynamic column state, and dynamic model lifecycle. |
| `solver` | `framework/solver` | Column generation, Benders, combinatorial solvers, framework solve options, dual solutions, and backend extension wrappers. |
| `solver::remote` | `framework/solver.remote` | Remote solver client, domain models, ports, adapters, HTTP task client, and model serialization. |
| `persistence` | `framework/persistence` | Persistence DTOs, repository contracts, expression schema, sort/update descriptors, and backend feature boundaries. |
| `network` | `framework/network` | HTTP/network helper contracts. |
| `running_heart_beat` | `framework heartbeat` | Progress, running, and finish heartbeat data structures. |

## Architecture Overview

The framework crate sits between `ospf-rust-core` and domain framework crates:

1. `core` owns modeling primitives and solver-facing standard forms.
2. `framework` composes those primitives into reusable solve orchestration, pipeline, shadow-price, and lifecycle abstractions.
3. domain crates such as CSP1D, BPP3D, and Gantt Scheduling assemble context / aggregation / model component / pipeline structures on top of `framework`.

Application code should generally use unified `solve(...)` and `solve_with_options(...)` entries. Domain-specific constraints and objectives should be registered through pipelines or context extensions rather than application-level model assembly.

## Core Concepts

1. `Pipeline<M>` registers and executes a constraint or objective family for a model.
2. `CGPipeline<Args, Model, Map>` extends pipeline behavior with shadow-price refresh and reduced-cost extraction.
3. `ShadowPriceKey`, `ShadowPrice`, and `ShadowPriceMap` provide stable dual mapping for column generation.
4. `DynamicModelLifecycle` tracks warm starts, hidden/fixed/removed columns, solver solutions, and column range restoration.
5. `FrameworkSolveOptions` centralizes solve name, logging, solution amount, conversion policy, callback, and algorithm tuning options.

## Public API

| API | Responsibility | Stability |
| --- | --- | --- |
| `Pipeline`, `PipelineList` | Constraint/objective pipeline execution. | stable within migration |
| `CGPipeline`, `BasicShadowPriceMap`, `ShadowPriceMap` | Column-generation shadow-price lifecycle. | stable within migration |
| `DynamicModelLifecycle`, `DynamicColumnContext`, `ColumnState` | Dynamic column lifecycle and warm-start state. | migration |
| `ColumnGenerationSolver` | Framework column-generation solver trait. | migration |
| `LinearBendersDecompositionSolver`, `QuadraticBendersDecompositionSolver` | Benders decomposition solver traits. | migration |
| `SerialCombinatorial*`, `ParallelCombinatorial*` | Solver combinators. | migration |
| `FrameworkSolveOptions` | Unified solve options. | migration |
| `RemoteSolverClient`, `RemoteLinearSolver`, `RemoteQuadraticSolver` | Remote solver orchestration when feature-gated. | migration |
| `ExpressionRepository`, `RepositoryQuery`, `SortBy`, `UpdateAssignments` | Persistence expression contracts. | migration |

## Modeling Extensions

Framework-level extensions should be expressed as:

1. `Pipeline` for ordinary constraint/objective registration.
2. `CGPipeline` for shadow-price-aware constraints and reduced-cost extraction.
3. `HAPipeline` for heuristic analysis and validation.
4. `DynamicModelLifecycle` for column hiding, fixing, removal, restoration, and warm starts.
5. solver wrappers or combinators for backend selection and fallback.

Domain crates should expose their own context / aggregation / pipeline extension points and use framework abstractions rather than adding domain logic to application solvers.

## Generic Numeric Boundaries

Framework Meta entries accept `MetaModel<V>` and convert to backend numeric domains with `SolveValueConversionPolicy`:

- `Strict` rejects lossy conversions.
- `AllowRounding` allows controlled rounding to `f64`.

Current value-type support includes `f64`, `BigRational` through `big-rational`, and `BigDecimal` through `big-decimal`. Conversion failures are explicit and should not silently downgrade precision.

## Solve Lifecycle

Common framework usage:

1. Domain context registers a `MetaModel`.
2. Application service chooses a solver or solver combinator.
3. `FrameworkSolveOptions` carries model-independent solve options.
4. Solver traits call core model lowering and backend adapters.
5. LP duals, Benders sub-results, or column-generation shadow prices are returned through framework result types.
6. Dynamic lifecycle state can be written back into `MetaModel` before final MILP solves when supported.

## Usage

```rust
use ospf_rust_core::model::MetaModel;
use ospf_rust_framework::solver::{ColumnGenerationSolver, FrameworkSolveOptions};

fn run_column_generation<S: ColumnGenerationSolver>(
    solver: &S,
    meta_model: &MetaModel<f64>,
) -> ospf_rust_core::error::Result<()> {
    let _result = solver.solve(meta_model)?;

    let options = FrameworkSolveOptions::new()
        .with_name("cg_case")
        .with_log_model(true);
    let _result_with_options = solver.solve_with_options(meta_model, options)?;
    Ok(())
}
```

The same pattern is available for linear/quadratic MetaModel solver extensions and linear/quadratic Benders solver entries.

## Feature Flags

- `async`: enable async/await support for solver operations.
- `nightly`: forward nightly callable support from `ospf-rust-core`.
- `big-rational`: enable `BigRational` `MetaModel` value type support.
- `big-decimal`: enable `BigDecimal` `MetaModel` value type support.
- `gurobi`, `gurobi10`, `gurobi11`, `gurobi12`: enable Gurobi backend forwarding through core.
- `scip`, `scip-bundled`, `scip-from-source`, `scip-quadratic`: enable SCIP backend forwarding through core.
- `persistence*`: enable persistence contracts and backend-specific translators.
- `remote-solver`: enable async remote solver domain models, ports, client orchestration, local object storage, and model serializers.
- `remote-solver-http-reqwest`: enable reqwest-based HTTP transport.

## Remote Solver Boundary

Remote solver support follows Cargo features instead of Maven-style backend modules. The common feature provides `RemoteSolverClient`, `RemoteLinearSolver`, `RemoteQuadraticSolver`, async `SolverExecutionPort` and `ObjectStoragePort`, `RemoteSolverHttpClient`, `LocalFileObjectStoragePort`, and `OspfRemoteModelSerializer`.

Inside a current-thread Tokio runtime, prefer `solve_remote(...)` / `solve_remote_with_options(...)` because the synchronous trait entry intentionally refuses to block that runtime flavor.

## Persistence Boundary

Persistence backends follow Cargo features. The public common layer contains `ExpressionRepository`, `RepositoryQuery`, `SortBy`, `UpdateAssignments`, `PredicateSchema`, `FieldPath`, and request/response DTO/record types.

Backend scope:

1. SQLx builds parameterized SQL statements.
2. SeaORM translates expressions into SeaQuery/SeaORM types and provides an async repository adapter.
3. Rbatis reuses the parameterized SQL builder under Rbatis-facing names.
4. Diesel and Toasty keep typed ORM boundaries explicit through planning helpers.
5. Cornucopia binds generated query function names.
6. MongoDB and Redis provide JSON/document and command helpers without requiring a concrete client type.

## Solver Backend Notes

Framework backend adapters are compiled only when their feature is enabled on `ospf-rust-framework`, and the feature is forwarded to `ospf-rust-core`.

Framework extensions are in:

1. `src/solver/gurobi_extension.rs`
2. `src/solver/scip_extension.rs`

Core backend setup details:

1. Gurobi: [`../ospf-rust-core/src/solver/solvers/gurobi/README.md`](../ospf-rust-core/src/solver/solvers/gurobi/README.md)
2. SCIP: [`../ospf-rust-core/src/solver/solvers/scip/README.md`](../ospf-rust-core/src/solver/solvers/scip/README.md)

For LP and linearized subproblems, framework first uses backend-provided row duals/Farkas certificates when they are present and consistent. For true quadratic subproblems, backend support for complete dual/Farkas certificates is limited, so a result may be returned without a complete quadratic Benders cut.

## Local Validation

```powershell
cargo check -p ospf-rust-framework
cargo test -p ospf-rust-framework --no-run
cargo test -p ospf-rust-framework --lib
cargo check -p ospf-rust-framework --features async
cargo test -p ospf-rust-framework --features remote-solver remote
cargo check -p ospf-rust-framework --features "remote-solver remote-solver-http-reqwest"
```

Backend feature examples:

```powershell
cargo test -p ospf-rust-framework --features gurobi10
cargo test -p ospf-rust-framework --features scip
cargo test -p ospf-rust-framework --features "scip-bundled async"
```

## Dependencies

- `ospf-rust-core`
- `ospf-rust-base`
- `parking_lot`
- `log`
- `async-trait` optional
- `tokio`, `serde`, `serde_json`, `sha2`, `reqwest`, and `sea-orm` optional by feature

## Related Modules

- [Root README](../README.md)
- [Core README](../ospf-rust-core/README.md)
- [Kotlin framework README](../../ospf-kotlin/ospf-kotlin-framework/README.md)

## License

This project is licensed under the same license as the main OSPF Rust project.
