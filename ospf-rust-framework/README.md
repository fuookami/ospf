# OSPF Rust Framework

[中文](README_ch.md) | English

A framework layer for the OSPF (Operational Research Solver Framework) Rust implementation, providing high-level solver abstractions and model management utilities.

## Overview

`ospf-rust-framework` provides:

- **Model Layer**: Pipeline management and Shadow Price utilities for column generation algorithms
- **Solver Layer**: Solver abstraction traits and combinators for linear and quadratic programming

## Features

### Model Module

- **Pipeline (`pipeline.rs`)**: `CGPipeline` trait for column generation pipeline management
- **Shadow Price (`shadow_price.rs`)**: Shadow price data structures and management utilities

### Solver Module

- **Column Generation (`column_generation.rs`)**: Core trait definitions for column generation solvers
- **Parallel Mode (`parallel_mode.rs`)**: Parallel execution mode configuration
- **Linear Solvers**: 
  - `parallel_linear.rs` - Parallel linear solver combinator
  - `serial_linear.rs` - Serial linear solver combinator
- **Quadratic Solvers**:
  - `parallel_quadratic.rs` - Parallel quadratic solver combinator
  - `serial_quadratic.rs` - Serial quadratic solver combinator
- **Column Generation Solvers**:
  - `parallel_column_generation.rs` - Parallel column generation solver combinator
  - `serial_column_generation.rs` - Serial column generation solver combinator
- **Benders Decomposition (`benders_decomposition.rs`)**: Benders decomposition algorithm support

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
ospf-rust-framework = { path = "path/to/ospf-rust-framework" }
```

### Unified Solve API (Recommended)

For framework users, prefer `solve(...)` for the shortest path and
`solve_with_options(...)` when non-model arguments are required.

```rust
use ospf_rust_core::model::MetaModel;
use ospf_rust_framework::solver::{ColumnGenerationSolver, SolveOptions};

fn run_column_generation<S: ColumnGenerationSolver>(
    solver: &S,
    meta_model: &MetaModel<f64>,
) -> ospf_rust_core::error::Result<()> {
    let _result = solver.solve(meta_model)?;

    let options = SolveOptions::new()
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
use ospf_rust_framework::solver::{ColumnGenerationSolver, SolveOptions};

fn run_with_policy<S: ColumnGenerationSolver>(
    solver: &S,
    meta_model: &MetaModel<f64>,
) -> ospf_rust_core::error::Result<()> {
    let options = SolveOptions::new()
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

### Feature Flags

- `async` - Enable async/await support for solver operations
- `nightly` - Forward nightly callable support from `ospf-rust-core`
- `big-rational` - Enable `BigRational` MetaModel value type support (forwarded to core)
- `big-decimal` - Enable `BigDecimal` MetaModel value type support (forwarded to core)

### SCIP Environment Notes

When enabling `scip`, besides `SCIPOPTDIR`, ensure `libclang` and SCIP runtime DLLs are discoverable:

- `LIBCLANG_PATH` should point to the directory containing `libclang.dll`
- `PATH` should include `<SCIPOPTDIR>\\bin` so test/runtime loading can find `libscip.dll`

### Solver Backend Notes (Gurobi / SCIP)

Framework extensions are in:

1. `src/solver/gurobi_extension.rs`
2. `src/solver/scip_extension.rs`

Core backend setup details are documented in core module:

1. Gurobi: `../ospf-rust-core/src/solver/solvers/gurobi/README.md`
2. SCIP: `../ospf-rust-core/src/solver/solvers/scip/README.md`

Feature examples for framework:

```bash
# Gurobi 12 extension path
cargo test -p ospf-rust-framework --features gurobi12

# SCIP extension path
cargo test -p ospf-rust-framework --features scip

# SCIP bundled + async extension path
cargo test -p ospf-rust-framework --features "scip-bundled async"
```

## Dependencies

- `ospf-rust-core` - Core data structures and traits
- `ospf-rust-base` - Base utilities
- `parking_lot` - High-performance synchronization primitives
- `log` - Logging facade
- `async-trait` (optional) - Async trait support

## License

This project is licensed under the same license as the main OSPF Rust project.
