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
```

Backend build-only verification:

```powershell
cargo test -p ospf-rust-example --features backend-gurobi --no-run
```

When backend feature is not enabled, running backend demos returns the hint `rerun with --features backend-gurobi`.

`framework:demo4` currently stays as a scaffold command entry. Gantt-scheduling parity is intentionally postponed and not marked as completed functionality.

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
