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


## Sub-package Overview

### solver/

| Sub-package | Description |
|-------------|-------------|
| `config` | Solver-specific configuration (COPT, Gurobi, SCIP). |
| `heuristic` | Metaheuristic framework (PSO, selection, crossover, mutation). |
| `iis` | Irreducible Infeasible Subsystem diagnostics. |
| `output` | Solver output data structures (feasible/infeasible). |
| `value` | Value type conversion (IntoValue trait). |
| `backend` | Feature-gated backend adapters (Gurobi, SCIP). |

### model/

| Sub-package | Description |
|-------------|-------------|
| `basic` | Foundation interfaces, enums, and view types. |
| `mechanism` | MetaModel, MechanismModel, constraint/objective DSL. |
| `intermediate` | Standard form models (triad/tetrad), sparse matrix. |
| `callback` | Callback model interface for heuristic solvers. |

### symbol/

| Sub-package | Description |
|-------------|-------------|
| `function` | 30+ function symbols (Slack, If, Max, Piecewise, etc.). |
| `flatten` | Expression flattening utilities. |

## Architecture Overview

`ospf-rust-core` follows the Kotlin-aligned model lifecycle:

```text
User definition layer  ->  MetaModel<V>
    -> mechanism layer  ->  MechanismModel<V>
    -> standard form    ->  LinearTriadModel / QuadraticTetradModel
    -> solver layer     ->  SolveReport (SolverOutput is compatibility-only)
```

The crate keeps model construction, expression flattening, solver-order token mapping, and result extraction explicit so higher-level framework crates can compose them without owning low-level modeling internals.


## Constraint Programming

The `model::constraint_programming` module provides integer-domain CP models with immutable snapshots, stable IDs, and source verification. Key components:

- **Boolean literals and variables**: `CpBoolVar`, `CpBoolLiteral` for Boolean decision variables.
- **Intervals**: `CpInterval` for modeling temporal intervals with start/end/duration.
- **Global constraints**: `AllDifferent`, `Element`, `Table` (support varies by backend).
- **Immutable snapshots**: `CpSnapshot` captures the complete CP model state for checkpoint/restart.
- **Identity scope**: Use `scope = "stable"` with a caller-owned `origin` when an ID must survive model rebuilds.
- **Portable codec**: `ConstraintProgrammingCheckpointCodec` writes portable checkpoint envelopes containing snapshot fingerprint, solver/configuration provenance, and validated incumbent.

### Solver Integration

The `solver::constraint_programming` module provides:

- **Solver/session SPI**: Trait definitions for CP solver backends.
- **Fake contract solver**: For testing and small exhaustive oracles.
- **SCIP integration**: Feature-gated SCIP CP backend (finite MIP-backed facade).
- **MIP-backed lowering**: Exact lowering for finite integer subsets with verified bounds; returns structured errors for unsupported constraints (`Cumulative`, `Circuit`, `Automaton`, `Reservoir`).

### Logic-Based Benders

For Logic-Based Benders decomposition, use `LogicBasedBendersEngine` from `ospf-rust-framework`. The engine combines a linear master with a CP subproblem, with explicit extension points for:

- Variable bindings and conflict/optimality cut oracles.
- Integer no-good encoding.
- Iteration traces and `Exact`/`Heuristic` proof gates.

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

