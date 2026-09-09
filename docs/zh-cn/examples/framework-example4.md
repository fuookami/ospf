# 复杂示例 4：航班恢复分支定价 — 总览

[English](../../examples/framework-example4)

## 1. 概述

Demo4 是架构示例。`Application` 为空类，通用量示例可运行；上下文页面描述已经实现的契约，不宣称已经组装了完整生产模型。

## 2. 上下文图与依赖

Task、rule、crew 和 cargo 数据输入 bunch generation；生成的 bunch 输入 bunch compilation；bunch selection 协调分支定价策略。Passenger 可选地提供编译 Pipeline。

## 3. 概念、集合与谓词

词汇包括航班任务、航段、机组、货物、乘客、可行 bunch、生成列和选中列。谓词区分可行任务转移、兼容资源和被编译上下文接受的列。

## 4. 变量与中间值

编译上下文可以选择 bunch 列 `x_b`，并计算任务/飞机/航段覆盖表达式。Generation 和 selection 是服务与回调，不创建独立主问题变量族。

## 5. 断言、约束与目标

已实现编译限制覆盖任务、航段、机队、容量关系和生成列一致性。注册 Passenger Pipeline 时会增加相应约束及目标项。`Application` 中不存在完整全局分支定价目标。

## 6. 算法与生命周期

预期流程是构造策略、初始化影子价格、生成 bunch、编译主问题、计算约化成本并执行分支。通用量示例只演示量类型和线性符号 API。

## 7. Register → construct → solve → analyze

每个上下文只注册自己的契约。由于顶层 `Application` 为空，目前没有可运行模型的端到端注册、求解和分析流程。

## 8. 源码与验证

- [Kotlin Demo4 源码](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo4)
- [Rust Demo4 源码](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src/core/demo4.rs)

## 9. Kotlin/Rust 对照与设计决策

两个实现都是架构参考。上下文注册状态和通用量行为必须分别核对源码，不能推导未实现的全局求解模型。

## 10. 上下文模型页面

[打开 Demo4 上下文索引](framework-example4/domain-models)，其中包含 task、rule、crew、cargo、passenger、bunch generation、bunch compilation 和 bunch selection。

## 11. 变更记录

| 版本 | 变更 | 原因 |
|---|---|---|
| 1.0 | 统一架构总览和实现边界 | 避免暗示不存在的全局模型 |

