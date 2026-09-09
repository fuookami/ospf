# 复杂示例 3：一维分切 — 总览

[English](../../examples/framework-example3)

## 1. 概述

本示例使用 csp1d framework API 建立受限主问题并执行列生成。Material 和 Produce 是核心领域上下文，定价和目标策略在其上扩展。

## 2. 上下文图与依赖

依赖方向为 `material → produce → cutting-plan-generation`；length assignment 和 wasting minimization 提供可选约束及目标项。应用客户端负责注册和列生成循环。

## 3. 概念、集合与谓词

`P` 是产品集合，`M` 是材料集合，`E` 是机器/资源集合，`K` 是当前切割计划列集合。谓词区分有需求产品、可行计划、有效列和使用某种材料的产品。

## 4. 变量与中间值

生产量是非负整数 `x_p`，RMP 列变量是 `lambda_k`。中间值包括材料用量 `u_m`、机器工时 `H_e`、容量 `C_e`、需求贡献 `a_{pk}` 和废料 `w_k`。

## 5. 断言、约束与目标

有效列必须覆盖需求，机器工时不得超过容量，计划废料不得为负，并满足可选长度规则。当前 Produce 与 wasting-minimization Pipeline 注册生产、材料和废料成本目标。

## 6. 算法与生命周期

客户端创建初始可行计划，注册 RMP，求解并读取对偶价格，定价新计划，加入改进列，直到不存在负约化成本计划。

## 7. Register → construct → solve → analyze

上下文构建器注册变量和 Pipeline；框架构造 RMP 与定价模型；求解器交替执行主问题和定价迭代；分析阶段输出生产量和废料。

## 8. 源码与验证

- [Kotlin Demo3 源码](https://github.com/fuookami/ospf-kotlin/tree/main/ospf-kotlin-example/src/main/fuookami/ospf/kotlin/example/framework_demo/demo3)
- [Rust Demo3 源码](https://github.com/fuookami/ospf-rust/tree/main/ospf-rust-example/src/core/demo3.rs)

## 9. Kotlin/Rust 对照与设计决策

两个版本共享列生成词汇，但系数名称、数值域和有效目标包装器必须以对应语言源码为准。

## 10. 上下文模型页面

- [Material 上下文](framework-example3/domain-material/domain-model)
- [Produce 上下文](framework-example3/domain-produce/domain-model)

## 11. 变更记录

| 版本 | 变更 | 原因 |
|---|---|---|
| 1.0 | 统一总览结构和 RMP 生命周期 | 与上下文页面保持一致 |

