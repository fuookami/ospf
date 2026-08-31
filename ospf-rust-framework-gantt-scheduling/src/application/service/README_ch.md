# 应用服务

:us: [English](README.md) | :cn: 简体中文

本目录为应用层求解流程提供易用构造函数和默认策略。它刻意保持轻量：service 函数组装 algorithm 与 domain context，而不直接构造优化表达式。

## 职责

- 为任务级和任务束级求解流程提供可直接组合的构造函数。
- 将默认策略放在 application 边界便于发现。
- 通过 algorithm policy、hook 和 context 参数传递领域定制。

## 模块

- `task.rs`：创建任务列生成流程的辅助入口。
- `bunch.rs`：任务束列生成、branch-and-price 构造、fresh-model 搜索和默认任务束生成策略。

## 公共 API

- `create_task_column_generation`
- `create_bunch_branch_and_price`
- `search_bunch_branch_and_price_with_fresh_model`
- `search_bunch_branch_and_price_with_hooks`
- `DefaultBunchGenerationPolicy`

## 扩展点

应用调用方需要可直接组合的求解流程时，应优先使用本目录。领域扩展、成本公式、可行性规则和自定义回调应通过 `application::algorithm` 与 `domain` 暴露的 policy / hook 类型注入。

## 生命周期与数据流

service constructor 组装 algorithm policy、domain compilation/generation context 和可选 hook，然后返回 application-level workflow。返回的 workflow 仍然把模型注册和 pricing 语义委托给 domain 模块。

## 验证

运行 `cargo test -p ospf-rust-framework-gantt-scheduling --lib`，修改默认策略或 hook wiring 时覆盖 service constructor 路径。

## 相关目录

- [`../algorithm`](../algorithm/README_ch.md)
- [`../../domain`](../../domain/README_ch.md)
