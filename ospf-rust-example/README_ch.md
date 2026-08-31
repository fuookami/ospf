# OSPF Rust Example

中文 | [English](README.md)

`ospf-rust-example` 提供基于 `ospf-rust-core` 与 `ospf-rust-framework` 的可运行、可测试示例。

## Kotlin 对齐目录

- `src/example_modeling.rs`：统一建模与 typed 求解 helper
- `src/core_demo`：core 示例主入口
- `src/heuristic_demo`：启发式示例入口（当前骨架阶段）
- `src/framework_demo`：framework 示例主入口
- `src/core` 与 `src/framework`：迁移期兼容实现模块

## 命令说明

默认构建/测试路径（不依赖商业后端）：

```bash
cargo check -p ospf-rust-example
cargo test -p ospf-rust-example --no-run
cargo test -p ospf-rust-example
```

需要后端的运行命令：

```bash
cargo run -p ospf-rust-example --features backend-gurobi -- core:demo1
cargo run -p ospf-rust-example --features backend-gurobi -- core:all
cargo run -p ospf-rust-example --features backend-gurobi -- core:generic-number
cargo run -p ospf-rust-example --features backend-gurobi -- core:shortcuts
cargo run -p ospf-rust-example --features backend-gurobi -- framework:demo1
cargo run -p ospf-rust-example --features backend-gurobi -- framework:demo2
cargo run -p ospf-rust-example --features backend-gurobi -- framework:demo3
cargo run -p ospf-rust-example --features backend-gurobi -- framework:demo4
```

后端 build-only 验证命令：

```bash
cargo test -p ospf-rust-example --features backend-gurobi --no-run
```

未启用后端 feature 时，直接运行 demo 命令会得到明确提示：
`rerun with --features backend-gurobi`。

`framework:demo4` 当前仅保持骨架与命令入口。由于 gantt-scheduling 对齐缺口仍在，
该方向明确标注为“暂缓实现”，不作为已完成功能宣称。

## Demo2 的 Benders 行为契约

`framework_demo::demo2` 支持自适应 Benders，并可按策略回退 MILP。

当 `prefer_benders=true` 时，应用层会基于问题规模（`cargo_count * position_count`）
推导生效参数，并在 notes 中同时输出“配置参数”和“生效参数”。
生效参数中包含 `max_stall_iterations`（连续无新 cut 的停滞窗口）。
同时包含 `objective_stall_iterations`（目标改进停滞窗口）。

### 状态与路径语义

1. `prefer_benders=false`
- 直接走 MILP 求解。
- `notes/diagnostics` 包含 `solver_path=milp_direct`。

2. `prefer_benders=true` 且 Benders 成功
- 响应状态为求解器返回的可行/最优（`Feasible` 或 `Optimal`）。
- `notes/diagnostics` 包含 `solver_path=benders`。
- 运行指标会同时输出为文本 note 与结构化 diagnostics：
  - `benders_iterations`
  - `benders_gap`
  - `benders_time_ms`
  - `benders_adaptive_effective`
  - `benders_problem_size_binary_variables`

3. `prefer_benders=true`，Benders 失败，`benders_fallback_to_milp=true`
- 应用层自动重试 MILP。
- 最终路径为 `solver_path=milp_fallback`。
- Benders 失败原因仍通过 `benders_failed` 保留可观测性。

4. `prefer_benders=true`，Benders 失败，`benders_fallback_to_milp=false`
- 响应状态为 `BendersFailed`。
- 不再回退 MILP。
- 通过 `benders_failed` 输出失败诊断。

`BendersFailed` 与 `milp_fallback` 的语义明确不同：
- `BendersFailed`：请求终止在 Benders 失败路径。
- `milp_fallback`：Benders 先失败，但请求通过 MILP 回退恢复并完成求解。

### 在线判定覆盖配置（请求级）

`Demo2Request.benders_quality_overrides` 支持按请求覆盖在线质量守卫阈值。
若不传，则使用默认守卫配置。

可覆盖字段：
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

示例：

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

当走 Benders 路径时，响应中的 notes/diagnostics 会输出
`benders_quality_guard_effective`，用于观测最终生效阈值。

## API 入口文件

- 公共示例入口：`src/framework_demo/demo2/mod.rs`
- 公共调度模块：`src/framework_demo/mod.rs`（`framework_demo::run_demo2`）
- 当前实现位置（兼容转发目标）：`src/framework/demo2/domain.rs`
- 请求/响应 DTO：`src/framework/demo2/infrastructure/dto.rs`
- 诊断解析与 grouped-note 约定：`src/framework/demo2/diagnostics.rs`
