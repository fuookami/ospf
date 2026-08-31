# OSPF Rust Core

:us: English | :cn: [简体中文](README_ch.md)

## Introduction

`ospf-rust-core` is the core modeling crate of the OSPF Rust workspace. It owns the optimization model lifecycle from variables and symbolic expressions, through `MetaModel` construction and flattening, to solver abstraction, result output, IIS diagnostics, and feature-gated backend adapters.

## Scope

This crate covers:

1. Variable and token systems.
2. Symbolic expression and function-symbol systems.
3. `MetaModel`, mechanism model, intermediate standard-form models, and callback model layers.
4. Solver traits, options, outputs, value conversion, heuristic interfaces, IIS diagnostics, and Gurobi/SCIP adapter boundaries.

Explicit non-goals:

1. Framework-level column-generation orchestration, Benders combinators, persistence, and remote solving; those belong in `ospf-rust-framework`.
2. Domain-specific modeling such as cutting stock, packing, or scheduling.
3. Solver installation and license management beyond backend setup notes.

## Module Structure

| Rust module | Kotlin boundary | Responsibility |
| --- | --- | --- |
| `variable` | `core/variable` | Variable type system, variable items, combinations, and ranges. |
| `token` | `core/token` | Variable-token mapping, token lists/tables, cached values, and solver result access. |
| `symbol` | `core/symbol` | Expression symbols, monomial cells, function symbols, intermediate symbols, and flattening helpers. |
| `model` | `core/model` | `MetaModel`, mechanism model, intermediate triad/tetrad models, callback model, constraint/objective DSL, and model state. |
| `solver` | `core/solver` | Solver traits, options, backend configs, output types, value conversion, heuristic helpers, IIS diagnostics, and backend adapters. |
| `error` | `core/error` | Core error and result types. |

## Architecture Overview

`ospf-rust-core` follows the Kotlin-aligned model lifecycle:

```text
User definition layer  ->  MetaModel<V>
    -> mechanism layer  ->  MechanismModel<V>
    -> standard form    ->  LinearTriadModel / QuadraticTetradModel
    -> solver layer     ->  SolverOutput
```

The crate keeps model construction, expression flattening, solver-order token mapping, and result extraction explicit so higher-level framework crates can compose them without owning low-level modeling internals.

## Core Concepts

1. Variables describe solver decision domains such as binary, integer, and continuous values.
2. Tokens connect variables and symbols to solver-order indices and cached results.
3. Symbols model linear/quadratic expressions, function symbols, and intermediate values.
4. `MetaModel` is the user-facing assembly layer; mechanism and intermediate models are generated for solver consumption.
5. Solver traits and backend adapters convert standard-form models into backend calls and return structured output.

## Public API

| API | Responsibility | Stability |
| --- | --- | --- |
| `MetaModel<V>` | Primary user-facing model assembly object. | stable within migration |
| `Variable`, `VariableRange`, `VariableType` | Decision variable definitions and ranges. | stable within migration |
| `Token`, `TokenList`, `TokenTable` | Solver-order token mapping and result/cache access. | stable within migration |
| `symbol::function` | Preferred function-symbol path. | stable within migration |
| `symbol::flatten` | Preferred expression-flattening path. | stable within migration |
| `model::mechanism` | Mechanism model and constraint/objective lowering. | migration |
| `LinearTriadModel`, `QuadraticTetradModel` | Standard-form solver input models. | migration |
| `Solver`, `SolverExt`, `SolveOptions` | Unified solver trait and solve options. | migration |
| `solver::backend::{GurobiSolver, ScipSolver}` | Feature-gated backend adapters. | migration |

## Generic Numeric Boundaries

`MetaModel<V>` is generic over the modeling value type. Backend adapters currently solve through backend numeric domains, most commonly `f64`. Conversions should remain explicit at solver, flattening, and extraction boundaries. The `big-rational` and `big-decimal` feature paths are coordinated through core/framework conversion policy support.

