# 应用层算法

:us: [English](README.md) | :cn: 简体中文

本目录放置应用层求解算法。算法负责协调模型生命周期、定价、分支和策略决策，具体建模注册仍由 domain 模块负责。

## 职责

- 编排任务级和任务束级模型生命周期，但不持有领域表达式。
- 运行列生成迭代、branch-and-price 搜索、回调和可选强分支。
- 将搜索策略、hook 和迭代状态保持在 application 边界附近。

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

## 扩展点

本目录的算法应编排既有 context 与 service。它们可以管理生命周期快照、warm start 状态、节点搜索顺序和回调分发，但领域约束与目标应保留在 `src/domain` 中。

## 生命周期与数据流

列生成算法创建或刷新主问题模型，向 pricing service 请求新列，记录迭代快照，并在满足 policy 阈值后停止。branch-and-price 将任务束列生成流程包在节点选择、分支决策、cut 和 callback hook 之上。

## 验证

运行 `cargo test -p ospf-rust-framework-gantt-scheduling --lib`，重点关注 task column generation、bunch column generation 和 branch-and-price 测试。

## 相关目录

- [`../service`](../service/README_ch.md)
- [`../../domain/bunch_compilation`](../../domain/bunch_compilation/README_ch.md)
- [`../../domain/bunch_generation`](../../domain/bunch_generation/README_ch.md)
- [`../../domain/task_compilation`](../../domain/task_compilation/README_ch.md)
