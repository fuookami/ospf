# 应用层算法

[English](README.md)

本目录放置应用层求解算法。算法负责协调模型生命周期、定价、分支和策略决策，具体建模注册仍由 domain 模块负责。

## 模块

- `policy.rs`：可复用的列生成策略配置。
- `task_column_generation.rs`：任务级列生成算法。
- `bunch_column_generation.rs`：任务束级列生成与 branch-and-price 适配。
- `branch_and_price.rs`：branch-and-price 树搜索、分支决策、节点回调、cut 回调和强分支扩展点。

## 公共 API

- `ColumnGenerationPolicy`
- `TaskColumnGenerationAlgorithm`
- `BunchBranchAndPriceAlgorithm`
- `BunchCGPolicy`
- `BranchAndPriceTreeSearch`
- `BranchSearchConfig`
- `BranchSearchHooks`
- `StrongBranchingStrategy`
- `BranchCutCallback`
- `BranchNodeCallback`

## 设计说明

本目录的算法应编排既有 context 与 service。它们可以管理生命周期快照、warm start 状态、节点搜索顺序和回调分发，但领域约束与目标应保留在 `src/domain` 中。

## 相关目录

- [`../service`](../service/README_ch.md)
- [`../../domain/bunch_compilation`](../../domain/bunch_compilation/README_ch.md)
- [`../../domain/bunch_generation`](../../domain/bunch_generation/README_ch.md)
- [`../../domain/task_compilation`](../../domain/task_compilation/README_ch.md)
