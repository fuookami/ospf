# OSPF Rust Framework

A framework layer for the OSPF (Operational Research Solver Framework) Rust implementation, providing high-level solver abstractions and model management utilities.

:us: English | :cn: [简体中文](README_ch.md)

## Overview

`ospf-rust-framework` provides:

- **Model Layer**: Pipeline management and Shadow Price utilities for column generation algorithms
- **Solver Layer**: Solver abstraction traits and combinators for linear and quadratic programming

## Features

### Model Module

- **Pipeline (`pipeline.rs`)**: `CGPipeline` trait for column generation pipeline management
- **Shadow Price (`shadow_price.rs`)**: Shadow price data structures and management utilities

### Solver Module

- **Column Generation (`column_generation_solver.rs`)**: Core trait definitions for column generation solvers
- **Parallel Mode (`parallel_combinatorial_mode.rs`)**: Parallel execution mode configuration
- **Linear Solvers**: 
  - `parallel_combinatorial_linear_solver.rs` - Parallel linear solver combinator
  - `serial_combinatorial_linear_solver.rs` - Serial linear solver combinator
- **Quadratic Solvers**:
  - `parallel_combinatorial_quadratic_solver.rs` - Parallel quadratic solver combinator
  - `serial_combinatorial_quadratic_solver.rs` - Serial quadratic solver combinator
- **Column Generation Solvers**:
  - `parallel_combinatorial_column_generation_solver.rs` - Parallel column generation solver combinator
  - `serial_combinatorial_column_generation_solver.rs` - Serial column generation solver combinator
- **Benders Decomposition (`linear_benders_decomposition_solver.rs` / `quadratic_benders_decomposition_solver.rs`)**: Benders decomposition algorithm support
- **Remote Solver (`remote-solver` feature)**: async remote solve client, HTTP task client, local object storage, and OSPF model serialization helpers

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
ospf-rust-framework = { path = "path/to/ospf-rust-framework" }
```

### Unified Solve API (Recommended)

For framework users, prefer `solve(...)` for the shortest path and
`solve_with_options(...)` when non-model arguments are required.
Preferred option type is `FrameworkSolveOptions` (`SolveOptions` remains as compatibility alias).

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

The same pattern is available for:
- `LinearMetaModelSolverExt`
- `QuadraticMetaModelSolverExt`
- `LinearBendersDecompositionSolver`
- `QuadraticBendersDecompositionSolver`

MetaModel-oriented entries:
- `solve_meta(...)` / `solve_meta_with_options(...)` for linear & quadratic meta-model extensions
- `solve_meta(...)` / `solve_meta_with_options(...)` for linear Benders meta-model entry
- `solve_meta_quadratic(...)` / `solve_meta_quadratic_with_options(...)` for quadratic Benders meta-model entry

### Value Types and Precision Policy

Meta-level solver entries accept `MetaModel<V>` and convert to backend numeric domain with
`SolveValueConversionPolicy`:

- `Strict` (default): reject lossy conversions
- `AllowRounding`: allow controlled rounding to `f64`

Current value-type support:
- `f64` (default, no extra feature)
- `BigRational` (`big-rational` feature)
- `BigDecimal` (`big-decimal` feature)

Conversion failures are explicit (no silent downgrade), typically surfaced as:
- `NonFinite`
- `PrecisionLoss`
- `Overflow`

Example:

```rust
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::SolveValueConversionPolicy;
use ospf_rust_framework::solver::{ColumnGenerationSolver, FrameworkSolveOptions};

