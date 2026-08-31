# OSPF Rust Core

The core module of the OSPF (Operational Research Solver Framework) Rust implementation, providing fundamental data structures and abstractions for optimization modeling.

:us: English | :cn: [简体中文](README_ch.md)

## Overview

`ospf-rust-core` implements an operations research modeling framework supporting:

- Linear Programming (LP)
- Mixed Integer Programming (MIP)
- Quadratic Programming (QP)

## Core Modules

### Variable System (`variable`)
- Variable type definitions (binary, integer, continuous)
- Variable arena for efficient memory management
- Variable combinations and ranges

### Token System (`token`)
- Token representation for variables in expressions
- Token lists and tables for efficient lookup
- Variable data management

### Symbol System (`symbol`)
- Expression symbols (monomials, polynomials)
- Function symbols for custom expressions
- Intermediate symbol representations
- Monomial cell operations

### Model System (`model`)
- **Basic Model**: Core model structure
- **Configuration**: Model configuration options
- **Flatten System**: Expression flattening for solver input
- **Mechanism Model**: Constraint groups and mechanism-based modeling
- **Intermediate Model**: Linear and quadratic model representations
  - `LinearTriadModel` - Linear model (A, b, c)
  - `QuadraticTetradModel` - Quadratic model (Q, A, b, c)
- **Callback Model**: Multi-objective and callback-based modeling

### Solver Interface (`solver`)
- Solver configuration
- Solver output structures
- Heuristic algorithms
  - Population management
  - Selection, crossover, mutation operators
  - Normalization techniques
- IIS (Irreducible Inconsistent Subsystem) analysis

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
ospf-rust-core = { path = "path/to/ospf-rust-core" }
```

### Unified Solve API (Recommended)

For the most common path, call `MetaModel` directly:

```rust
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::{SolveOptions, SolverExt};

fn solve_model<S: ospf_rust_core::solver::Solver>(
    meta_model: &MetaModel<f64>,
    solver: &S,
) -> ospf_rust_core::error::Result<ospf_rust_core::solver::SolverOutput> {
    // shortest path
    let _output = meta_model.solve(solver)?;

    // with options
    let options = SolveOptions::new();
    solver.solve_with_options(meta_model, &options)
}
```

With `nightly` feature enabled, a callable wrapper is also available:

```rust
#[cfg(feature = "nightly")]
fn solve_with_callable<S: ospf_rust_core::solver::Solver>(
    meta_model: &MetaModel<f64>,
    solver: &S,
) -> ospf_rust_core::error::Result<ospf_rust_core::solver::SolverOutput> {
    let f = solver.as_fn();
    f(meta_model)
}
```

With `async` feature enabled, blocking solver calls can be moved to Tokio's blocking
thread pool:

```rust
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::{solve_async_with_callback, SolvingStatusCallback, Solver};

#[cfg(feature = "async")]
async fn solve_in_background<S: Solver + 'static>(
    meta_model: MetaModel<f64>,
    solver: Arc<S>,
    callback: SolvingStatusCallback,
) -> ospf_rust_core::error::Result<ospf_rust_core::solver::SolverOutput> {
    solve_async_with_callback(solver, meta_model, callback).await
}
```

### MetaModel Shortcut APIs

`MetaModel` now provides shortcut APIs for high-frequency modeling paths:

1. Add linear inequality with metadata (`group/lazy/priority/args`)
2. Batch-add symbolic constraints
3. Build partition constraints from coefficients or index list

```rust,ignore
use std::sync::Arc;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::{
    ConstraintGroup, ConstraintRelation, LinearInequality, MetaModel, SymbolicLinearInequality,
};

let mut model = MetaModel::<f64>::new("shortcut_demo");

// 1) Metadata shortcut
let g = Arc::new(ConstraintGroup::new(1001, "logic"));
let ineq = LinearInequality::new(
    Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0),
    ConstraintRelation::LessEqual,
    10.0,
);
model.add_inequality_with_metadata(
    ineq,
    "c_meta",
    Some(g),
    true,   // lazy
    20,     // priority
    Some("{\"tag\":\"demo\"}".to_string()),
)?;

// 2) Batch symbolic shortcuts
model.add_symbolic_inequalities(vec![
    (
        SymbolicLinearInequality::new(Linear::new(vec![LinearMonomial::new(1.0, 0)], 0.0), ConstraintRelation::GreaterEqual, 0.0),
        "sym_lb",
    ),
    (
        SymbolicLinearInequality::new(Linear::new(vec![LinearMonomial::new(1.0, 1)], 0.0), ConstraintRelation::LessEqual, 5.0),
        "sym_ub",
    ),
]);

// 3) Partition shortcuts
model.partition_linear_coefficients(&[(0, 1.0), (1, 1.0)], "p_coeff")?;
model.partition_linear_indices(&[2, 3, 4], "p_idx")?;
```

### Phase4 Builder APIs

`MetaModel` also provides Kotlin-aligned builder inputs to reduce manual sparse arrays:

```rust,ignore
use ospf_rust_core::model::{LinearExpressionBuilder, MetaModel, ObjectiveCategory};

