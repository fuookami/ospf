# 复杂示例 1：最短服务路径（SSP）— 总览

[English](../../examples/framework-example1)

## 1. 概述

本示例组合 Route 和 Bandwidth 两个有界上下文，为服务分配普通节点并计算有向边上的带宽。当前 Kotlin Pipeline 的注册结果是事实标准。

## 2. 上下文图与依赖

| 上下文 | 职责 | 依赖 |
|---|---|---|
| Route | 图、客户端、服务、分配变量、分配约束和服务成本目标 | — |
| Bandwidth | 边/服务/节点带宽表达式和有效带宽约束 | Route |

## 3. 概念、集合与谓词

`N` 是节点集合，`E` 是有向边集合，`S` 是服务集合。`N^C` 是客户端子集，`E^N` 是源节点为普通节点的边集合。有效分配必须引用图中的节点和已定义的服务需求。

## 4. 变量与中间值

`x_{n,s}` 是二元分配变量，`y_{e,s}` 是非负整数带宽变量。上下文从这些变量计算服务需求、边使用量、节点使用量和服务成本；客户端行按照当前 `Assignment` 实现固定。

## 5. 断言、约束与目标

有效 Pipeline 约束节点分配、服务分配、边带宽、服务容量和需求。Route 注册服务成本目标；Bandwidth 注册边、需求、服务容量和带宽成本 Pipeline。源码中的 `TransferNodeBandwidthConstraint` 当前没有被 Pipeline 生成器返回，因此不属于有效模型。

## 6. 算法与生命周期

应用初始化 Route，将其传入 Bandwidth，注册两个聚合根，构造元模型，使用 SCIP 求解，并分析分配和带宽结果。当前应用没有额外引入流守恒等式。

## 7. Register → construct → solve → analyze

`init` 创建上下文和数据；`register` 添加变量和 Pipeline；`construct` 构造求解模型；`solve` 调用求解器；`analyze` 将结果绑定回领域对象。

## 8. 源码与验证

- [Kotlin Demo1 源码](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo1)
- [Rust Demo1 源码](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src/core/demo1.rs)

## 9. Kotlin/Rust 对照与设计决策

两个实现都采用 Route/Bandwidth 上下文划分。解释公式时必须以各语言源码中的变量域和有效 Pipeline 清单为准。

## 10. 上下文模型页面

- [Route 上下文](framework-example1/domain-route/domain-model)
- [Bandwidth 上下文](framework-example1/domain-bandwidth/domain-model)

## 11. 变更记录

| 版本 | 变更 | 原因 |
|---|---|---|
| 1.0 | 统一总览结构并说明有效 Pipeline | 与上下文模型模板一致 |

