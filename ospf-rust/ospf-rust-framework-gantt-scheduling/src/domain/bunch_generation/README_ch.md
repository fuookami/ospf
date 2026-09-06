# 任务束生成

:us: [English](README.md) | :cn: 简体中文

本目录实现定价侧的任务束生成，对应 Kotlin `gantt-scheduling-domain-bunch-generation-context` 模块。

## 职责

- 为列生成定价问题构造候选任务束。
- 表达任务转移图和标签状态。
- 在返回生成列前应用可行性规则和时隙约束。
- 支持已计划任务、未计划任务和按时隙生成流程。

## 模块

- `model.rs`：图和任务束生成模型结构。
- `label.rs`：标签状态和标签扩展辅助结构。
- `pricing.rs`：`BunchPricingProblem` 与 `LabelSettingAlgorithm`。
- `service.rs`：任务束生成器、可行性策略、时隙约束和生成配置。

## 公共 API

- `BunchPricingProblem`
- `LabelSettingAlgorithm`
- `BunchGenerationConfig`
- `BunchFeasibilityPolicy`
- `DefaultBunchFeasibilityPolicy`
- `PlannedTaskBunchGenerator`
- `UnplannedTaskBunchGenerator`
- `SlotBasedBunchGenerator`

## 扩展点

pricing 行为通过 `BunchFeasibilityPolicy`、`BunchGenerationConfig`、时隙约束、label-state 扩展或专用 generator 实现扩展。主问题列注册保留在 `bunch_compilation`。

## 生命周期与数据流

pricing 接收任务图、shadow price 和生成配置，运行 label-setting 或 slot-based generation，通过可行性策略过滤候选任务束，并返回可由 bunch compilation 注册到主问题的列。

## 验证

修改 label expansion、feasibility policy、planned/unplanned generation 或 slot-based generation 时运行 `cargo test -p ospf-rust-framework-gantt-scheduling --lib`。

## 相关目录

- [`../bunch_compilation`](../bunch_compilation/README_ch.md)
- [`../task`](../task/README_ch.md)
- [`../../application/algorithm`](../../application/algorithm/README_ch.md)
