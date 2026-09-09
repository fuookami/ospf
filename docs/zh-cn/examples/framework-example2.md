# 复杂示例 2：航空货运装载规划 — 总览

[English](../../examples/framework-example2)

## 1. 概述

本示例说明航空货运装载领域及其按模式注册的上下文。本文是总览，11 个有界上下文的契约分别维护在子页面中。

## 2. 上下文图与依赖

公共依赖方向为 `aircraft → stowage → {mac, airworthiness_security, soft_security, mac_optimization, express_effectiveness, loading_effectiveness, redundancy, recommended_weight_equalization, payload_maximization}`。LoadingOrder、FullLoad、Predistribution 和 WeightRecommendation 注册不同子集。

## 3. 概念、集合与谓词

`I` 是货物集合，`J` 是机位集合，`P` 是飞行阶段集合。谓词区分已分配货物、空机位、装载区域、有效阶段和当前模式 Pipeline。Aircraft 是配置数据，Stowage 及安全相关上下文提供优化符号。

## 4. 变量与中间值

核心符号包括分配 `x_{ij}`、调整 `u_{ij}`、装载量 `y_j` 和建议量 `z_j`。中间值包括机位载荷 `L_j`、各阶段力矩/MAC、区域密度、载荷总量、空位指示量和建议偏差。有效符号由应用模式决定。

## 5. 断言、约束与目标

有效约束包括货物分配、调整范围、装载限制、MAC/力矩和适航包络、软安全惩罚、装载顺序、冗余、建议重量均衡和最大载荷。上下文页面不表示其 Pipeline 在所有模式中都被注册。

## 6. 算法与生命周期

应用选择模式，初始化 aircraft/stowage 数据，注册模式相关 Pipeline，可选构造 Benders 分解，求解 MILP，并分析装载方案。

## 7. Register → construct → solve → analyze

模式选择决定注册边界。`register` 只创建该模式的变量和 Pipeline；`construct` 构造元模型；`solve` 执行 MILP/Benders 路径；`analyze` 返回机位、载荷、MAC 和安全结果。

## 8. 源码与验证

- [Kotlin Demo2 源码](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo2)
- [Rust Demo2 源码](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src/core/demo2.rs)

## 9. Kotlin/Rust 对照与设计决策

两个实现共享上下文词汇，但模式注册和变量边界必须以各语言的 Pipeline 生成器为准。子页面描述的是有效数学边界，而不是单一全局模型。

## 10. 上下文模型页面

[打开 Demo2 上下文索引](framework-example2/domain-models)，其中包含 aircraft、stowage、MAC、安全、效果、冗余、建议重量和载荷上下文。

## 11. 变更记录

| 版本 | 变更 | 原因 |
|---|---|---|
| 1.0 | 统一总览结构并明确模式边界 | 使上下文注册关系可审查 |

