# 产能排程

:us: [English](README.md) | :cn: 简体中文

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

## 扩展点

产能排程通过 `ProductionActionTrait`、capacity column、ordered compilation，以及 executor capacity、order constraint、capacity-cost objective 等 limit pipeline 扩展。资源词汇保留在 `resource`，物料流词汇保留在 `produce`。

## 生命周期与数据流

production action 和 executor-slot 候选被编译为 capacity variable；可选 ordered compilation 注册顺序敏感变量；limit pipeline 注册产能约束和成本目标；solution extraction 输出 action allocation 与 executor capacity result。

## 验证

修改 capacity column、ordered compilation、capacity limit 或 solution extraction 时运行 `cargo test -p ospf-rust-framework-gantt-scheduling --lib`。

## 相关目录

- [`../produce`](../produce/README_ch.md)
- [`../resource`](../resource/README_ch.md)
- [`../bunch_generation`](../bunch_generation/README_ch.md)
