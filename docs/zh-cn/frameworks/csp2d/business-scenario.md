# 刀缝、工艺与卷材：能放下不等于能加工

完成[第一次板材套裁](./first-cutting)后，按“刀缝 → 切分树 → 卷材”逐步改变问题。每次重新生成图案，而不是只改最终报表里的工艺名称。

## 1. 先用一刀理解刀缝

把 `10 × 6 m` 原料在 x=4 m 竖切，刀缝为 0.5 m。正确结果为：左块宽 4 m，右块从 x=4.5 m 开始、宽 5.5 m；宽度守恒为 `4 + 0.5 + 5.5 = 10`。不能让左右两块继续合计 10 m，还声称扣掉了刀缝。

下面用两种新产品构造完整请求，并一起设置刀缝与工艺。

练习：两种产品分别为 `4 × 6 m`、`6 × 6 m`，各一件，禁止旋转。零刀缝时一张 10×6 板刚好切出；正刀缝时原排布不可行。重新生成并检查程序是否改用其他供料/更多板材或正确报告无可交付方案。

## 2. 从自由排样改为断头台切割

### 构造两产品、刀缝与工艺

下面完整构造 10×6 m 板材、a=4×6 m、b=6×6 m 各一件，显式禁止旋转。调用 `makeProblem(0.5)` / `make_problem(0.5)` 得到正刀缝场景，零刀缝则传 0.0。

::: code-group

<<< ../../../frameworks/_examples/csp2d-business.kt#input [Kotlin]

<<< ../../../frameworks/_examples/csp2d-business.rs#input [Rust]

:::

| 必须保留的信息 | 为什么 |
| --- | --- |
| 区域位置与宽高 | 每个子区域必须来自实际父区域 |
| 刀向、刀位和刀缝 | 决定子区域边界与被移除的材料 |
| 工艺级数 | 两级和三级不是任意深度切割的别名 |
| 产品叶节点与坐标 | 几何放置要与树的叶节点区域相符 |

不要只检查“树非空”：每个产品必须对应叶节点区域，每次切分必须守恒，树的最大级数不得超过工艺限制。沿用第一课配置的后端，保存本次工艺参数与求解报告。

换工艺后应同时更新生成配置、图案校验和验收。没有两级候选不证明任意工艺都不可加工；应报告当前工艺与候选搜索范围。

构造函数已将 `process` 设为 `GuillotineTwoStage`。改工艺时，在输入处同时更新工艺和刀缝，再重新生成图案。

## 3. 补齐候选并配置生成预算

### 候选不足时，补一个实际贯穿刀图案

当前 Rust 默认逐行排样生成器会在这个紧贴板高的正刀缝场景返回 `NoCandidatePlan`：它在每行产品的高度之外还预留刀缝，无法生成这两个完整板高产品。这不意味着“两张板分别加工 a、b”物理不可行。下面按本课特例直接构造一刀图案：刀在产品右缘，余料由 `Region2.cut` 推导；几何和切分树先通过框架校验，再进入主问题。

::: code-group

<<< ../../../frameworks/_examples/csp2d-business.kt#cut [Kotlin]

<<< ../../../frameworks/_examples/csp2d-business.rs#cut [Rust]

:::

第 4 节的 `solveRestricted(solver, kerf = 0.5)` / `solve_restricted(master_solver, 0.5)` 将这两个图案直接送入领域主问题；后端仍由应用提供。零刀缝的禁配练习传 0.0。此模板仅针对本课“成品与板材等高、右侧有正余料”的特例，不是通用套裁算法。

### 切换入口与生成预算 {#entry-budget}

下面针对未启用自定义禁配的刀缝场景，在同一输入上比较一次生成与逐轮追加。`maxCandidatesPerIteration` / `max_candidates_per_iteration` 限制候选数，迭代上限限制追加轮数。

::: code-group

<<< ../../../frameworks/_examples/csp2d-business.kt#entry [Kotlin]

