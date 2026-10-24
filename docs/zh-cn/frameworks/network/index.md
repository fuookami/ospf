# 网络优化（Network）：适用问题与学习路线

在有向网络中满足供需，遵守运输或处理容量，并最小化费用。之后可以增加网络建设、资源路径、网络结构与故障场景。节点表示供给点、需求点或中转点；连接两个节点的有向服务称为“弧”。车辆路径只是 Network 的一种应用。

## 最少需要理解的对象

| 业务概念 | API | 角色 | 核心字段 |
| --- | --- | --- | --- |
| 拓扑 | Node / Arc | 输入 | 稳定身份、端点和平行服务 |
| 商品与供需 | Commodity / Balance | 输入 | 各商品的节点供需与计量单位 |
| 容量和费用 | Capacity / Cost | 输入 | 弧及共享资源的上限、单价 |
| 流问题 | `FlowProblem` | 输入 | 拓扑、商品、费用与资源配置 |
| 注册与提取 | `FlowContext` | 调用 / 校验入口 | 守恒、容量、费用表达式和流量结果 |
| 流量结果 | `FlowSolution` | 输出 | 商品-弧流量、资源占用、未使用供应、缺货与费用 |

整数步长表示计量粒度，不表示整单不可拆。不同商品共同占用资源时，需要明确物理量换算，不能把吨和件直接相加。

::: tip 商品可用弧的空值
`allowedArcs = null` / `allowed_arcs = None` 表示允许全部服务弧，空集合则不允许任何服务弧。供需与共享容量均保留单位；整数步长只定义计量粒度。
:::

## 从哪个入口调用？

::: info Kotlin 与 Rust 的入口差异
独立流入口为 Kotlin `NetworkFlowApplicationService`、Rust `FlowApplicationService`。嵌入其他模型时使用 `FlowContext` 注册，不要再启动一个不共享变量的独立优化。
:::

先学习流与容量，再扩展网络建设和故障场景下的供需保障。带时间窗的车辆路径（VRPTW）及其列生成算法属于进阶内容，算法名称见[术语说明](../#terminology)。

纯流与共享容量直接使用独立流服务；增加联合决策时改为共同模型注册，而不是切换到不存在的 `NetworkColumnGeneration`。车辆路线列生成属于另一个进阶流程，当前流问题的后端预算与列数无关。见[调用代码](./business-scenario#entry-budget)。

## 能力边界

- 平行弧需要独立身份，不能只按起点/终点去重。
- 连通性不证明运输容量充足；建成总容量不等于增量容量。
- 故障模型只检查明确列出的场景，不能由此保证任意未知故障下仍可满足需求。
- 维修作业的时间、人员和日历属于 [Gantt](../gantt/)，不是 Network 自动决定的。

## 学习路线

1. 完成[环境准备](../setup)。
2. 跟随[第一次网络流优化](./first-flow)，先检查守恒、费用与容量。
3. 增加共享资源、容量等级、建设预算和一个故障场景，每次只改一类条件。
4. 需要车辆路线时，继续到 [VRPTW 综合案例](/zh-cn/examples/framework-example5)。

## 接口参考

- [Kotlin 网络领域与独立入口](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-network-scheduling/README_ch.md)
- [Rust 网络领域与求解结果说明](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-network-scheduling/README_ch.md)

[返回场景选型](../)

## 继续实践

下一课：[第一次网络流优化](./first-flow)。完成后再学[共享资源、缺货与联合模型](./business-scenario)。将场景迁入自己的应用前，完成[业务接入检查](../integration)。
