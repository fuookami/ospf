# 任务束编译

:us: [English](README.md) | :cn: 简体中文

本目录包含任务束编译的主问题侧实现，对应 Kotlin `gantt-scheduling-domain-bunch-compilation-context`。

## 职责

- 向 `MetaModel` 注册任务束决策变量和聚合表达式。
- 为列生成流程维护迭代任务束列。
- 从 solver 结果中提取任务和任务束排程解。
- 提供时隙级编译和产能预求解辅助能力。
- 暴露定价子问题使用的影子价格 pipeline。

## 模块

- `model.rs`：任务束编译、任务束聚合、entry、解和解摘要。
- `context.rs`：基础与迭代任务束编译上下文。
- `iterative.rs`：迭代任务束编译状态和列操作。
- `slot_based.rs`：时隙级任务束编译上下文和产能预求解器。
- `service.rs`：解分析和服务辅助。

## 公共 API

- `BunchCompilation`
- `BunchAggregation`
- `BunchEntry`
- `BunchSolution`
- `BunchSolutionSummary`
- `IterativeBunchCompilation`
- `BasicBunchCompilationContext`
- `IterativeBunchCompilationContext`
- `BunchShadowPricePipeline`
- `TaskShadowPriceKey`
- `SlotBasedBunchCompilationContext`
- `BasicSlotBasedBunchCompilationContext`
- `SlotBasedCapacityPreSolver`
- `StaticSlotBasedCapacityPreSolver`

## 扩展点

主问题行为通过 bunch compilation context、迭代任务束列操作、时隙级产能预求解器和 shadow-price pipeline 扩展。pricing 规则保留在 `bunch_generation`，任务级变量语义保留在 `task_compilation`。

## 生命周期与数据流

compilation context 将初始任务束列和聚合表达式注册到 `MetaModel`，iterative context 在列生成过程中新增或刷新列，shadow-price pipeline 将对偶值暴露给 pricing，solution analyzer 从最终 solver value 中提取任务和任务束排程。

## 验证

修改 bunch compilation、shadow price 提取、slot-based pre-solve 或 solution analysis 时运行 `cargo test -p ospf-rust-framework-gantt-scheduling --lib`。

## 相关目录

- [`../bunch_generation`](../bunch_generation/README_ch.md)
- [`../task`](../task/README_ch.md)
- [`../../application/algorithm`](../../application/algorithm/README_ch.md)
