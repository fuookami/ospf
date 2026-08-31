# 产出与消耗领域

[English](README.md)

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

## 相关目录

- [`../resource`](../resource/README_ch.md)
- [`../task`](../task/README_ch.md)
- [`../capacity_scheduling`](../capacity_scheduling/README_ch.md)