fn run_with_policy<S: ColumnGenerationSolver>(
    solver: &S,
    meta_model: &MetaModel<f64>,
) -> ospf_rust_core::error::Result<()> {
    let options = FrameworkSolveOptions::new()
        .with_value_conversion_policy(SolveValueConversionPolicy::Strict);
    let _result = solver.solve_with_options(meta_model, options)?;
    Ok(())
}
```

### Interface Convergence Notes

Public application-facing usage is converged to:
- `solve(...)`
- `solve_with_options(...)`

For Benders and solver-extension trait paths, `solve_meta*` methods are still available for
explicit MetaModel-oriented flows, but application code should prefer the two unified entries above.

### Default Validation Commands

```bash
cargo check -p ospf-rust-framework
cargo test -p ospf-rust-framework --no-run
cargo test -p ospf-rust-framework --lib
```

If async path is touched, also run:

```bash
cargo check -p ospf-rust-framework --features async
```

Remote solver code is feature gated and is not covered by default framework tests. Run:

```bash
cargo test -p ospf-rust-framework --features remote-solver remote
cargo check -p ospf-rust-framework --features "remote-solver remote-solver-http-reqwest"
```

### Feature Flags

- `async` - Enable async/await support for solver operations
- `nightly` - Forward nightly callable support from `ospf-rust-core`
- `big-rational` - Enable `BigRational` MetaModel value type support (forwarded to core)
- `big-decimal` - Enable `BigDecimal` MetaModel value type support (forwarded to core)
- `gurobi` - Alias for `gurobi10`
- `gurobi10`, `gurobi11`, `gurobi12` - Enable the matching Gurobi backend in core
- `scip` - Enable the SCIP backend in core
- `scip-bundled` - Enable bundled SCIP through core
- `scip-from-source` - Build SCIP from source through core
- `scip-quadratic` - Enable SCIP quadratic support through core
- `persistence` - Enable common expression repository contracts, sort/update descriptors, DTO/record types
- `persistence-sqlx`, `persistence-sqlx-postgres`, `persistence-sqlx-mysql`, `persistence-sqlx-sqlite` - Enable parameterized SQL statement builders
- `persistence-sea-orm` - Enable SeaORM translator and async repository adapter
- `persistence-diesel`, `persistence-diesel-postgres`, `persistence-diesel-mysql`, `persistence-diesel-sqlite` - Enable Diesel typed field planning helpers
- `persistence-rbatis` - Enable Rbatis-named parameterized SQL statement builders
- `persistence-toasty` - Enable Toasty typed repository plans
- `persistence-cornucopia` - Enable Cornucopia generated-query binding helpers
- `persistence-mongodb` - Enable MongoDB JSON filter/update helpers
- `persistence-redis` - Enable Redis key/value command helpers
- `remote-solver` - Enable async remote solver domain models, ports, client orchestration, local file object storage, and OSPF model serializers
- `remote-solver-http-reqwest` - Enable the reqwest-based HTTP transport for the remote task client

### Remote Solver Notes

Remote solver support follows Cargo features instead of Maven-style backend modules. The common
feature provides:

- `RemoteSolverClient`, `RemoteLinearSolver`, `RemoteQuadraticSolver`
- async `SolverExecutionPort` and `ObjectStoragePort`
- `RemoteSolverHttpClient` with pluggable transport and Kotlin-compatible task routes
- `LocalFileObjectStoragePort` with path escape protection, metadata sidecar, and SHA-256 etag
- `OspfRemoteModelSerializer` for `LinearTriadModel` and `QuadraticTetradModel`
- helpers for `SerializedSolution` / `SolveResult` to `SolverOutput`

`RemoteLinearSolver` and `RemoteQuadraticSolver` implement the core synchronous solver traits for
drop-in use. Inside a current-thread Tokio runtime, prefer `solve_remote(...)` /
`solve_remote_with_options(...)` because the synchronous trait entry intentionally refuses to block
that runtime flavor.

For `RemoteSolverHttpExecutionPort`, the current HTTP task API assigns the canonical task id on the
server side. The caller-provided `task_id` is used as `requestId` and payload-path correlation key,
and the returned handle stores the server task id. The current `/resume` route resumes the task's
latest checkpoint; explicit checkpoint selection requires server-side API support.

The reqwest transport is only compiled with `remote-solver-http-reqwest`:

```toml
[dependencies]
ospf-rust-framework = { path = "../ospf-rust-framework", features = ["remote-solver-http-reqwest"] }
```

```rust,ignore
use std::time::Duration;
use ospf_rust_framework::solver::{
    RemoteSolveContext, RemoteSolveOptions, RemoteSolverClient,
};

let options = RemoteSolveOptions::new()
    .with_quantum(Duration::from_secs(4))
    .with_max_rounds(64);
