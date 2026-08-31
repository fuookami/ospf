# 应用服务

[English](README.md)

本目录为应用层求解流程提供易用构造函数和默认策略。它刻意保持轻量：service 函数组装 algorithm 与 domain context，而不直接构造优化表达式。

## 模块

- `task.rs`：创建任务列生成流程的辅助入口。
- `bunch.rs`：任务束列生成、branch-and-price 构造、fresh-model 搜索和默认任务束生成策略。

## 公共 API

- `create_task_column_generation`
- `create_bunch_branch_and_price`
- `search_bunch_branch_and_price_with_fresh_model`
- `search_bunch_branch_and_price_with_hooks`
- `DefaultBunchGenerationPolicy`

## 使用方式

应用调用方需要可直接组合的求解流程时，应优先使用本目录。领域扩展、成本公式、可行性规则和自定义回调应通过 `application::algorithm` 与 `domain` 暴露的 policy / hook 类型注入。

## 相关目录

- [`../algorithm`](../algorithm/README_ch.md)
- [`../../domain`](../../domain/README_ch.md)
