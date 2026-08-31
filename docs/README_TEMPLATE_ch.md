# <module-name>

:us: [English](README.md) | :cn: 简体中文

## 简介

简要说明本模块提供什么能力、位于 `ospf-rust` workspace 的哪一层，以及在需要时对应 Kotlin 的哪个模块或包。

## 作用范围

说明本模块负责什么。

明确非目标：

1. 业务专用 DTO、公式语言、租户上下文、心跳逻辑和运行时策略，除非本模块明确拥有这些能力。
2. solver backend 实现，除非这是求解器后端模块。
3. starter 依赖聚合或 demo 编排，除非这是 example 或 starter 模块。

## 模块结构

| Rust 模块或目录 | Kotlin 边界 | 职责 |
| --- | --- | --- |
| `<path>` | `<kotlin-module-or-package>` | `<responsibility>` |

## 架构概览

framework 模块说明 context / aggregation / model component / pipeline 流程和 `MetaModel` 注册路径；工具模块说明主要类型和数据流边界。

## 核心概念

列出读者使用或扩展本模块前需要理解的概念。

## Public API

| API | 职责 | 稳定性 |
| --- | --- | --- |
| `<TypeOrFunction>` | `<responsibility>` | stable / migration / internal |

## 建模扩展点

framework 模块说明 context、aggregation、model component、pipeline、extra context 或 extra pipeline 扩展点。非 framework 模块可省略本节，或改写为对应扩展面。

## 泛型数值边界

说明 public API 是否使用泛型数值类型，以及哪里允许转换为 `f64`。

## 物理量边界

说明哪些值使用 `Quantity<V, U>` 或明确物理单位包装，哪些值可以保持无量纲。

## 求解生命周期

framework 模块按实际情况说明注册、LP/MILP 求解、shadow price 提取、加列/删列、最终求解和结果提取。

## 输出

说明 solution、trace、KPI、render DTO 或诊断输出。

## 使用方式

```rust
// 最小示例。
```

## 本地验证

```powershell
cargo check -p <crate-name>
cargo test -p <crate-name>
```

## 当前边界

迁移型模块说明已对齐的 Kotlin 行为、Rust 替代实现、剩余差距和长期非目标。

## 相关模块

- [根 README](../README_ch.md)
