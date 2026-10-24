# 二维套裁（CSP2D）：适用问题与学习路线

把定尺板材或卷材裁成矩形产品，同时满足订单和设备工艺。第一课先学定尺板与自由排样，再比较卷材、两级和三级断头台切割工艺。

## 最少需要理解的对象

| 业务概念 | API | 角色 | 核心字段 |
| --- | --- | --- | --- |
| 供料 | `Material2` | 输入 | 固定宽高为板材，无固定高度为卷材 |
| 产品与需求 | `Product2` / `ProductDemand2` | 输入 | 尺寸、件数等需求模式 |
| 切割图案 | `Pattern2` | 中间概念 | 坐标、产品产出与可选切分树 |
| 切割方案 | `CuttingPlan2` | 中间概念 | 原料消耗与图案使用 |
| 问题 | `Csp2dProblem` | 输入 | 原料、产品、需求、刀缝、工艺和配置 |
| 裁切结果 | `Csp2dSolution` | 输出 | 图案使用量、覆盖量、消耗长度、余料与终止信息 |

自由排样（`Freeform`）要求几何合法；断头台切割要求每一刀贯穿当前矩形区域，并用切分树记录切割过程，两级/三级工艺还限制切割阶段。板材的张数和卷材的消耗长度是不同的目标计量方式。

::: tip 空高度与工艺树
`Material2.height = null / None` 表示卷材，不是高度为 0 的板材；图案仍需实际段高。自由排样的空切分树是合法表达，断头台切割工艺则需有与放置一致的切分树。
:::

## 从哪个入口调用？

::: info Kotlin 与 Rust 的入口差异
Kotlin 使用 `Csp2dMilp` / `Csp2dColumnGeneration`。Rust 使用 `Csp2dSolvers`，默认带内置启发式，可注入 `Csp2dMasterSolver`。接口名称带 MILP，不代表未注入后端的 Rust 默认路径真的调用了原生 MILP。
:::

请按各语言示例选择入口和配置；LP、MILP 和列生成的含义见[术语说明](../#terminology)。

`Csp2dMilp` / `solve_milp` 适合先检查一次生成的候选图案集合；`Csp2dColumnGeneration` / `solve_column_generation` 按剩余需求追加图案。后者目前不使用能够证明线性松弛最优性的精确候选搜索算法，增大候选数和迭代次数上限也不自动证明全局最优；[第二课](./business-scenario#entry-budget)展示配置与切换。

## 能力边界

- 当前面向矩形产品，不是任意多边形异形套料。
- 坐标合法不等于工艺合法，断头台切割方案需要通过切分树校验。
- 按剩余需求或估计价格筛选候选，不能据此证明未生成的候选都无法改进目标。
- 默认启发式得到覆盖方案，不等于完整问题已证明最优。

## 学习路线

1. 完成[环境准备](../setup)。
2. 跟随[第一次板材套裁](./first-cutting)，从产品件数与方案使用量开始。
3. 增加刀缝，再切换卷材或断头台切割工艺，分别检查消耗与可制造性。
4. 如果只需要已有物品的几何装载，比较 [BPP2D](../bpp2d/)。

## 接口参考

- [Kotlin 模块与工艺说明](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-csp2d/README_ch.md)
- [Rust API、主问题与定价边界](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-csp2d/README_ch.md)

Kotlin 模块 README 的历史阶段描述可能落后于应用源码；本教程展示当前公开 API 的业务示例，源码行为另见验证参考，不以阶段标签推断能力。

[返回场景选型](../)

## 继续实践

下一课：[第一次板材套裁](./first-cutting)。完成后再学[刀缝、工艺与卷材](./business-scenario)。将场景迁入自己的应用前，完成[业务接入检查](../integration)。
