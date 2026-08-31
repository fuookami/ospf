# 产能排程

[English](README.md)

本目录包含产能排程模型组件和 pipeline，对应 Kotlin `gantt-scheduling-domain-capacity-scheduling-context` 模块。

## 职责

- 建模生产动作和执行器-时隙分配。
- 注册无序与带序产能编译变量。
- 从 solver-order 解值中提取产能排程解。
- 为迭代和列生成流程维护产能列。
- 提供产能约束和产能成本目标。

## 模块

- `model.rs`：生产动作、产能编译、带序产能编译、产能列、聚合和解提取。
- `service/limits.rs`：执行器产能约束、顺序约束和产能成本最小化。
- `service/mod.rs`：服务层重导出。

## 公共 API

- `ProductionActionTrait`
- `BasicProductionAction`
- `CapacityCompilation`
- `CapacityOrderCompilation`
- `CapacityColumn`
- `CapacityColumnAggregation`
- `CapacitySchedulingSolution`
- `ActionAllocation`
- `ExecutorCapacityResult`
- `ExecutorCapacityConstraint`
- `OrderConstraint`
- `CapacityCostMinimization`
- `CapacitySchedulingAggregation`
- `CapacitySchedulingContext`

## 相关目录

- [`../produce`](../produce/README_ch.md)
- [`../resource`](../resource/README_ch.md)
- [`../bunch_generation`](../bunch_generation/README_ch.md)
