# 应用层

[English](README.md)

本目录是 Gantt Scheduling framework 面向应用调用方的编排层，对应 Kotlin `gantt-scheduling-application` 模块，但建模细节仍下沉到 domain context 与 pipeline。

## 职责

- 编排任务级与任务束级求解流程。
- 组合列生成、分支定价和服务层辅助入口。
- 维护迭代快照、trace 友好的应用状态和公共入口标记。
- 将变量、约束、目标和解提取细节委托给 `domain`。

## 模块

- [`algorithm`](algorithm/README_ch.md)：列生成策略、任务列生成、任务束列生成和 branch-and-price 树搜索。
- [`service`](service/README_ch.md)：应用调用方使用的构造函数和默认服务策略。
- `model`：任务迭代与任务束迭代模型。
- `iteration.rs`：共享迭代与迭代快照结构。

## 入口

- `APS`：高级计划与排程入口标记。
- `MPS`：主生产计划入口标记。
- `LSP`：批次排序计划入口标记。
- `create_task_column_generation`
- `create_bunch_branch_and_price`
- `search_bunch_branch_and_price_with_fresh_model`
- `search_bunch_branch_and_price_with_hooks`

## 边界

应用层不应直接拼装领域变量或硬编码约束族。新增业务规则应先通过 domain context、aggregation、pipeline、policy 或 solver hook 表达，再在本层组合。

## 相关目录

- [`../domain`](../domain/README_ch.md)
- [`../infrastructure`](../infrastructure/README_ch.md)
