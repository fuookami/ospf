# 二维套裁 CSP2D

把定尺板材或卷材裁成矩形产品，同时满足订单和设备工艺。第一课先学定尺板与 Freeform，再比较卷材、两级和三级断头台工艺。

## 最少需要理解的对象

| 业务概念 | API | 输入或输出 |
| --- | --- | --- |
| 供料 | `Material2` | 固定宽高为板材，无固定高度为卷材 |
| 产品与需求 | `Product2` / `ProductDemand2` | 尺寸、件数等需求模式 |
| 图案 | `Pattern2` | 坐标、覆盖贡献与可选切分树 |
| 方案 | `CuttingPlan2` | 原料消耗与图案使用 |
| 问题 | `Csp2dProblem` | 物料、产品、需求、刀缝、工艺和配置 |

Freeform 要求几何合法；两级/三级断头台还需要合法切分树。板材的张数和卷材的消耗长度是不同目标口径。

## 从哪个入口调用？

Kotlin 使用 `Csp2dMilp` / `Csp2dColumnGeneration`。Rust 使用 `Csp2dSolvers`，默认带内置启发式，可注入 `Csp2dMasterSolver`。接口名称带 MILP，不代表未注入后端的 Rust 默认路径真的调用了原生 MILP。

两种语言的当前实现与配置并不完全对称；从各自可执行用例出发，不把对照代码误当作能力一致性承诺。

## 能力边界

- 当前面向矩形产品，不是任意多边形异形套料。
- 坐标合法不等于工艺合法，断头台方案需要通过切分树校验。
- 残余需求驱动或估计价格筛选不构成真实 LP 对偶定价证书。
- 默认启发式得到覆盖方案，不等于完整问题已证明最优。

## 学习路线

1. 完成[环境准备](../setup)。
2. 跟随[第一次板材套裁](./first-cutting)，从产品件数与方案使用量开始。
3. 增加刀缝，再切换卷材或断头台工艺，分别检查消耗与可制造性。
4. 如果只需要已有物品的几何装载，比较 [BPP2D](../bpp2d/)。

## 接口参考

- [Kotlin 模块与工艺说明](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-csp2d/README_ch.md)
- [Rust API、主问题与定价边界](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-csp2d/README_ch.md)

Kotlin 模块 README 的历史阶段描述可能落后于应用源码；本教程展示当前公开 API 的业务示例，源码契约另见验证参考，不以阶段标签推断能力。

[返回场景选型](../)

## 继续实践

下一课：[刀缝、工艺与卷材](./business-scenario)。将场景迁入自己的应用前，完成[业务接入检查](../integration)。
