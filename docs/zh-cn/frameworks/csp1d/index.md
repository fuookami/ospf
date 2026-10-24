# 一维分切 CSP1D

原料具有可用幅宽，订单需要不同宽度的产品。选择切割方案及其使用次数，让产出满足需求，并按配置处理原料、设备、超欠产与余料。

## 最少需要理解的对象

| 业务概念 | API | 输入或输出 |
| --- | --- | --- |
| 产品与需求 | `Product` / `ProductDemand` | 宽度、可选长度、数量和计量模式 |
| 原料与设备 | `Material` / `Machine` | 幅宽、批次、设备及可选资源限制 |
| 切割方案 | `CuttingPlan` | 每次执行对各产品的产出贡献 |
| 问题 | `Csp1dProblem` | 领域数据与求解配置 |
| 求解与结果 | `Csp1dMilp` / `Csp1dColumnGeneration` | 方案使用、产出、需求满足与增强指标 |

**变量是方案使用次数，不是产品产量。** 产品产量由各方案贡献乘使用次数汇总。卷数、片数、长度和重量需求不能互换。

## 从哪个入口调用？

两种语言都提供普通 MILP 和列生成入口，但 adapter、返回类型与配置不同。先学普通入口，再观察初始方案、LP、定价、加列和最终整数求解。

下游 HTTP DTO、租户、公式语言和运行时策略留在业务适配层，不属于这个通用内核。

## 能力边界

- 需求的产品、数量和单位必须同时明确；不能把不同单位的需求合成一个裸数。
- 设备有数据不代表每一种产能约束都自动存在，必须核对方案资源消耗与配置。
- 初始方案不足可能需要可行性阶段；受限池不可行不等于完整业务问题不可行。
- 启发式或有界定价结束不自动证明整数最优。

## 学习路线

1. 完成[环境准备](../setup)。
2. 跟随[第一次卷材分切](./first-slitting)，检查方案如何贡献产量。
3. 增加超产上界、欠产惩罚、余料和长度配置，再考虑自定义管线。
4. 阅读[复杂示例 3](/zh-cn/examples/framework-example3)及其上下文模型，理解组合与列生成。

## 接口参考

- [Kotlin API 与扩展契约](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-csp1d/README_ch.md)
- [Rust API 与求解生命周期](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-csp1d/README_ch.md)

[返回场景选型](../)

## 继续实践

下一课：[需求口径与业务规则](./business-scenario)。将场景迁入自己的应用前，完成[业务接入检查](../integration)。