<<< ../../../frameworks/_examples/csp2d-business.rs#entry [Rust]

:::

::: info Rust 实际求解方式
Rust 本例使用默认启发式，不调用原生优化器。CSP2D 支持注入 `Csp2dMasterSolver`，但高层实现可能改用启发式；如果业务不允许自动更换算法，可直接调用领域主问题，如本课的 `solve_restricted`。
:::

## 4. 接入禁配规则并明确损耗定义

### 最小扩展：同一图案只含一种产品

实现 `PatternGenerator2`，按产品分别调用默认工艺生成器，生成可替代的单产品图案，再排除混合产品图案。Kotlin 包装整个 `PatternGenerationContext2`（而非仅某个内部生成器），使复用的历史图案也经过规则。

::: code-group

<<< ../../../frameworks/_examples/csp2d-business.kt#rule [Kotlin]

<<< ../../../frameworks/_examples/csp2d-business.rs#rule [Rust]

:::

接入位置：

::: code-group

<<< ../../../frameworks/_examples/csp2d-business.kt#injection [Kotlin]

<<< ../../../frameworks/_examples/csp2d-business.rs#injection [Rust]

:::

::: info 高层入口的扩展边界
目前 `Csp2dMilp` / `Csp2dColumnGeneration` 和 Rust `Csp2dSolvers` 都未提供自定义生成器参数。上面的 `restrictedPlans` 只生成按业务规则筛选的候选；继续调用普通高层入口会重新生成图案，不会沿用这些规则。下面直接把按业务规则筛选的方案注册到领域主问题并调用后端，避免重新生成绕过禁配。此最小调用示例仅覆盖本课定尺、件数订单；设备产能、超产和卷材等还需注册各自的建模步骤。
:::

::: code-group

<<< ../../../frameworks/_examples/csp2d-business.kt#master [Kotlin]

<<< ../../../frameworks/_examples/csp2d-business.rs#master [Rust]

:::

### 卷材按消耗长度计量

卷材有固定幅宽，但每个切割方案还需实际段长。若一个图案消耗 4 m、执行 3 次，则总消耗为 12 m，不是“使用 1 个图案”，也不等于“只消耗 3 m”。

定尺模式先关注张数；卷材模式先关注消耗长度。若混合原料、不同幅宽或费用，必须显式选择统一目标，不能把“张”和“米”直接相加。

损耗至少区分产品净面积、消耗原料面积、刀缝损失和剩余区域。可回用余料需满足业务定义的最小宽高和工艺标准，不能将所有空白区域都计入可用库存。需要更详细的损耗分解时，应根据实际字段计算，或在应用层补充统计。

## 5. 新应用的验收清单

1. 产品产出乘使用次数满足订单，且没有超出配置允许的超产。
2. 原料使用不超库存，卷材段长乘使用次数与总长一致。
3. 实际坐标无越界、无内部重叠，刀缝在生成和验证中定义一致。
4. 切分树满足级数、刀向、子区域和叶子规则。
5. 记录实际使用的启发式算法或原生求解后端及其终止原因；没有追加图案，不等于已证明其他图案都不能改进目标。

完成后使用[接入指南](../integration)保留输入、工艺配置和失败诊断；只需要物品排布、不涉及加工产出时回看 [BPP2D](../bpp2d/business-scenario)。

源码回归命令统一放在[仓库验证参考](../verification)；先完成业务输入、调用与结果复核，再按需验证。


## 完整应用源码

片段来自同一个源码文件；保留导入声明和辅助函数后，从应用调用 `runBusiness` / `run_business`。构造函数也可独立用于输入校验。后端依赖沿用第一课；这些入口不创建隐式求解后端。

:::: details 展开完整 Kotlin / Rust 源码

::: code-group

<<< ../../../frameworks/_examples/csp2d-business.kt [Kotlin]

<<< ../../../frameworks/_examples/csp2d-business.rs [Rust]

:::

::::
