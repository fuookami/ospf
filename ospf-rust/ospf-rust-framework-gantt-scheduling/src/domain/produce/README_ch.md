# 产出与消耗领域

:us: [English](README.md) | :cn: 简体中文

本目录建模物料产出与消耗，对应 Kotlin `gantt-scheduling-domain-produce-context`。

## 职责

- 建模物料 trait、产品、半成品、原料、需求和储备。
- 建模用于 `MetaModel` 注册的产出和消耗使用量。
- 提供基于数量的产出与消耗目标和约束。

## 模块

- `model/`：需求、物料、生产任务和使用量结构。
- `service/limits.rs`：产出与消耗数量约束和目标。

## 公共 API

- `MaterialTrait`
- `Product`
- `SemiProduct`
- `RawMaterial`
- `MaterialDemand`
- `MaterialReserves`
- `ProductionTaskTrait`
- `ProduceUsage`
- `ConsumptionUsage`
- `ProduceQuantityConstraint`
- `ConsumptionQuantityConstraint`
- `ProduceOverQuantityMinimization`
- `ProduceLessQuantityMinimization`
- `ProduceQuantityMaximization`
- `ProduceQuantityMinimization`
- `ConsumptionOverQuantityMinimization`
- `ConsumptionLessQuantityMinimization`
- `ConsumptionQuantityMaximization`
- `ConsumptionQuantityMinimization`

## 扩展点

物料流通过 `MaterialTrait`、`ProductionTaskTrait`、produce/consumption usage component 和数量型 limit pipeline 扩展。资源容量逻辑保留在 `resource`，执行器/时间分配保留在 `capacity_scheduling`。

## 生命周期与数据流

material demand 与 reserve 模型定义需求和可用数量，production task 暴露 produce 与 consumption usage，limit pipeline 注册数量约束或目标，下游 scheduling context 将这些表达式与产能和任务决策组合。

## 验证

修改 material model、usage component 或 produce/consumption quantity limit 时运行 `cargo test -p ospf-rust-framework-gantt-scheduling --lib`。

## 相关目录

- [`../resource`](../resource/README_ch.md)
- [`../task`](../task/README_ch.md)
- [`../capacity_scheduling`](../capacity_scheduling/README_ch.md)
