# 二维装箱（BPP2D）：适用问题与学习路线

把已有矩形零件排到板面上，检查旋转、间距、越界和重叠，再比较板材消耗与余料。二维装箱不仅是画坐标，还需要需求覆盖、板型选择和最终校验。

## 最少需要理解的对象

| 业务概念 | API | 角色 | 核心字段 |
| --- | --- | --- | --- |
| 物品 | `ActualItem` | 输入 | 形状、重量和旋转许可 |
| 板型 | `SheetType` | 输入 | 宽高、承重、类型 ID |
| 候选排布 | `Layer` | 中间概念 | 携带板型的放置集合 |
| 生成请求 | `Bpp2dLayerGenerationRequest` | 输入 | 物品、板型、间距与候选限制 |
| 应用服务 | `ColumnGenerationApplicationService` | 调用 / 校验入口 | 生成、分配、装箱与 KPI / 渲染 |
| 装箱结果与指标 | `PackingResult` / `response.packing` | 输出 | 板材、放置、空闲矩形区域、面积与终止诊断 |

矩形支持可选 90° 旋转；圆形与混合形状属于后续专题。内部相交才算重叠，相切不算重叠。实际面积与外接面积应分别阅读。

## 从哪个入口调用？

::: info Kotlin 与 Rust 的入口差异
两种语言都有 `ColumnGenerationApplicationService`。Kotlin 接收求解器；Rust 需要分别提供受限主问题和最终整数求解的执行器。Rust 未配置这两个执行器时，不会完成对应的整数优化。
:::

先理解候选排布和最终几何校验，再配置受限主问题、候选搜索和最终整数求解。受限主问题只考虑当前已生成的候选；列生成等算法名称见[术语说明](../#terminology)。几何约束由框架提供，应用应复用这些规则。

这里的应用入口本身就是列生成流程，没有名为 `Bpp2dMilp` 的对等切换入口。小实例先固定板型并限制候选；多板型时逐步扩大每轮候选与迭代预算。参数代码见[第二课](./business-scenario#entry-budget)。

## 能力边界

- 几何上放得下，不等于满足断头台切割工艺。生产需求、切分树与卷材消耗见 [CSP2D](../csp2d/)。
- 圆形面积为实际圆面积，不能用外接矩形面积冒充装载率。
- 面积消耗、板数和板型费用是不同目标；加权求和不自动保证“先优化第一目标，再优化第二目标”的严格优先级。
- 仅有候选生成或模拟执行，不构成原生求解或完整问题最优性证据。

## 学习路线

1. 完成[环境准备](../setup)。
2. 跟随[第一次二维排样](./first-layout)，从矩形和单板型开始。
3. 修改旋转与间距，再引入多板型与费用。新生成策略应接生成器接口，而不是复制求解循环。
4. 添加业务规则前，明确它约束单个候选还是多个候选的组合选择。

## 接口参考

- [Kotlin API 与形状支持矩阵](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp2d/README_ch.md)
- [Rust API 与执行器接口说明](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp2d/README_ch.md)

[返回场景选型](../)

## 继续实践

下一课：[第一次二维排样](./first-layout)。完成后再学[多板型、旋转与间距](./business-scenario)。将场景迁入自己的应用前，完成[业务接入检查](../integration)。
