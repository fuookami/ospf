# 一维装箱 BPP1D

已有一批长条物品，要放进有限长度的货槽。每种货槽有长度、承重、成本和可用库存；相邻物品可能需要间距。BPP1D 帮你选择装载组合，并生成可复核的放置结果。

## 最少需要理解的对象

| 业务概念 | API | 输入或输出 |
| --- | --- | --- |
| 物品 | `Item` | 稳定 ID、长度、重量、精确件数 |
| 容器类型 | `BinType` | 长度、承重、成本、可选库存 |
| 问题 | `PackingProblem` | 物品、容器、间距、目标 |
| 装载组合 | `PackingPattern` | 一个容器中各物品的件数 |
| 最终结果 | `PackingResult` | 放置、空闲区间、物料和 KPI |

间距只存在于相邻物品之间，不额外收取尾间距。物理量保留单位，不能直接把 `40 cm` 和裸数 `1` 比较。

## 从哪个入口调用？

Kotlin 的 `Bpp1dMilp` / `Bpp1dColumnGeneration` 注入 `ColumnGenerationSolver` 和数值适配器。Rust 同名应用入口通过 callback 接收实际 solver 模型；构造和错误处理不应逐行对译。

你负责数据、目标和后端，框架负责组合生成、模型注册、选择与装载复核。先使用普通 MILP，再观察列生成流程。

## 能力边界

- 这是已有物品的装载模型。需要原料加工、产品产出和超欠产时，比较 [CSP1D](../csp1d/)。
- 候选枚举可能受节点数和候选数限制；`complete` 是需要读取的结果，不是默认假定。
- LP 定价结束不等于整数最优；有限候选池内最优也不自动等于全局最优。
- 尾箱重排必须复核完整方案，不能把只适用于全局的业务规则直接当成局部子问题规则。

## 学习路线

1. 完成[环境准备](../setup)。
2. 跟随[第一次一维装箱](./first-packing)，学习输入、求解、放置和数量复核。
3. 在同一场景里增加间距、库存、多容器类型和成本，再阅读扩展契约。
4. 需要自己建模时，参考[建模流程](/zh-cn/guide/modeling-workflow)，不要在应用服务里复制已有领域公式。

## 接口参考

- [Kotlin API 与扩展契约](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp1d/README_ch.md)
- [Rust API 与扩展契约](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp1d/README_ch.md)
- [理解求解结果](/zh-cn/guide/solving-results)

[返回场景选型](../)

## 继续实践

下一课：[多箱型、库存与成本](./business-scenario)。将场景迁入自己的应用前，完成[业务接入检查](../integration)。
