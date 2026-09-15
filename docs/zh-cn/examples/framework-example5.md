# 复杂示例 5：VRPTW 分支定价 — 总览

[English](/examples/framework-example5)

## 1. 概述

Demo5 是可运行的 VRPTW 集成。它建立基于路线的受限主问题，定价带资源约束的初等路线，并通过 network-scheduling framework 执行分支定价。

## 2. 上下文与依赖

| 上下文 | 职责 | 依赖 |
|---|---|---|
| 车辆路径（VRP） | 客户、车型、路线、资源与验证 | 归一化输入、计算策略 |
| 路线生成 | 资源约束初等路线定价 | VRP、对偶价格、分支规则 |
| 路线编译 | 客户覆盖、车队数量、人工覆盖和两阶段目标 | VRP、已生成路线 |

应用层协调分支节点与阶段切换，生成和编译通过路线列及对偶价格交换信息。

## 3. 概念、集合与谓词

$C$ 是客户集合，$V$ 是车型集合，$R_{b,t}$ 是分支节点 $b$ 在迭代 $t$ 已插入的兼容路线列。谓词区分客户覆盖、初等路线、资源可行性与分支兼容性。

## 4. 变量与中间值

$x_r$ 是路线使用量，$u_i$ 是第一阶段人工覆盖。主问题计算客户覆盖量 $Y_i$ 和车队使用量 $F_v$；路线生成计算标签载荷、服务开始时刻和约化成本。输入属性、算法状态、线性中间值和求解器变量在子页中分别说明。

## 5. 断言、约束与目标

客户覆盖满足 $Y_i+u_i=1$，车队使用不超过可用数量。第一阶段最小化人工覆盖，第二阶段固定 $u_i=0$ 后最小化路线成本，不使用一个含任意大常数的混合目标。路线容量与时间窗由定价和路线验证保证。

## 6. 算法与生命周期

应用生成初始路线并求解节点 LP，通过 ESPPRC 搜索改进列，再处理分数解和分支节点。定价需要同时计入客户与车队对偶价格；受返回列数等限制的定价不能直接视为完成。

## 7. 注册 → 构造 → 求解 → 分析

VRP 校验并归一化输入；路线生成构造定价图；路线编译注册受限主问题；分支定价协调主问题、定价与两阶段切换；分析阶段将整数路线使用量还原为经过验证的方案。

## 8. 源码入口

- [Kotlin Demo5 源码](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo5)
- [Rust Demo5 源码](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src/framework/demo5)

## 9. Kotlin/Rust 对照与设计决策

两种语言的复杂示例均使用路线列框架。公式区分节点 LP 与最终整数方案，并区分第一阶段可行性恢复与第二阶段成本优化；直接弧模型测试参照不替代这里的路线列主模型。

## 10. 上下文模型页面

- [车辆路径上下文](framework-example5/domain-vrp/domain-model)
- [路线生成上下文](framework-example5/domain-route-generation/domain-model)
- [路线编译上下文](framework-example5/domain-route-compilation/domain-model)

## 11. 变更记录

| 版本 | 变更 | 原因 |
|---|---|---|
| 1.1 | 统一中英文总览、数学记号和源码入口 | 与所属上下文模型保持一致 |