The full report, proof, cancellation, identity, and legacy-migration contract is documented in
the [Unified Solve Contract](#unified-solve-contract) below.

Native feature and license evidence follows the
[Native Validation Matrix](#native-validation-matrix) below. Source
commit coverage is recorded in [Source Traceability](#source-traceability);
feature compilation alone is not native solver evidence.

For the most common path, call `MetaModel` directly:

```rust
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::{SolveOptions, SolveReport, SolverExt};

fn solve_model<S: ospf_rust_core::solver::Solver>(
    meta_model: &MetaModel<f64>,
    solver: &S,
) -> ospf_rust_core::error::Result<SolveReport<f64>> {
    let _report = meta_model.solve_report(solver)?;

    let options = SolveOptions::new();
    solver.solve_report_with_options(meta_model, &options)
}
```

Exact consumers must use the report certificate helpers. A limit or interruption
may retain an incumbent, but it cannot be treated as an optimality proof. Native
license failures are classified as `LICENSE`; missing libraries and environment
setup remain `ENVIRONMENT`.

### Constraint Programming Boundary

The core crate also provides an integer CP AST for Boolean literals, integer expressions,
immutable snapshots, canonical snapshot artifacts, and global constraints such as
`NoOverlap`. CP domains and objective evaluation use exact `i64` values; the CP path does
not silently narrow integer semantics to `f64`.

Gurobi and SCIP expose the CP facade through feature-gated, MIP-backed `ExactLowering`.
This is an exact formulation path with snapshot validation and unified `SolveReport` proof
checks, not a claim that either backend provides native CP search. Native optional-interval
trees, incremental CP sessions, and global constraints without an explicit lowering are
outside this facade and remain `Conditional` or `Unsupported` until their capability gate
is closed. Consumers should inspect the capability matrix before selecting a backend.

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

### Conditional Function Contract

Conditional functions use the following stable names: `IfFunction` is the legacy ternary
expression, `IfElseFunction` is the preferred explicit-binary ternary form,
`ConditionalIndicatorFunction` is the registerable relation indicator, and
`ConditionalIfFunction` is the non-registering classifier. `semantic::if_` and
`semantic::if_named` construct range-driven indicators; `if_legacy` preserves the old
threshold behavior. `IfInFunction` remains discrete-set membership, while
`IfInRangeFunction` and `RegisterableIfInRangeFunction` describe and register a closed
range. `ConditionalThenFunction` and `ConditionalImplyFunction` are the range-driven
registerable forms; `IfThenConstraintFunction` and `imply_constraint` remain legacy
Big-M compatibility entries. `SigmoidStepFunction` is the registerable relation-step
form and `SigmoidFunction` remains the continuous PWL form.

For `d = lhs - rhs`, relation indicators are defined only outside the undefined gap:

| Relation | True | False |
| --- | --- | --- |
| `Greater` | `d >= g` | `d <= 0` |
| `GreaterEqual` | `d >= 0` | `d <= -g` |
| `Less` | `d <= -g` | `d >= 0` |
| `LessEqual` | `d <= 0` | `d >= g` |

`g` is the positive business `strict_boundary`, not a solver tolerance. Registerable
indicators never infer Big-M: callers provide finite ordered bounds covering the
condition polynomial, and ranges wholly inside the undefined gap are rejected. A
`None` result from `evaluate` may mean either undefined input or unavailable input;
use `classify` when that distinction matters. Non-constant conditional branches also
require explicit finite bounds.

Discrete conditions derive and validate their lattice proof from linear coefficients
and registered token metadata: participating variables are integer, coefficients are
finite integers, `delta` is their gcd, and the constant is normalized modulo `delta`.
`MetaModel::add_symbols` and `register_combination` are atomic transactions; failed
registration restores tokens, symbols, constraints, and cache bindings. Third-party
`MutableTokenList`/`MutableTokenTable` implementations must provide an explicit atomic
batch implementation and should use `try_add_tokens` to observe validation failures.
Concurrent token collections expose borrowed trait views through a held `read()` guard.
Legacy Big-M helpers reject non-finite, zero, and negative values. `IfInRangeFunction`
accepts one shared variable with finite, ordered side bounds.

## Unified Solve Contract

New solver-facing code consumes `solver::SolveReport<V>`. `SolverOutput`,
`FeasibleSolution`, `SolveResult`, and `SerializedSolution` are compatibility projections
and must not be used to infer proof or cancellation semantics. A report keeps
`problem_status`, `termination_reason`, validated incumbent, proof, statistics,
diagnostics, provenance, fingerprints, and trace separate. A feasible report requires
an incumbent; infeasible/unbounded reports cannot carry one; an optimality proof requires
completed termination and a reliable complete certificate. An incumbent at a limit or
interruption is a candidate only and never closes an exact bound.

`SolverErrorClass` is the stable error boundary: `INPUT`, `MODELING`, `ENVIRONMENT`,
`LICENSE`, `CALLBACK`, `BACKEND`, `PARSING`, `NUMERICAL`, `INTERNAL_CONTRACT`,
`TERMINAL_PROJECTION`, and `UNSUPPORTED`. Cancellation is a normal report terminal;
legacy methods may project it to `SolverError::Cancelled`. Each solve owns an idempotent
`SolveHandle`; async wrappers use Tokio's blocking pool, and `cancel_and_wait` is the
resource-release boundary. Reports carry deterministic model, configuration, and
solver-environment fingerprints. Callback registration is provenance and is marked
non-replayable. Remote reports/checkpoints preserve schema, run/attempt identity,
parent links, artifact digest, provenance, fingerprints, proof, incumbent, and the
cancellation chain; unknown schemas or mismatches are rejected before resume.

The exact native backend scope is Gurobi and SCIP. Feature compilation proves wiring
only; native capability requires a matching library, runtime, and license probe. Gurobi
license failures (including code `10009`) are `LICENSE`; missing libraries are
`ENVIRONMENT`.

### Native Validation Matrix

`cargo check` is compile evidence only. Native tests that cannot load their backend are
`unsupported` (or `failed` when requested), and ignored tests require
`-- --include-ignored`. Golden/replay comparisons include status, termination,
incumbent objective, best bound, gap, solution values, residuals, fingerprints, and
provenance. The executable gates are:

| Scope | Command shape |
| --- | --- |
| Core contract | `cargo test -p ospf-rust-core --test native_contract_suite` |
| Gurobi | same target with `--features gurobi10`/`gurobi11`/`gurobi12` |
| SCIP | same target with `--features scip` (or explicitly bundled/from-source) |
| Release terminals | `native_release_matrix` with a native backend and `-- --include-ignored` |
| Framework reports | `cargo test -p ospf-rust-framework --no-default-features` and `--features async`/`remote-solver` |
| Network and Demo5 | crate README validation commands plus the selected native feature |

### CP Capability Boundary

The CP AST uses exact `i64` values and immutable snapshots. Gurobi and SCIP expose an
exact finite MIP-backed `ExactLowering` facade, not native CP search. Safe linear
integer/binary variables, positive-literal indicators, SOS1, status metadata,
cancellation, event handlers, and scoped probing are native only when their explicit
probe succeeds. Reified Boolean forms, sparse domains, AllDifferent, Element, tables,
and finite NoOverlap are `ExactLowering`; Circuit, Automaton, Reservoir, incremental
sessions, optional/variable-duration native intervals, and native CP conflict graphs
are `Unsupported`. Raw Cumulative probing is `Conditional` and is not production
capability evidence. Missing libraries, licenses, bundled downloads, or source builds
remain `not executed/unsupported`, never a compile-only pass.

### Terminal-Loss Inventory

The report layer replaces the former loss of terminal information at every boundary:

| Boundary | Current owner |
| --- | --- |
| Core status/value projection | `ProblemStatus`, `TerminationReason`, `SolveReport`, validated builders, and explicit legacy projections |
| Column generation/Benders | report aggregation and optimal-LP/dual/Farkas certificate gates |
| Branch-and-Price | node conclusion, pricing-complete flag, inherited/certified bound, and certificate gates |
| Combinatorial wrappers | parent/child attempt identity, completion linearization, loser traces, and cancellation snapshots |
| Remote/checkpoint | versioned DTOs, artifact/fingerprint validation, provenance, parent attempt, and cancellation origin |

### Source Traceability

The unified migration tracks these eleven immutable Kotlin source commits and their Rust
owners: `b8d67c96` (report/progress), `5f616747` (termination propagation), `25bcb176`
(Benders/branch-and-price proof gates), `32f7d5aa` (LP infeasibility), `e0bca1eb`
(identity/fingerprint/remote/checkpoint), `4efac629` (capability/provenance),
`58930764` (aggregate provenance), `e5089f18` (aggregate identity), `b9db32a8`
(in-flight cancellation), `ae0b01fb` (completion freeze), and `b5b83d7d`
(parallel cancellation tests). Core report, proof, diagnostics, identity, fingerprint,
progress, cancellation, and checkpoint are implemented here; Gurobi/SCIP are feature
gated; framework combinatorial/Benders/remote paths are implemented in
`ospf-rust-framework`; CPLEX, COPT, Hexaly, MindOPT, MOSEK, and native CP search are
explicitly excluded. Full manifests remain reproducible with `git show --no-renames`
against the immutable hashes; generated command output is not versioned. The complete
hashes, in order, are `b8d67c96be2d29e6477838adbbb3ee6fec27ddf5`,
`5f61674788ae9543c4769b1eacbc74d24296012b`,
`25bcb176ebe3c4380f84eac6c3777f930f9ba47e`,
`32f7d5aa76fb9f7f5982d856497b480bbf7f3b3f`,
`e0bca1eb4d04e99fa8048deb9b2bb1731a1ac6b4`,
`4efac629a57571497695352cf8448980be4e418b`,
`589307646757d7f43afda299b866b2cfcf874ac2`,
`e5089f1886b0fb924b8f721966cf1bb511be7395`,
`b9db32a86af51e8ea976b81c2c15cbc3126dd006`,
`ae0b01fbb516a4fbd834adda5643b9a16d4b8041`, and
`b5b83d7d6f470c363e1044cd6b0266604ad5aaa1`.

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