## Solve Lifecycle

The common solve path is:

1. Build a `MetaModel<V>` with variables, symbols, constraints, and objectives.
2. Lower symbolic expressions through mechanism and flattening layers.
3. Dump to linear or quadratic standard form.
4. Invoke a `Solver` implementation.
5. Write solver-order values back to tokens and expose structured output.

## Usage

Add the crate:

```toml
[dependencies]
ospf-rust-core = { path = "path/to/ospf-rust-core" }
```

### Unified Solve API

For the most common path, call `MetaModel` directly:

```rust
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::{SolveOptions, SolverExt};

fn solve_model<S: ospf_rust_core::solver::Solver>(
    meta_model: &MetaModel<f64>,
    solver: &S,
) -> ospf_rust_core::error::Result<ospf_rust_core::solver::SolverOutput> {
    let _output = meta_model.solve(solver)?;

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

With `async` feature enabled, blocking solver calls can be moved to Tokio's blocking thread pool:

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

`MetaModel` provides shortcut APIs for high-frequency modeling paths:

1. Add linear inequality with metadata (`group/lazy/priority/args`).
2. Batch-add symbolic constraints.
3. Build partition constraints from coefficients or index lists.

```rust,ignore
use std::sync::Arc;
use ospf_rust_core::symbol::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::{
    ConstraintGroup, ConstraintRelation, LinearInequality, MetaModel, SymbolicLinearInequality,
};

let mut model = MetaModel::<f64>::new("shortcut_demo");

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
    true,
    20,
    Some("{\"tag\":\"demo\"}".to_string()),
)?;

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

model.partition_linear_coefficients(&[(0, 1.0), (1, 1.0)], "p_coeff")?;
model.partition_linear_indices(&[2, 3, 4], "p_idx")?;
```

### Builder Inputs

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

## Feature Flags

- `async`: enable async support.
- `serde`: enable serialization/deserialization.
- `nightly`: enable callable solver wrapper (`as_fn`).
- `gurobi`: alias for `gurobi10`.
- `gurobi10`, `gurobi11`, `gurobi12`: Gurobi version-specific bindings.
- `scip`: enable SCIP solver bindings.
- `scip-bundled`: enable bundled SCIP from `russcip`.
- `scip-from-source`: build SCIP from source through `russcip`.
- `scip-quadratic`: enable SCIP quadratic support.

## Solver Backends

Rust does not split solver backends into Maven-style modules. Enable each backend through Cargo features on the crate that you depend on.

Backend-specific notes:

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
cargo test -p ospf-rust-core --features gurobi10
cargo test -p ospf-rust-core --features scip
cargo test -p ospf-rust-core --features scip-bundled
```

Recommended backend imports:

```rust,ignore
use ospf_rust_core::solver::backend::{GurobiSolver, ScipSolver};
```

`SCIPSolver` and `SCIPConfig` are kept as compatibility names. Prefer `ScipSolver` and `ScipConfig` in new Rust code.

## Kotlin-Aligned Public Paths

- `symbol::function` is the new preferred entry; legacy `symbol::functions` is kept for compatibility.
- `symbol::flatten` is the new preferred entry; legacy `model::flatten` is kept for compatibility.
- `solver::config`, `solver::output`, `solver::value`, and `solver::backend` are introduced as aligned paths.

## Local Validation

```powershell
cargo check -p ospf-rust-core
cargo test -p ospf-rust-core --no-run
cargo test -p ospf-rust-core --lib
bash scripts/phase5_gate.sh
powershell -File scripts/phase5_gate.ps1
```

## Dependencies

- `ospf-rust-base`
- `ospf-rust-math`
- `ospf-rust-multiarray`
- `thiserror`

## Related Modules

- [Root README](../README.md)
- [Framework README](../ospf-rust-framework/README.md)
- [Kotlin core README](../../ospf-kotlin/ospf-kotlin-core/README.md)

## License

This project is licensed under the same license as the main OSPF Rust project.
