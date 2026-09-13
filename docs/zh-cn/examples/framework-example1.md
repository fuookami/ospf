# 复杂示例 1：服务放置 — 总览

[English](/examples/framework-example1)

## 1. 概述

本示例用路由（Route）和带宽（Bandwidth）两个有界上下文表达服务放置与网络带宽分配。总览介绍协作关系；子页给出 Kotlin 示例实际注册的变量、中间值和约束。

## 2. 上下文与依赖

| 上下文 | 职责 | 依赖 |
|---|---|---|
| Route | 图、客户端、服务、分配变量、分配约束和服务成本目标 | — |
| Bandwidth | 边/服务/节点带宽表达式和有效带宽约束 | Route |

## 3. 概念、集合与谓词

$N$ 是节点集合，$E$ 是有向边集合，$S$ 是服务集合。$N^{normal}$ 与 $N^{client}$ 分别表示普通节点和客户端节点；$E^{normal}$ 表示源节点为普通节点的边，与子页采用相同记号。

## 4. 变量与中间值

$x_{n,s}$ 是二元服务分配变量，客户端行固定为零；$y_{e,s}$ 是受边界限制的非负整数带宽。服务分配计数 $A_s$、节点分配计数 $A_n$、边总带宽 $B_e$ 和节点入带宽 $I_n$ 均为这些变量的线性中间值，客户需求则是输入参数。

## 5. 断言、约束与目标

路由上下文限制每个普通节点至多承载一个服务、每个服务至多放置一次，并注册服务成本目标。带宽上下文注册边带宽、客户需求、服务容量约束及带宽成本目标。未被管线生成器返回的 `TransferNodeBandwidthConstraint` 不属于已注册模型；净流出量定义也不能代替流守恒约束。

## 6. 算法与生命周期

应用初始化 Route，将其传入 Bandwidth，注册两个聚合根，构造元模型，使用 SCIP 求解，并分析分配和带宽结果。当前应用没有额外引入流守恒等式。

## 7. 注册 → 构造 → 求解 → 分析

`init` 创建上下文和数据；`register` 添加变量和 Pipeline；`construct` 构造求解模型；`solve` 调用求解器；`analyze` 将结果绑定回领域对象。

## 8. 源码入口

- [Kotlin Demo1 源码](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo1)
- [Rust Demo1 源码](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src/framework/demo1)

## 9. Kotlin/Rust 对照与设计决策

Kotlin 与 Rust 的复杂示例均具有路由和带宽上下文。下方链接指向各自的框架示例，而非同编号的简单示例。子页以 Kotlin 注册结果为数学模型基准，不因问题名称而隐式增加约束。

## 10. 上下文模型页面

- [路由上下文](framework-example1/domain-route/domain-model)
- [带宽上下文](framework-example1/domain-bandwidth/domain-model)

## 11. 变更记录

| 版本 | 变更 | 原因 |
|---|---|---|
| 1.1 | 统一中英文总览、数学记号和源码入口 | 与所属上下文模型保持一致 |
