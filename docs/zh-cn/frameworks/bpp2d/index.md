# 二维装箱 BPP2D

把已有矩形零件排到板面上，检查旋转、间距、越界和重叠，再比较板材消耗与余料。二维装箱不仅是画坐标，还需要需求覆盖、板型选择和最终校验。

## 最少需要理解的对象

| 业务概念 | API | 输入或输出 |
| --- | --- | --- |
| 物品 | `ActualItem` | 形状、重量和旋转许可 |
| 板材类型 | `SheetType` | 宽高、承重、类型身份 |
| 候选排布 | `Layer` | 携带板材类型的放置集合 |
| 生成请求 | `Bpp2dLayerGenerationRequest` | 物品、板型、间距与候选限制 |
| 应用服务 | `ColumnGenerationApplicationService` | 生成、分配、装箱与 KPI / 渲染 |

矩形支持可选 90° 旋转；圆形与混合形状属于后续专题。内部相交才算重叠，相切不算重叠。实际面积与外接面积应分别阅读。

## 从哪个入口调用？

两种语言都有 `ColumnGenerationApplicationService`，但 Kotlin 的 solver 注入与 Rust 的 RMP/final executor 接口不同。Rust 不传求解器的退化执行不能解释为完成最终整数优化。

先理解候选层和终态几何校验，再接通受限主问题、定价和最终 MILP。框架负责这些领域步骤，业务调用方不应另写一套重叠约束。

## 能力边界

- 几何上放得下，不等于满足断头台裁切工艺。生产需求、切分树与卷材消耗见 [CSP2D](../csp2d/)。
- 圆形面积为实际圆面积，不能用外接矩形面积冒充装载率。
- 面积消耗、板数和板型成本是不同目标；加权和不是天然的严格词典序优化。
- 仅有候选生成或 mock 执行，不构成原生求解或完整问题最优性证据。

## 学习路线

1. 完成[环境准备](../setup)。
2. 跟随[第一次二维排样](./first-layout)，从矩形和单板型开始。
3. 修改旋转与间距，再引入多板型与成本。新生成策略应接生成器接口，而不是复制求解循环。
4. 添加业务规则前，明确它约束单个候选还是多个候选的组合选择。

## 接口参考

- [Kotlin API 与形状支持矩阵](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp2d/README_ch.md)
- [Rust API 与执行器契约](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp2d/README_ch.md)

[返回场景选型](../)

## 继续实践

下一课：[多板型、旋转与间距](./business-scenario)。将场景迁入自己的应用前，完成[业务接入检查](../integration)。
