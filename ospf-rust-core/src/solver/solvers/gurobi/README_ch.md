# Gurobi 求解器说明

English version: [README.md](./README.md)

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

## 原生回调语义

1. `add_native_callback` 为覆盖语义（后者覆盖前者）。
2. `add_native_observer` 为累加多播语义。
3. native observer 可返回 `Terminate` 请求终止求解。

## 最小验证命令

在 workspace 根目录执行：

```bash
cargo test -p ospf-rust-core gurobi_native_observer_integration --features gurobi12 -- --nocapture
cargo test -p ospf-rust-core gurobi_telemetry_callback_integration --features gurobi12 -- --nocapture
cargo test -p ospf-rust-core gurobi_stage_callback_integration --features gurobi12 -- --nocapture
```

若当前环境是 Gurobi 10/11，请将 `gurobi12` 替换为 `gurobi10`/`gurobi11`。
