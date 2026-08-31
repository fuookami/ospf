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
| `persistence` | `framework/persistence` | Persistence DTOs, repository contracts, expression schema, relational query plans, sort/update descriptors, and backend feature boundaries. |
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
| `RelationalQueryPlan`, `JoinSpec`, `ProjectionSpec` | Database-independent relational query planning with validation and canonical audit summaries. | migration |
| `DiagnosticPersistenceFieldResolver`, `PersistenceFieldResolution` | Preserve missing, ambiguous, and invalid field mapping diagnostics. | migration |
| `SqlxRelationalQueryCompiler` | Compile allowlisted relational plans into parameterized SQLx SQL and execution statistics. | feature-gated |

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

The shared report/proof/cancellation contract is documented in
the [Unified Solve Contract](../ospf-rust-core/README.md#unified-solve-contract).

Remote checkpoint recovery keeps the legacy `CheckpointResumeExpectation` and
`validate_resume` compatibility projections. Exact resume must use
`CheckpointResumeExpectationWithAttempt`, `load_checkpoint_artifact_from`, and
`validate_resume_from`, which validate the source attempt, expected parent,
solver provenance, and cancellation chain. A versioned report carrying a
checkpoint must also match its model/configuration/solver fingerprints and
provenance; matching only run and attempt IDs is insufficient.
Use the [native matrix](../ospf-rust-core/README.md#native-validation-matrix) for backend evidence and
the [traceability table](../ospf-rust-core/README.md#source-traceability) for Kotlin source coverage.

Remote solver support follows Cargo features instead of Maven-style backend modules. The common feature provides `RemoteSolverClient`, `RemoteLinearSolver`, `RemoteQuadraticSolver`, async `SolverExecutionPort` and `ObjectStoragePort`, `RemoteSolverHttpClient`, `LocalFileObjectStoragePort`, and `OspfRemoteModelSerializer`.

Inside a current-thread Tokio runtime, prefer `solve_remote(...)` / `solve_remote_with_options(...)` because the synchronous trait entry intentionally refuses to block that runtime flavor. New code that needs terminal semantics should use `solve_remote_report(...)` or the core `solve_linear_report` / `solve_quadratic_report` entries. Versioned remote reports preserve the core `SolveReport` together with run/attempt identity and artifact digest; legacy `SolveResult` and `SerializedSolution` remain read-compatible, while unknown report schema versions are rejected.

## Persistence Boundary

Persistence backends follow Cargo features. The public common layer contains `ExpressionRepository`, `RepositoryQuery`, `SortBy`, `UpdateAssignments`, `PredicateSchema`, `FieldPath`, and request/response DTO/record types.

`RelationalQueryPlan` is the database-independent planning boundary for sources, aliases, joins, predicates, projections, grouping, ordering, pagination, and optional root keys. Plans recursively snapshot owned expression trees and dynamic symbol metadata at the plan boundary, validate join correlations before compilation, and expose `canonical()` plus a SHA-256 `canonical_hash()` for audit correlation. Canonical keys normalize nested boolean expressions and membership candidates while retaining literal type shape rather than literal values. The plan remains intentionally independent of permissions, budgets, physical table names, database connections, and row-mapping types.

Field mappings should implement `DiagnosticPersistenceFieldResolver` when the adapter must distinguish a missing field from an ambiguous mapping or invalid configuration. `PredicateSchema<String>` supplies this behavior for registered string mappings.

Backend scope:

1. SQLx builds parameterized SQL statements through `SqlxRelationalQueryCompiler`. `Inner`, `Left`, and correlated `Exists` joins are supported; implicit projections use an explicit source allowlist, and `COUNT(DISTINCT root_key)` preserves root granularity for one-to-many relationships. `compile_count` keeps the compatibility `SqlxSql` result, while `compile_count_typed` preserves parameter types and ordering for audit or adapter-side typed binding.
2. SeaORM translates expressions into SeaQuery/SeaORM types and provides an async repository adapter.
3. Rbatis reuses the parameterized SQL builder under Rbatis-facing names.
4. Diesel and Toasty keep typed ORM boundaries explicit through planning helpers.
5. Cornucopia binds generated query function names.
6. MongoDB and Redis provide JSON/document and command helpers without requiring a concrete client type.

The SQLx compiler returns `SqlxCompiledQuery` with a SQL template, parameter values, parameter type summary, dialect, and the copied plan. `execute_with` deliberately accepts an external executor because connection ownership and row mapping belong to the application or SQLx adapter; it only adds returned-row, duration, and truncation statistics. Adapters that need structured execution categories should use `execute_with_classifier` and explicitly map executor errors to `Timeout`, `UnsupportedDialect`, or `Database`; the generic layer does not infer categories from error text. Unknown sources, unknown or ambiguous columns, invalid joins, unsupported predicates, and malformed allowlists are returned as structured `RelationalQueryFailure` values.

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
- `tokio`, `serde`, `serde_json`, `reqwest`, and `sea-orm` optional by feature; `sha2` is required for query audit hashes

## Related Modules

- [Root README](../README.md)
- [Core README](../ospf-rust-core/README.md)
- [Kotlin framework README](../../ospf-kotlin/ospf-kotlin-framework/README.md)

## License

This project is licensed under the same license as the main OSPF Rust project.
