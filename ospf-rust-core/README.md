# OSPF Rust Core

[中文](README_ch.md) | English

The core module of the OSPF (Operational Research Solver Framework) Rust implementation, providing fundamental data structures and abstractions for optimization modeling.

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

### MetaModel Shortcut APIs

`MetaModel` now provides shortcut APIs for high-frequency modeling paths:

1. Add linear inequality with metadata (`group/lazy/priority/args`)
2. Batch-add symbolic constraints
3. Build partition constraints from coefficients or index list

```rust,ignore
use std::sync::Arc;
use ospf_rust_core::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::{
    ConstraintGroup, ConstraintRelation, LinearInequality, MetaModel, SymbolicLinearInequality,
};

let mut model = MetaModel::<f64>::new("shortcut_demo");

// 1) Metadata shortcut
let g = Arc::new(ConstraintGroup::new("logic"));
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

### Feature Flags

- `async` - Enable async/await support
- `serde` - Enable serialization/deserialization
- `nightly` - Enable callable solver wrapper (`as_fn`)
- `gurobi` - Enable Gurobi solver bindings
- `gurobi10`, `gurobi11`, `gurobi12` - Gurobi version-specific bindings
- `scip` - Enable SCIP solver bindings
- `scip-quadratic` - Enable SCIP with quadratic support

### Solver Backends (Gurobi / SCIP)

Backend-specific notes are documented here:

1. Gurobi: [solver/solvers/gurobi/README.md](src/solver/solvers/gurobi/README.md)
2. SCIP: [solver/solvers/scip/README.md](src/solver/solvers/scip/README.md)

Common feature examples:

```bash
# Gurobi 12
cargo test -p ospf-rust-core --features gurobi12

# SCIP (system install)
cargo test -p ospf-rust-core --features scip

# SCIP bundled
cargo test -p ospf-rust-core --features scip-bundled
```

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
