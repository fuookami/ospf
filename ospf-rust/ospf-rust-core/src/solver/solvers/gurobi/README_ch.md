# Gurobi 求解器说明

:us: [English](README.md) | :cn: 简体中文

## 前置依赖

`ospf-rust-core` 的 Gurobi 集成需要：

1. 本机已安装 Gurobi。
2. 已配置有效 Gurobi License。
3. Cargo feature 启用以下之一：
- `gurobi10`
- `gurobi11`
- `gurobi12`

## 主要能力

1. 支持 LP/MIP/QP/MIQP 求解。
2. 支持 stage callback 与 telemetry callback。
3. 支持 native callback 与 native observer。
4. 支持数值诊断与数值策略推荐。

## CP 边界

Gurobi 不提供 CP model component。Gantt task-compilation 先构造 immutable CP snapshot；本 backend
只消费精确的 `ExactLowering` MIP facade，并返回统一 `SolveReport`。原生 optional/variable-duration
interval 以及未经验证的 global-constraint 分解不会被声明为 native CP 能力。

## 原生回调语义

1. `add_native_callback` 为覆盖语义（后者覆盖前者）。
2. `add_native_observer` 为累加多播语义。
3. native observer 可返回 `Terminate` 请求终止求解。

## 最小验证命令

在 workspace 根目录执行：

```bash
cargo test -p ospf-rust-core gurobi_native_observer_integration --features gurobi10 -- --nocapture
cargo test -p ospf-rust-core gurobi_telemetry_callback_integration --features gurobi10 -- --nocapture
cargo test -p ospf-rust-core gurobi_stage_callback_integration --features gurobi10 -- --nocapture
```

当前 workspace 按本机 Gurobi 10 环境验证。若使用 Gurobi 11/12，
请将 `gurobi10` 替换为 `gurobi11`/`gurobi12`。

共享 native contract 还会比较 report identity、best bound、gap、solution value 和 constraint residual。
缺少许可证属于 `LICENSE` 错误（包括原生错误码 `10009`），不得当作环境跳过后成功。详见
core README 中的 [Solver 原生验收矩阵](../../../../README_ch.md#solver-原生验收矩阵)。
