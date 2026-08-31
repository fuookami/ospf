# 任务束生成

[English](README.md)

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

## 相关目录

- [`../bunch_compilation`](../bunch_compilation)
- [`../task`](../task/README_ch.md)
- [`../../application/algorithm`](../../application/algorithm/README_ch.md)
