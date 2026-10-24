# 一维装箱（BPP1D）：适用问题与学习路线

已有一批长条物品，要放进有限长度的容器，例如型材定尺装载。每种容器类型有长度、承重、费用和可用库存；相邻物品可能需要间距。BPP1D 帮你选择装载组合，并生成可复核的放置结果。本模块统一将 `Bin` 称为“容器”、`BinType` 称为“容器类型”。

## 最少需要理解的对象

| 业务概念 | API | 角色 | 核心字段 |
| --- | --- | --- | --- |
| 物品 | `Item` | 输入 | `id`、`length`、`weight`、精确件数 `amount` |
| 容器类型 | `BinType` | 输入 | 长度、承重、`cost`、可选 `inventory` |
| 问题 | `PackingProblem` | 输入 | 物品、容器类型、`spacing`、`objective` |
| 装载组合 | `PackingPattern` | 中间概念 | 容器类型 ID、每种物品的件数；尚未展开坐标 |
| 求解结果 | `Bpp1dSolution` | 输出 | 状态/报告、候选完整性、列数、`packing` |
| 放置结果 | `PackingResult` | 输出 | `plans`、空闲/尾部余量区间、物品汇总 |
| 指标 | `PackingKpi` | 输出 | 容器数、件数、消耗容器长度、尾部余量长度、利用率、费用 |

间距（`spacing`）仅存在于同一个容器的相邻物品之间，容器首尾不额外计入间距，即装入 $n$ 件物品产生 $n-1$ 个间距。

::: tip 单位与库存
长度和重量保留物理单位。Kotlin 通过适配器转换兼容单位；Rust 本课在输入时统一为 m/kg。`inventory = null`（Kotlin）/ `None`（Rust）表示不限库存，`0` / `Some(0)` 表示不能使用。
:::

### 从返回值定位业务字段

| 想读取什么 | Kotlin | Rust |
| --- | --- | --- |
| 候选是否枚举完整 | `solution.candidateComplete` | `solution.candidate_complete` |
| 启用的容器 | `solution.packing.plans` | `solution.packing.as_ref()` 后读 `plans` |
| 容器及容器类型 | `plan.bin.id`、`plan.bin.type` | `plan.bin_id`、`plan.bin_type_id`，按 ID 查 `context.problem().bin_types` |
| 物品区间 | `placement.itemId`、`start`、`start + length` | `placement.item_id`、`start`、`length` |
| 尾部余量 / 空闲区间 | `packing.remnants`（尾部余量）；相邻放置可推导间隙 | `packing.free_intervals`（含内部间隙和尾部余量） |
| 核心 KPI | `packing.kpi(Meter)` | `context.kpi(&packing)` |

::: info 两种语言的结果边界
Kotlin 返回成功结果后直接读取 `packing`；Rust 的 `packing` 是 `Option`，没有找到可行解时先读取 `report`。两者的 KPI `remnantLength` / `remnant_length` 都只统计尾部余量，不包含物品间距。生成器的 `complete` 对应求解结果中的候选完整性字段，**不代表已证明整数最优**。
:::

## 从哪个入口调用？

| 入口 | 适用场景 | 如何判断是否合适 |
| --- | --- | --- |
| `Bpp1dMilp` | 首次接入、小型实例，或生成后的候选集合可以直接交给整数主问题 | 先用它建立正确性和耗时的对照基准，观察候选完整性与列数 |
| `Bpp1dColumnGeneration` | 需要观察 LP 定价过程，或比较分轮激活候选列的表现 | 在同一数据、生成预算和后端下比较耗时、列数和报告 |

当前两个入口都会先生成**有界的有限候选集合**；列生成从这个集合中搜索并选择可改进目标的候选，最终整数阶段使用生成列与初始候选列的合并集合。它不是绕过枚举的无限规模入口，因此没有通用的“超过多少件就切换”阈值。先用代表性数据测量，再选择入口。

Kotlin 向构造函数注入 `ColumnGenerationSolver` 和数值适配器；Rust 向 `solve` 注入求解器回调。LP、MILP 和定价的含义见[术语说明](../#terminology)；切换方式与节点数、候选列数、每轮列数的配置见[第二课的列生成示例](./business-scenario#column-generation)。

## 能力边界

- 这是已有物品的装载模型。需要原料加工、产品产出和超欠产时，比较 [CSP1D](../csp1d/)。
- 候选枚举可能受节点数和候选数限制；`complete` 是需要读取的结果，不是默认假定。
- LP 定价结束不等于整数最优；有限候选集合内最优也不自动等于全局最优。
- 最后几个容器的重新装载必须复核完整方案，不能把只适用于全局的业务规则直接当成局部子问题规则。

## 学习路线

1. 完成[环境准备](../setup)。
2. 跟随[第一次一维装箱](./first-packing)，学习输入、求解、放置和数量复核。
3. 在同一场景里增加间距、库存、多容器类型和费用，再阅读扩展接口。
4. 需要自己建模时，参考[建模流程](/zh-cn/guide/modeling-workflow)，不要在应用服务里复制已有领域公式。

## 接口参考

- [Kotlin API 与扩展接口](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp1d/README_ch.md)
- [Rust API 与扩展接口](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp1d/README_ch.md)
- [理解求解结果](/zh-cn/guide/solving-results)

[返回场景选型](../)

## 继续实践

下一课：[第一次一维装箱：物品、间距与库存](./first-packing)。完成后再学[多容器类型、库存与费用](./business-scenario)，并按[业务接入检查](../integration)迁入自己的应用。