```

### Persistence Backend Notes

Persistence backends follow Cargo features instead of Maven-style modules. The public common layer
contains `ExpressionRepository`, `RepositoryQuery`, `SortBy`, `UpdateAssignments`,
`PredicateSchema`, `FieldPath`, and request/response DTO/record types.

Backend scope:
- SQLx builds parameterized `SELECT` / `COUNT` / `UPDATE` / `DELETE` SQL statements.
- SeaORM translates expressions into SeaQuery/SeaORM types and provides an async repository adapter.
- Rbatis reuses the parameterized SQL builder under Rbatis-facing names.
- Diesel and Toasty keep typed ORM boundaries explicit through planning helpers.
- Cornucopia binds generated query function names instead of forcing dynamic SQL.
- MongoDB and Redis provide JSON/document and command helpers without requiring a concrete client type.

### SCIP Environment Notes

When enabling `scip`, besides `SCIPOPTDIR`, ensure `libclang` and SCIP runtime DLLs are discoverable:

- `LIBCLANG_PATH` should point to the directory containing `libclang.dll`
- `PATH` should include `<SCIPOPTDIR>\\bin` so test/runtime loading can find `libscip.dll`

### Solver Backend Notes (Gurobi / SCIP)

Framework follows the Rust feature model: backend adapters are compiled only when their feature is
enabled on `ospf-rust-framework`, and the feature is forwarded to `ospf-rust-core`.

Framework extensions are in:

1. `src/solver/gurobi_extension.rs`
2. `src/solver/scip_extension.rs`

Core backend setup details are documented in core module:

1. Gurobi: `../ospf-rust-core/src/solver/solvers/gurobi/README.md`
2. SCIP: `../ospf-rust-core/src/solver/solvers/scip/README.md`

### Benders Dual/Farkas Notes

For LP and linearized subproblems, framework first uses backend-provided row duals/Farkas
certificates when they are present and consistent. If LP row dual objective does not match the
primal objective, or if a Farkas certificate is missing, framework falls back to an explicit dual or
Farkas dual model.

For true quadratic subproblems, backend support for complete dual/Farkas certificates is limited.
When a quadratic subproblem cannot be represented as a linear surrogate and the backend does not
return usable multipliers, framework returns the feasible/infeasible result but may not be able to
generate a complete quadratic Benders cut for that iteration.

Feature examples for framework:

```toml
[dependencies]
ospf-rust-framework = { path = "../ospf-rust-framework", features = ["gurobi10"] }
# or
ospf-rust-framework = { path = "../ospf-rust-framework", features = ["scip-bundled"] }
```

```bash
# Gurobi 10 extension path
cargo test -p ospf-rust-framework --features gurobi10

# SCIP extension path
cargo test -p ospf-rust-framework --features scip

# SCIP bundled + async extension path
cargo test -p ospf-rust-framework --features "scip-bundled async"
```

Recommended framework imports:

```rust,ignore
use ospf_rust_framework::solver::{
    ColumnGenerationSolver, FrameworkSolveOptions,
    GurobiColumnGenerationSolver, ScipColumnGenerationSolver,
};
```

Common Kotlin-concept options are available with Rust-style builders:

```rust,ignore
let options = FrameworkSolveOptions::new()
    .with_solution_amount(5)
    .with_benders_iteration_limit(100)
    .with_benders_stall_iteration_limit(3);
```

Backend wrappers also forward the common tuning aliases:

```rust,ignore
let gurobi = GurobiColumnGenerationSolver::new()
    .with_gap(1e-4)
    .with_memory_limit_gb(8.0)
    .with_improve_threshold(1e-6);

let scip = ScipColumnGenerationSolver::new()
    .with_gap(1e-4)
    .with_memory_limit_mb(2048.0)
    .with_improve_threshold(1e-6)
    .with_lp_subproblem_defaults();
```

For ColumnGeneration/Benders subproblems, prefer `with_lp_subproblem_defaults()` on SCIP wrappers
when stable LP row duals are needed. It uses one thread and disables presolving and heuristics on
the subproblem backend config.

## Dependencies

- `ospf-rust-core` - Core data structures and traits
- `ospf-rust-base` - Base utilities
- `parking_lot` - High-performance synchronization primitives
- `log` - Logging facade
- `async-trait` (optional) - Async trait support

## License

This project is licensed under the same license as the main OSPF Rust project.
