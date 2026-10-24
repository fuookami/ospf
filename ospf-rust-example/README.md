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
| [`src/framework/demo2`](src/framework/demo2/README.md) | Complete shared aviation registration: FullLoad, Predistribution and WeightRecommendation, with MILP, safe Benders and independent partition CG. |
| `src/framework/demo3` | CSP1D-facing demo scaffold. |
| `src/framework/demo4` | Lossless flight recovery, managed Gantt master, Network path pricing and actual result extraction. |
| `src/framework/demo5` | Solomon parser/adapter and VRPTW Branch-and-Price solver wiring. |

## Public API

The crate is primarily executable. The stable surface is its command names and documented behavior contracts:

| Command | Responsibility | Backend |
| --- | --- | --- |
| `core:demo1` | Core modeling demo. | Gurobi feature |
| `core:all` | Run core demo set. | Gurobi feature |
| `core:capability-gate` | Capability gating demo. | Gurobi feature |
| `core:shortcuts` | `MetaModel` shortcut API demo. | Gurobi feature |
| `framework:demo1` | Framework routing/bandwidth context demo. | Gurobi feature |
| `framework:demo2` | Shared BPP1D/BPP2D aviation loading and actual solver-evidence demo. | Gurobi feature |
| `framework:demo3` | CSP1D scaffold entry. | Gurobi feature |
| `framework:demo4` | Flight-recovery domain showcase or actual Gantt column-generation application. | `demo4-offline` or `demo4-gurobi-cg` |
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
cargo run -p ospf-rust-example --features backend-gurobi -- core:capability-gate
cargo run -p ospf-rust-example --features backend-gurobi -- core:shortcuts
cargo run -p ospf-rust-example --features backend-gurobi -- framework:demo1
cargo run -p ospf-rust-example --features backend-gurobi -- framework:demo2
cargo run -p ospf-rust-example --features backend-gurobi -- framework:demo3
cargo +nightly-2026-08-01 run -p ospf-rust-example --no-default-features --features demo4-gurobi-cg -- framework:demo4
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

`framework:demo4` runs the recovery domain showcase under `demo4-offline` and additionally executes
the actual Gantt application under `demo4-gurobi-cg`: automatic seeds, RMP, complete current-model
duals, Network path pricing, column insertion, final MILP and production validation. It is not a
private simplified CG loop and does not derive global integer optimality from finite-master reports.
The [paired Demo4 documentation](src/framework/demo4/README.md) describes actual records, links/
passengers, fleet, restrictions, flow controls, aircraft/time locks and snapshot boundaries.
Offline tests require no commercial solver; native gates must actually execute:

```powershell
cargo +nightly-2026-08-01 test -p ospf-rust-example --no-default-features --features demo4-offline -- --list
cargo +nightly-2026-08-01 test -p ospf-rust-example --no-default-features --features demo4-offline
cargo +nightly-2026-08-01 test -p ospf-rust-example --no-default-features --features demo4-gurobi-cg -- --list
cargo +nightly-2026-08-01 test -p ospf-rust-example --no-default-features --features demo4-gurobi-cg -- --include-ignored
```

`framework:demo5` uses an inline Solomon fixture and exercises parser, adapter, route generation,
restricted master and Branch-and-Price wiring. The corresponding integration targets also execute
native solves, route validation and the small full-route oracle beyond fixture/parser checks.
Library gates separately cover the 25-customer direct-MIP cross-check, 100-customer smoke, strict-proof
fixture and SCIP legal terminal states. Both levels must run; neither replaces the other:

```powershell
cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib demo17_25_branch_and_price_matches_direct_mip_objective -- --include-ignored
cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib demo17_100_branch_and_price_smoke_respects_limits -- --include-ignored
cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib proof_100_customer_fixture_closes_direct_mip_and_branch_and_price_bounds -- --include-ignored
cargo test -p ospf-rust-example --features demo5-scip-bp --lib demo17_25_scip_branch_and_price_returns_legal_terminal -- --include-ignored
```

Solver and pricing failure paths are covered by the network crate's offline tests. Missing native runtime or license is a failed gate, not a pass.

## Demo2 Benders Behavior Contract

`framework::demo2` uses the shared fixed-recourse Benders service with optional fresh-model MILP fallback.

When `prefer_benders=true`, the application selects Benders using the actual sparse assignment count and emits configured and effective settings. Legacy cut/objective stall controls are explicitly inactive in the safe fixed-recourse service; they do not silently alter stopping or fallback.

### Status and Path Semantics

1. `prefer_benders=false`: solve directly with MILP unless independent partition CG is explicitly selected; diagnostics identify the actual path.
2. Benders/CG accelerator incumbents stay `Feasible`; master/sub and restricted reports are not promoted. `solve_policy.source_milp_finishing` defaults to `IfUnproven`: without a complete-source terminal proof, a fresh equivalent source MILP finishes with reserved time and a validated warm start. `Disabled` appends no source solve and reserves no finishing time, while retaining full-source feasibility validation.
3. Recoverable decomposition failures may use `benders_fallback_to_milp=true` to launch a fresh equivalent MILP with only the remaining total time. Input/modeling/callback errors, cancellation and exhausted time do not trigger fallback.
4. The legacy fallback switch independently controls early fallback inside acceleration; disable it and set finishing to `Disabled` to prohibit all ordinary MILP solves after Benders. Cancellation/interruption and an exhausted total deadline never start another solve. Actual accelerator and certification evidence stay separate, without synthetic `BendersFailed` states.

`milp_fallback` records that a fresh MILP was attempted; it does not itself certify success. An empty or worse fallback result cannot discard a better validated decomposition incumbent. See the [complete Demo2 contract](src/framework/demo2/README.md) for terminal states, independent CG, callbacks and extraction.

### Quality Override

`Demo2Request.benders_quality_overrides` overrides observational diagnostic thresholds. These diagnostics use actual available bounds and cut counts; they neither fabricate trajectories nor trigger fallback or discard an incumbent. If omitted, default diagnostic values are applied.

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
