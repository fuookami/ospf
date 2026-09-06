# 任务生成

:us: [English](README.md) | :cn: 简体中文

本目录是从 Kotlin `gantt-scheduling-domain-task-generation-context` 映射而来的任务生成扩展点。

## 职责

Rust workspace 目前将该模块保留为未来任务生成流程与扩展 hook 的占位入口。

## 模块

- `mod.rs`：为未来 task-generation context 代码保留的模块入口。

## Public API

目前尚未导出 public task-generation API。

## 扩展点

未来任务生成应在这里新增 context、policy 和 generator trait，同时保持生成任务词汇兼容 `task`，并可被 `task_compilation` 编译。

## 生命周期与数据流

目前没有运行时流程。预期边界是 task compilation 之前的上游任务创建，以及下游 scheduling workflow。

## 验证

新增 task-generation API 后运行 `cargo check -p ospf-rust-framework-gantt-scheduling`，并为 generation policy 或 context 补充聚焦测试。

## 相关目录

- [`../task`](../task/README_ch.md)
- [`../task_compilation`](../task_compilation/README_ch.md)
