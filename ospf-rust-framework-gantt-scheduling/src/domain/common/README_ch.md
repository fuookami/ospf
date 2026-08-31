# 通用领域辅助

[English](README.md)

本目录存放 Gantt 各子领域共享的辅助结构。

## 职责

- 提供共享的约束索引类型。
- 重导出 `ospf_rust_framework::model` 的动态模型生命周期辅助类型。
- 将跨领域状态辅助保持在领域层附近，避免重复实现。

## 模块

- `constraint_index.rs`：约束 key、entry 和 map 辅助。

## 公共 API

- `ConstraintIndexEntry`
- `ConstraintIndexKey`
- `ConstraintIndexMap`
- `GanttDynamicModelLifecycle`
- `GanttDynamicModelSnapshot`
- `GanttModelStateFacade`

## 相关目录

- [`../task`](../task/README_ch.md)
- [`../task_compilation`](../task_compilation/README_ch.md)
- [`../../application`](../../application/README_ch.md)
