# OSPF Rust Example

:us: [English](README.md) | :cn: 简体中文

## 简介

`ospf-rust-example` 提供基于 `ospf-rust-core`、`ospf-rust-framework` 和领域 framework crate 的可运行、可测试示例。它把 Kotlin example/demo 职责映射为 Rust 命令入口和迁移兼容示例。

## 作用范围

本 crate 拥有示例、命令分发、迁移兼容 demo、请求/响应 DTO 示例，以及用于验证 framework 使用方式的行为契约。

明确非目标：

1. 属于 `ospf-rust-core` 或 `ospf-rust-framework` 的共享建模抽象。
2. 属于 CSP1D、BPP3D 或 Gantt Scheduling 等领域 crate 的可复用领域 framework 逻辑。
3. 生产级请求协议或部署 adapter。

## 模块结构

| Rust 路径 | 职责 |
| --- | --- |
| `src/main.rs` | 命令分发入口。 |
| `src/example_modeling.rs` | 共享建模和 typed solve helper。 |
| `src/core` | core 建模 demo、shortcut API、capability gate 和 backend-aware 示例。 |
| `src/framework/demo1` | framework routing/bandwidth context 示例。 |
| `src/framework/demo2` | Adaptive Benders / MILP fallback 行为契约示例。 |
| `src/framework/demo3` | CSP1D-facing demo scaffold。 |
| `src/framework/demo4` | Gantt-scheduling-facing scaffold 与 domain layout experiment。 |
| `src/framework/demo5` | Solomon parser/adapter 与 VRPTW Branch-and-Price solver wiring。 |

## Public API

本 crate 主要作为 executable 使用。稳定表面是命令名称和已文档化的行为契约：

| Command | 职责 | Backend |
| --- | --- | --- |
| `core:demo1` | core 建模 demo。 | Gurobi feature |
| `core:all` | 运行 core demo set。 | Gurobi feature |
| `core:generic-number` | generic-number 建模路径。 | Gurobi feature |
| `core:shortcuts` | `MetaModel` shortcut API demo。 | Gurobi feature |
| `framework:demo1` | framework routing/bandwidth context demo。 | Gurobi feature |
| `framework:demo2` | Adaptive Benders 和 MILP fallback demo。 | Gurobi feature |
| `framework:demo3` | CSP1D scaffold 入口。 | Gurobi feature |
| `framework:demo4` | Gantt scaffold 入口。 | Gurobi feature |
| `framework:demo5` | VRPTW Branch-and-Price demo。 | `demo5-gurobi-bp` 或 `demo5-scip-bp` |

## 命令说明

默认构建/测试路径，不依赖商业后端：

```powershell
cargo check -p ospf-rust-example
cargo test -p ospf-rust-example --no-run
cargo test -p ospf-rust-example
```

需要后端的运行命令：

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

后端 build-only 验证：

```powershell
cargo test -p ospf-rust-example --features backend-gurobi --no-run
cargo check -p ospf-rust-example --features demo5-gurobi-bp
cargo check -p ospf-rust-example --features demo5-scip-bp
```

未启用 backend feature 时，运行后端 demo 会返回提示 `rerun with --features backend-gurobi`。

`framework:demo4` 当前保持为 scaffold 命令入口。Gantt-scheduling parity 明确暂缓，不标记为已完成功能。

`framework:demo5` 使用 inline Solomon fixture，验证 parser、adapter、route generation、restricted master 和 Branch-and-Price wiring。integration target 只覆盖 target 自身的夹具/parser 检查；下面的库内门禁命令才会执行 25 客户 direct-MIP 交叉验证、5 客户全路线 master oracle、100 客户 smoke、strict-proof fixture 和 SCIP 合法终态：

```powershell
cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib demo17_25_branch_and_price_matches_direct_mip_objective -- --include-ignored
cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib demo17_100_branch_and_price_smoke_respects_limits -- --include-ignored
cargo test -p ospf-rust-example --features demo5-gurobi-bp --lib proof_100_customer_fixture_closes_direct_mip_and_branch_and_price_bounds -- --include-ignored
cargo test -p ospf-rust-example --features demo5-scip-bp --lib demo17_25_scip_branch_and_price_returns_legal_terminal -- --include-ignored
```

solver 和 pricing 失败路径由 network crate 的离线测试覆盖。缺少 native runtime 或许可证时，门禁必须失败，不能计为通过。

## Demo2 的 Benders 行为契约

`framework::demo2` 支持自适应 Benders，并可选 MILP fallback。

当 `prefer_benders=true` 时，应用层会基于问题规模（`cargo_count * position_count`）推导生效的 adaptive profile，并在 notes 中同时输出配置参数和生效参数。生效参数包含用于 cut stagnation 的 `max_stall_iterations`，以及用于 objective-improvement stagnation 的 `objective_stall_iterations`。

### 状态与路径语义

1. `prefer_benders=false`：直接走 MILP；diagnostics 包含 `solver_path=milp_direct`。
2. `prefer_benders=true` 且 Benders 成功：响应状态为 `Feasible` 或 `Optimal`；diagnostics 包含 `solver_path=benders`。
3. `prefer_benders=true`、Benders 失败且 `benders_fallback_to_milp=true`：应用层重试 MILP；最终路径为 `solver_path=milp_fallback`；`benders_failed` 保持可观测。
4. `prefer_benders=true`、Benders 失败且 `benders_fallback_to_milp=false`：响应状态为 `BendersFailed`；不再重试 MILP。

`BendersFailed` 表示请求终止在 Benders 错误路径。`milp_fallback` 表示 Benders 先失败，但请求通过 MILP 恢复。

### 在线判定覆盖配置

`Demo2Request.benders_quality_overrides` 支持按请求覆盖在线质量守卫阈值。不传时使用默认守卫配置。

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

走 Benders 路径时，响应 notes/diagnostics 会输出 `benders_quality_guard_effective`，用于观测最终生效阈值。

## 本地验证

```powershell
cargo check -p ospf-rust-example
cargo test -p ospf-rust-example --no-run
cargo test -p ospf-rust-example
cargo test -p ospf-rust-example --features backend-gurobi --no-run
```

## 当前边界

本 crate 允许保留迁移兼容布局和 scaffold 命令入口。在这里沉淀出的可复用建模模式，应在下游依赖前上移到合适的 core/framework/domain crate。

## 相关模块

- [根 README](../README_ch.md)
- [Core README](../ospf-rust-core/README_ch.md)
- [Framework README](../ospf-rust-framework/README_ch.md)
- [Kotlin example README](../../ospf-kotlin/ospf-kotlin-example/README_ch.md)
