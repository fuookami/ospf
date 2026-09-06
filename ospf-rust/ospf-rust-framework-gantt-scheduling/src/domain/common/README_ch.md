# 通用领域辅助

:us: [English](README.md) | :cn: 简体中文

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

## 扩展点

context 需要稳定访问已生成 constraint token 时使用 `ConstraintIndexMap`。迭代 context 需要跨 Gantt 子领域复用 snapshot、restore 或 state facade 时使用动态模型生命周期重导出。

## 生命周期与数据流

domain context 在模型注册时填充 constraint index，迭代算法读取这些索引用于 shadow price 或 diagnostics，动态 lifecycle helper 让 task、bunch 和 capacity context 复用模型状态转换。

## 验证

修改 constraint index 行为或动态模型生命周期集成时运行 `cargo test -p ospf-rust-framework-gantt-scheduling --lib`。

## 相关目录

- [`../task`](../task/README_ch.md)
- [`../task_compilation`](../task_compilation/README_ch.md)
- [`../../application`](../../application/README_ch.md)
