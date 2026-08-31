# OSPF Rust Example

:us: English | :cn: [简体中文](README_ch.md)

## Introduction

`ospf-rust-example` contains runnable and testable examples built on top of `ospf-rust-core`, `ospf-rust-framework`, and domain framework crates. It maps Kotlin example/demo responsibilities into Rust command entries and migration compatibility examples.

## Scope

This crate owns examples, command dispatch, migration compatibility demos, request/response DTO examples, and behavior contracts used to validate framework usage.

Explicit non-goals:

1. Shared modeling abstractions that belong in `ospf-rust-core` or `ospf-rust-framework`.
2. Reusable domain framework logic that belongs in domain crates such as CSP1D, BPP3D, or Gantt Scheduling.
3. Production request protocols or deployment adapters.

## Module Structure

| Rust path | Responsibility |
| --- | --- |
| `src/main.rs` | Command dispatch entry. |
| `src/example_modeling.rs` | Shared modeling and typed solve helpers. |
| `src/core` | Core modeling demos, shortcut APIs, capability gates, and backend-aware examples. |
| `src/framework/demo1` | Framework routing/bandwidth context example. |
| `src/framework/demo2` | Adaptive Benders / MILP fallback behavior contract example. |
| `src/framework/demo3` | CSP1D-facing demo scaffold. |
| `src/framework/demo4` | Gantt-scheduling-facing scaffold and domain layout experiment. |
| `src/framework/demo5` | Solomon parser/adapter and VRPTW Branch-and-Price solver wiring. |

## Public API

The crate is primarily executable. The stable surface is its command names and documented behavior contracts:

| Command | Responsibility | Backend |
| --- | --- | --- |
| `core:demo1` | Core modeling demo. | Gurobi feature |
| `core:all` | Run core demo set. | Gurobi feature |
| `core:generic-number` | Generic-number modeling path. | Gurobi feature |
| `core:shortcuts` | `MetaModel` shortcut API demo. | Gurobi feature |
| `framework:demo1` | Framework routing/bandwidth context demo. | Gurobi feature |
| `framework:demo2` | Adaptive Benders and MILP fallback demo. | Gurobi feature |
| `framework:demo3` | CSP1D scaffold entry. | Gurobi feature |
| `framework:demo4` | Gantt scaffold entry. | Gurobi feature |
| `framework:demo5` | VRPTW Branch-and-Price demo. | `demo5-gurobi-bp` or `demo5-scip-bp` |

## Commands

Default build/test path without commercial backend:

```powershell
cargo check -p ospf-rust-example
cargo test -p ospf-rust-example --no-run
cargo test -p ospf-rust-example
```

Runtime demos requiring backend:

```powershell
cargo run -p ospf-rust-example --features backend-gurobi -- core:demo1
cargo run -p ospf-rust-example --features backend-gurobi -- core:all
cargo run -p ospf-rust-example --features backend-gurobi -- core:generic-number
cargo run -p ospf-rust-example --features backend-gurobi -- core:shortcuts
cargo run -p ospf-rust-example --features backend-gurobi -- framework:demo1
cargo run -p ospf-rust-example --features backend-gurobi -- framework:demo2
cargo run -p ospf-rust-example --features backend-gurobi -- framework:demo3
cargo run -p ospf-rust-example --features backend-gurobi -- framework:demo4
cargo run -p ospf-rust-example --features demo5-gurobi-bp -- framework:demo5
cargo run -p ospf-rust-example --features demo5-scip-bp -- framework:demo5
```

Backend build-only verification:

```powershell
cargo test -p ospf-rust-example --features backend-gurobi --no-run
cargo check -p ospf-rust-example --features demo5-gurobi-bp
cargo check -p ospf-rust-example --features demo5-scip-bp
```

When backend feature is not enabled, running backend demos returns the hint `rerun with --features backend-gurobi`.

`framework:demo4` currently stays as a scaffold command entry. Gantt-scheduling parity is intentionally postponed and not marked as completed functionality.

`framework:demo5` uses an inline Solomon fixture and exercises parser, adapter, route generation, restricted master, and Branch-and-Price wiring. The integration targets cover target-level fixtures/parser checks; the library gates below are the commands that execute the direct-MIP cross-check, five-customer full-route master oracle, 100-customer smoke, and strict-proof fixture:

```powershell
cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib demo17_25_branch_and_price_matches_direct_mip_objective -- --include-ignored
cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib demo17_100_branch_and_price_smoke_respects_limits -- --include-ignored
cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib proof_100_customer_fixture_closes_direct_mip_and_branch_and_price_bounds -- --include-ignored
cargo test -p ospf-rust-example --features demo5-scip-bp --lib demo17_25_scip_branch_and_price_returns_legal_terminal -- --include-ignored
```

Solver and pricing failure paths are covered by the network crate's offline tests. Missing native runtime or license is a failed gate, not a pass.

## Demo2 Benders Behavior Contract

`framework::demo2` supports adaptive Benders with optional MILP fallback.

When `prefer_benders=true`, the application derives an effective adaptive profile from problem size (`cargo_count * position_count`) and emits both configured and effective settings in notes. The effective profile includes `max_stall_iterations` for cut stagnation and `objective_stall_iterations` for objective-improvement stagnation.

### Status and Path Semantics

1. `prefer_benders=false`: solve directly with MILP; diagnostics include `solver_path=milp_direct`.
2. `prefer_benders=true` and Benders succeeds: response status is `Feasible` or `Optimal`; diagnostics include `solver_path=benders`.
3. `prefer_benders=true`, Benders fails, and `benders_fallback_to_milp=true`: application retries with MILP; final path is `solver_path=milp_fallback`; `benders_failed` remains observable.
4. `prefer_benders=true`, Benders fails, and `benders_fallback_to_milp=false`: response status is `BendersFailed`; no MILP retry is performed.

`BendersFailed` means the request terminates on the Benders error path. `milp_fallback` means Benders failed first, but the request recovered through MILP.

### Quality Override

`Demo2Request.benders_quality_overrides` can override online quality-guard thresholds per request. If omitted, default guard values are applied.

Supported fields:

- `weak_gap_multiplier`
- `weak_gap_floor`
- `iteration_pressure_percent`
- `cut_density_min_iterations`
- `cut_density_threshold`
- `trajectory_min_snapshots`
- `trajectory_step_multiplier`
- `trajectory_step_floor`
- `time_guard_min_ms`
- `score_gap_weight`
- `score_time_weight`
- `score_iteration_weight`
- `score_cut_density_weight`
- `score_trajectory_weight`

When Benders is selected, response notes/diagnostics include `benders_quality_guard_effective` with final effective values.

## Local Validation

```powershell
cargo check -p ospf-rust-example
cargo test -p ospf-rust-example --no-run
cargo test -p ospf-rust-example
cargo test -p ospf-rust-example --features backend-gurobi --no-run
```

## Current Boundaries

This crate is intentionally allowed to contain migration compatibility layouts and scaffold command entries. Reusable modeling patterns discovered here should be promoted into the appropriate core/framework/domain crate before downstream code depends on them.

## Related Modules

- [Root README](../README.md)
- [Core README](../ospf-rust-core/README.md)
- [Framework README](../ospf-rust-framework/README.md)
- [Kotlin example README](../../ospf-kotlin/ospf-kotlin-example/README.md)
