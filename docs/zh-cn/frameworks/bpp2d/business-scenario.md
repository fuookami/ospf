# 多板型、旋转与间距：不要只比较面积

完成[第一次二维排样](./first-layout)后，用一个四件问题区分几何可行性、板材消耗和费用。以下用新输入替换第一课的物品与板型。

## 1. 两种板材与一个矩形零件

| 对象 | 尺寸 / 数量 | 单价 |
| --- | --- | ---: |
| R | 2 × 2 m，共 4 件 | — |
| S | 4 × 4 m，供给至少 1 张 | 5 |
| L | 5 × 5 m，供给至少 1 张 | 3 |

承重充分，先禁止旋转、间距为零。S 可按 2×2 网格恰好放下四件，L 也能放下。以面积消耗为唯一目标会偏好 S（16 m²），以采购费用为唯一目标会偏好 L（3）。最少板数无法区分它们。

然后设相邻件间距为 0.2 m：每行两件需要 `2 + 0.2 + 2 = 4.2 m`，S 不再容纳这个 2×2 排布，L 仍能容纳。不能因为需求总面积仍为 16 m² 就断言 S 可行；对于四个轴对齐的 2×2 正方形，4×4 板也没有其他带正间距的四件排布。

## 2. 把所有板型传入生成请求

### 构造本课的请求

`makeRequest` / `make_input` 创建 R×4 和 S、L 板型，并显式绑定米；Kotlin 另将承重绑定千克。`spacing` 传给 `kerf`，改为 0.2 后重新调用。

::: code-group

<<< ../../../frameworks/_examples/bpp2d-business.kt#input [Kotlin]

<<< ../../../frameworks/_examples/bpp2d-business.rs#input [Rust]

:::

::: warning Rust 单位边界
Rust 当前应用接口的重量与长度共享同一单位类型，故本例只演示几何：重量和承重均为 0，不代表 0 kg 已获得类型安全。真实承重业务需在适配层补齐量纲归一化与独立校验。
:::

### 把费用写入目标配置


::: code-group

<<< ../../../frameworks/_examples/bpp2d-business.kt#objective [Kotlin]

<<< ../../../frameworks/_examples/bpp2d-business.rs#objective [Rust]

:::

Kotlin 的配置随 `request.executorConfig` 进入标准执行器。Rust 的 `cost_config()` 交给调用方实现的标准执行器适配器；`run_business` 接收这些已配置好的受限主问题与最终整数求解适配器，而不在内部创建后端。

| 责任 | Kotlin | Rust |
| --- | --- | --- |
| 多板型请求 | `Bpp2dLayerGenerationRequest.sheets` | `Bpp2dLayerGenerationRequest` 的板型集合 |
| 候选身份 | `Layer.sheet` 保留类型 | 每个候选保留实际板型 |
| 间距 | 生成请求的 `kerf` | 请求的 `with_kerf(...)` |
| 目标 | 面积、数量、按板型费用的建模步骤 | 对应面积、数量、板型费用建模步骤 |

`sheet` 的默认/主板型不替代 `sheets` 的完整集合。同一物品坐标放到 S、L 上是两种列，不能只按坐标去重。

沿用第一课的物品构造方式，构造 S、L 两种 `SheetType` 并一起传入请求。费用目标还需进入主问题配置，不能仅在输出表中附加单价。

## 3. 单独验证间距路径

改变间距后重新生成候选并求解，再逐件计算实际边界间距；只看到候选数量下降，不能证明所有放置间距都合法。

### 入口与生成预算 {#entry-budget}

本服务已经是列生成入口，使用每轮列数与迭代上限控制搜索。它们不是候选集合完整枚举的证明，也不是后端节点数上限。

::: code-group

<<< ../../../frameworks/_examples/bpp2d-business.kt#entry [Kotlin]

<<< ../../../frameworks/_examples/bpp2d-business.rs#entry [Rust]

:::

## 4. 增加审批规则，再做旋转对照

### 最小扩展：只允许已审批板型

假设 L 暂未通过采购审批，用生成器包装层过滤 L。Kotlin 把包装器传给 `algorithmFactory`；初始候选和后续生成使用同一个包装器，避免历史/初始候选列绕过规则。

::: code-group

<<< ../../../frameworks/_examples/bpp2d-business.kt#rule [Kotlin]

<<< ../../../frameworks/_examples/bpp2d-business.rs#rule [Rust]

:::

接入位置：

::: code-group

<<< ../../../frameworks/_examples/bpp2d-business.kt#injection [Kotlin]

<<< ../../../frameworks/_examples/bpp2d-business.rs#injection [Rust]

:::

Rust 在较低层的 `ColumnGenerationAlgorithm` 注入生成器，再显式调用装箱结果分析器；最终后端仍需按板型维持列与实物的对应关系。这里的单个候选方案审批规则不替代跨板材采购总额约束。

把问题单独改为一张 `3 × 2 m` 板和一件 `2 × 3 m` 零件，间距为零。允许 90° 旋转时可放，禁止时不可放。这样可以隔离测试 `allowRotate`，避免使用正方形却误以为自己验证了旋转。

## 5. 加入业务后怎样验收？

- 零间距面积目标选 S，费用目标选 L；若采用加权和，按配置重算总分，不能继续要求两个单目标结论。
- 间距改变后重新生成候选、求解和复核；只在图上挪坐标不算接入规则。
- 多板型库存/采购限制只有真正进入主问题才有效；不要把输入目录中的板型数量当作已建模库存。
- 净物品面积、板材消耗、空闲区域和可回用余料分开报告；可回用还需要最小宽高等业务标准。

需要断头台裁切树时继续 [CSP2D 工艺场景](../csp2d/business-scenario)，不是给 BPP2D 排布改一个名字。将结果接入业务前完成[公共接入检查](../integration)。

源码回归命令统一放在[仓库验证参考](../verification)；先完成业务输入、调用与结果复核，再按需验证。


## 完整应用源码

片段来自同一个源码文件；保留导入声明和辅助函数后，从应用调用 `runBusiness` / `run_business`。构造函数也可独立用于输入校验。后端依赖沿用第一课；这些入口不创建隐式求解后端。

:::: details 展开完整 Kotlin / Rust 源码

::: code-group

<<< ../../../frameworks/_examples/bpp2d-business.kt [Kotlin]

<<< ../../../frameworks/_examples/bpp2d-business.rs [Rust]

:::

::::
