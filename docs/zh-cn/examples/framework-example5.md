# 复杂示例 5：VRPTW 分支定价 — 总览

[English](../../examples/framework-example5)

## 1. 概述

Demo5 是可运行的 VRPTW 集成。它建立基于路线的受限主问题，定价带资源约束的初等路线，并通过 network-scheduling framework 执行分支定价。

## 2. 上下文图与依赖

VRP 负责客户、车辆、路线、单位和验证器。Route generation 消费 VRP 词汇和主问题对偶价格；Route compilation 负责客户覆盖、车队、人工覆盖和路线成本 Pipeline。

## 3. 概念、集合与谓词

`C` 是客户集合，`V` 是车辆类型集合，`R` 是路线列集合，`A` 是有向弧集合。谓词区分可行路线、客户覆盖、容量可行标签和改进定价结果。

## 4. 变量与中间值

编译上下文选择路线列 `x_r` 和 Phase-I 人工覆盖 `a_i`。生成上下文计算标签载荷、到达时间、路线成本和约化成本。VRP 提供需求、服务时间窗、容量以及策略相关距离/成本。

## 5. 断言、约束与目标

路线必须从仓库出发并返回仓库，满足客户访问、容量和时间窗规则。主问题约束客户覆盖和车队上限，Phase I 惩罚人工覆盖，Phase II 最小化路线成本。

## 6. 算法与生命周期

应用初始化实例和策略，生成初始路线，求解受限主问题，定价 ESPPRC 路线，加入改进列，并在达到终止条件前继续分支。

## 7. Register → construct → solve → analyze

VRP 验证输入；Route generation 构造定价图；Route compilation 注册主问题；分支定价交替执行主问题和定价求解；分析阶段返回路线及客户/车队结果。

## 8. 源码与验证

- [Kotlin Demo5 源码](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo5)
- [Rust Demo5 源码](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src/core/demo5.rs)
- [Kotlin network-scheduling 上下文](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-framework-network-scheduling)

## 9. Kotlin/Rust 对照与设计决策

两个版本共享路线/主问题词汇，但适配器、数值域、成本策略和分支细节因语言实现而异。上下文页面标明当前注册的数学边界。

## 10. 上下文模型页面

- [VRP 上下文](framework-example5/domain-vrp/domain-model)
- [Route generation 上下文](framework-example5/domain-route-generation/domain-model)
- [Route compilation 上下文](framework-example5/domain-route-compilation/domain-model)

## 11. 变更记录

| 版本 | 变更 | 原因 |
|---|---|---|
| 1.0 | 统一 VRPTW 总览结构 | 与其他复杂示例保持一致 |

