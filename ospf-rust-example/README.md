# OSPF Rust Example

[中文](README_ch.md) | English

`ospf-rust-example` contains runnable and testable examples built on top of `ospf-rust-core` and `ospf-rust-framework`.

## Demo2 Benders Behavior Contract

`framework::demo2` supports adaptive Benders with optional MILP fallback.

When `prefer_benders=true`, the application derives an effective adaptive profile from problem size
(`cargo_count * position_count`) and emits both configured and effective settings in notes.
The effective profile includes `max_stall_iterations` (cut-stagnation window).
It also includes `objective_stall_iterations` (objective-improvement stall window).

### Status and Path Semantics

1. `prefer_benders=false`
- Solve directly with MILP.
- `notes/diagnostics` include `solver_path=milp_direct`.

2. `prefer_benders=true` and Benders succeeds
- Response status is solver feasible/optimal result (`Feasible` or `Optimal`).
- `notes/diagnostics` include `solver_path=benders`.
- Runtime metrics are emitted as both plain notes and structured diagnostics:
  - `benders_iterations`
  - `benders_gap`
  - `benders_time_ms`
  - `benders_adaptive_effective`
  - `benders_problem_size_binary_variables`

3. `prefer_benders=true`, Benders fails, `benders_fallback_to_milp=true`
- Application retries with MILP.
- Final path is `solver_path=milp_fallback`.
- Failure reason remains observable via `benders_failed`.

4. `prefer_benders=true`, Benders fails, `benders_fallback_to_milp=false`
- Response status is `BendersFailed`.
- No MILP retry is performed.
- `benders_failed` is emitted for diagnostics.

`BendersFailed` and `milp_fallback` are intentionally different:
- `BendersFailed` means the request terminates on the Benders error path.
- `milp_fallback` means Benders failed first, but the request recovered through MILP.

### Quality Override (Request-Level)

`Demo2Request.benders_quality_overrides` can override online quality-guard thresholds per request.
If omitted, default guard values are applied.

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

Example:

```rust
let mut request = Demo2Request::sample();
request.solve_policy.prefer_benders = true;
request.benders_quality_overrides = Some(BendersQualityOverrideConfig {
    weak_gap_multiplier: Some(30.0),
    weak_gap_floor: Some(2e-5),
    iteration_pressure_percent: Some(80),
    cut_density_min_iterations: Some(10),
    cut_density_threshold: Some(0.4),
    trajectory_min_snapshots: Some(8),
    trajectory_step_multiplier: Some(25.0),
    trajectory_step_floor: Some(2e-6),
    time_guard_min_ms: Some(1200),
    score_gap_weight: Some(0.4),
    score_time_weight: Some(0.3),
    score_iteration_weight: Some(0.1),
    score_cut_density_weight: Some(0.1),
    score_trajectory_weight: Some(0.1),
});
```

When Benders is selected, response notes/diagnostics include `benders_quality_guard_effective`
with the final effective values.

## API Pointers

- Request/response DTO: `src/framework/demo2/infrastructure/dto.rs`
- Demo2 application orchestration: `src/framework/demo2/domain.rs`
- Diagnostics parser and grouped-note contract: `src/framework/demo2/diagnostics.rs`