let mut model = MetaModel::<f64>::new("builder_demo");

let c = LinearExpressionBuilder::new()
    .term(0, 1.0)
    .term(1, 2.0)
    .constant(-3.0)
    .le(0.0, "capacity");
model.add_linear_constraint_input(c)?;

let objective = LinearExpressionBuilder::new()
    .term(0, 4.0)
    .term(1, 5.0)
    .maximize("profit")
    .category(ObjectiveCategory::Maximum);
model.set_linear_objective_input(objective);
```

### Phase5 Gate

Default gate and scans:

```bash
bash scripts/phase5_gate.sh
powershell -File scripts/phase5_gate.ps1
```

### Feature Flags

- `async` - Enable async/await support
- `serde` - Enable serialization/deserialization
- `nightly` - Enable callable solver wrapper (`as_fn`)
- `gurobi` - Alias for `gurobi10`
- `gurobi10`, `gurobi11`, `gurobi12` - Gurobi version-specific bindings
- `scip` - Enable SCIP solver bindings
- `scip-bundled` - Enable bundled SCIP from `russcip`
- `scip-from-source` - Build SCIP from source through `russcip`
- `scip-quadratic` - Enable SCIP with quadratic support

### Solver Backends (Gurobi / SCIP)

Rust does not split solver backends into Maven-like modules. Enable each backend through Cargo
features on the crate that you depend on.

Backend-specific notes are documented here:

1. Gurobi: [solver/solvers/gurobi/README.md](src/solver/solvers/gurobi/README.md)
2. SCIP: [solver/solvers/scip/README.md](src/solver/solvers/scip/README.md)

Common feature examples:

```toml
[dependencies]
ospf-rust-core = { path = "../ospf-rust-core", features = ["gurobi10"] }
# or
ospf-rust-core = { path = "../ospf-rust-core", features = ["scip-bundled"] }
```

```bash
# Gurobi 10
cargo test -p ospf-rust-core --features gurobi10

# SCIP (system install)
cargo test -p ospf-rust-core --features scip

# SCIP bundled
cargo test -p ospf-rust-core --features scip-bundled
```

Recommended backend imports:

```rust,ignore
use ospf_rust_core::solver::backend::{GurobiSolver, ScipSolver};
```

`SCIPSolver` and `SCIPConfig` are kept as compatibility names. Prefer `ScipSolver` and
`ScipConfig` in new Rust code.

Backend configs expose concise aliases for common tuning knobs:

```rust,ignore
use ospf_rust_core::solver::backend::{GurobiConfig, GurobiSolver, ScipConfig, ScipSolver};

let gurobi = GurobiSolver::with_config(
    GurobiConfig::new()
        .with_gap(1e-4)
        .with_memory_limit_gb(8.0)
        .with_improve_threshold(1e-6),
);

let scip = ScipSolver::with_config(
    ScipConfig::new()
        .with_gap(1e-4)
        .with_memory_limit_mb(2048.0)
        .with_improve_threshold(1e-6),
);
```

Multi-solution calls use `solution_amount`. Gurobi and SCIP both try the native solution-pool path
when the backend supports it:

```rust,ignore
use ospf_rust_core::solver::{SolveOptions, SolverExt};

let options = SolveOptions::new().with_solution_amount(5);
let multi = solver.solve_multi_with_options(&meta_model, &options)?;
```

### Kotlin-Aligned Public Paths (Migration Note)

- `symbol::function` is the new preferred entry (legacy `symbol::functions` kept for compatibility).
- `symbol::flatten` is the new preferred entry (legacy `model::flatten` kept for compatibility).
- `solver::config`, `solver::output`, `solver::value`, `solver::backend` are introduced as aligned paths.

## Dependencies

- `ospf-rust-base` - Base utilities and collections
- `ospf-rust-math` - Mathematical types and operations
- `ospf-rust-multiarray` - Multi-dimensional array support
- `thiserror` - Error handling derive macros

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      User Code                          │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                   Model Layer                           │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────┐  │
│  │ Callback │ │Intermed. │ │Mechanism │ │   Flatten │  │
│  └──────────┘ └──────────┘ └──────────┘ └───────────┘  │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                   Symbol Layer                          │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐                │
│  │Expression│ │ Function │ │Intermed. │                │
│  │  Symbol  │ │  Symbol  │ │  Symbol  │                │
│  └──────────┘ └──────────┘ └──────────┘                │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                   Token Layer                           │
│  ┌──────────┐ ┌──────────┐                             │
│  │  Token   │ │TokenList │                             │
│  └──────────┘ └──────────┘                             │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                  Variable Layer                         │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐                │
│  │VariableId│ │ Variable │ │  Arena   │                │
│  └──────────┘ └──────────┘ └──────────┘                │
└─────────────────────────────────────────────────────────┘
```

## License

This project is licensed under the same license as the main OSPF Rust project.
