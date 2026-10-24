# 网络优化 Network

在有向网络中满足供需，遵守运输或处理容量，并最小化费用。之后可以增加网络建设、资源路径、结构与显式故障场景。不要把 Network 只理解为车辆路径。

## 最少需要理解的对象

| 业务概念 | API | 输入或输出 |
| --- | --- | --- |
| 拓扑 | Node / Arc | 稳定身份、端点和平行服务 |
| 商品与供需 | Commodity / Balance | 各商品的节点供需与计量单位 |
| 容量和费用 | Capacity / Cost | 弧及共享资源的上限、单价 |
| 流问题 | `FlowProblem` | 拓扑、商品、费用与资源配置 |
| 注册与提取 | `FlowContext` | 守恒、容量、费用表达式和流量结果 |

整数步长表示计量粒度，不表示整单不可拆。不同商品共同占用资源时，需要明确物理量换算，不能把吨和件直接相加。

## 从哪个入口调用？

独立流入口为 Kotlin `NetworkFlowApplicationService`、Rust `FlowApplicationService`。嵌入其他模型时使用 `FlowContext` 注册，不要再启动一个不共享变量的独立优化。

先学习流与容量，再扩展建设和韧性。VRPTW 的路线列、ESPPRC 和分支定价属于进阶路线。

## 能力边界

- 平行弧需要独立身份，不能只按起点/终点去重。
- 连通性不证明运输容量充足；建成总容量不等于增量容量。
- 显式韧性只保障给定场景，不能外推为对未知故障的普遍保证。
- 维修作业的时间、人员和日历属于 [Gantt](../gantt/)，不是 Network 自动决定的。

## 学习路线

1. 完成[环境准备](../setup)。
2. 跟随[第一次网络流优化](./first-flow)，先检查守恒、费用与容量。
3. 增加共享资源、容量等级、建设预算和一个故障场景，每次只改一类条件。
4. 需要车辆路线时，继续到 [VRPTW 综合案例](/zh-cn/examples/framework-example5)。

## 接口参考

- [Kotlin 网络领域与独立入口](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-network-scheduling/README_ch.md)
- [Rust 网络领域与证明契约](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-network-scheduling/README_ch.md)

[返回场景选型](../)

## 继续实践

下一课：[共享资源、缺货与联合模型](./business-scenario)。将场景迁入自己的应用前，完成[业务接入检查](../integration)。
