# 多箱型与目标配置：先说明“装得好”是什么意思

在[第一次三维装箱](./first-loading)接通最终选择和装载分析后，再加入第二种箱型。这个新练习只用正方体，避免让复杂朝向掩盖目标配置问题。

## 1. 一个能区分目标的场景

货物为 `2 × 2 × 2 m` 正方体，共 10 件，净体积 80 m³。承重充分，不加额外间距、支撑或平衡规则：

| 箱型 | 内尺寸 | 体积 | 可用箱数 | 单箱费用 |
| --- | --- | ---: | ---: | ---: |
| S | 4 × 4 × 4 m | 64 m³ | 2 | 4 |
| L | 4 × 4 × 6 m | 96 m³ | 1 | 10 |

一箱 S 最多放 8 件；一箱 L 可按 2×2×3 网格放 12 件，因此可以选择一箱 L，或两箱 S。体积消耗分别为 96、128 m³，费用分别为 10、8。最少箱数与最少体积消耗偏好 L，最小费用偏好两个 S。

下面把这组数据落到请求和目标配置中。手算用于比对实际启用箱体，最终是否可交付还取决于执行器产生的坐标与完整需求覆盖。

## 2. 多箱型有三处要同时接通

### 从手算数据构造请求

Kotlin 复用第一课的 `makeItem(edge = 2.0)`，创建 S、S、L 三个箱体实例，并按网格顺序放入 1 件、2 件直至达到容量上限，生成带实际坐标的候选层；完整函数见页末。Rust 每件货物使用单独记录，从 CSV 加载并转换得到米制几何、箱体与初始候选；`amount` 记录与最终实际放置件数都需保留。

::: code-group

<<< ../../../frameworks/_examples/bpp3d-business.kt#input [Kotlin]

<<< ../../../frameworks/_examples/bpp3d-business.rs#input [Rust]

:::

::: info 两种输入路径
Kotlin 候选包含实际放置，Rust CSV 的 `layers` 记录只给出初始候选层元数据，实际几何还取决于执行器生成与重建；不要将 `layers` 记录数当作可交付装箱。Rust 当前米制接口还共享重量/长度单位类型，生产接入需单独校验重量归一化。
:::

1. 生成请求携带全部箱型：Kotlin `Bpp3dLayerGenerationRequest.bins`；Rust `LayerGenerationRequest.bins`。
2. 每列保留来源箱型，不能把同坐标、不同箱型的层合并。
3. 最终模型有真实可用箱体：Kotlin `finalBins`；Rust 对应最终整数求解执行器的箱体输入。只增加箱型目录不等于可用箱体数量不限，数组下标也不应成为跨请求 ID。

然后对同一货物 ID 设置精确 10 件需求，按第一课的请求构造函数同步维护需求表示、初始候选列和最终箱体。不要把一箱 S 的 8 件容量写成订单需求。

## 3. 显式配置目标，而不是猜默认值

| 意义 | Kotlin 配置字段 | Rust 配置字段 |
| --- | --- | --- |
| 体积消耗权重 | `finalBinVolumeWeight` | `final_bin_volume_weight` |
| 箱数系数 | `finalBinAmountCoefficient` | `final_bin_amount_coefficient` |
| 箱型费用权重 | `finalBinTypeCostWeight` | `final_bin_type_cost_weight` |
| 箱型费用表 | `finalBinTypeCosts` | `final_bin_type_costs` |

配置定义见 [Kotlin 标准执行器](https://github.com/fuookami/ospf-kotlin/blob/main/ospf-kotlin-framework-bpp3d/bpp3d-application/src/main/fuookami/ospf/kotlin/framework/bpp3d/application/service/ColumnGenerationStandardExecutors.kt)和 [Rust 执行器配置](https://github.com/fuookami/ospf-rust/blob/main/ospf-rust-framework-bpp3d/src/application/service/executor/config.rs)。当前默认体积权重为 1、箱数系数为 `1e-6`、费用权重为 0；只填费用表不会让费用自动主导结果。

先分别做纯目标实验：只保留体积权重，或只保留费用权重。再把箱数系数设零，研究归一化后的 `score = consumed_volume + λ × cost`：L 得分 `96 + 10λ`，两个 S 得分 `128 + 8λ`；λ=16 打平，超过 16 后费用偏好反转。这里体积数值统一按 m³ 取值，权重需要有明确换算规则。

加权求和不自动保证“先最小体积、后最小费用”。若业务要求体积始终优先，可评估分阶段求解：先求最小体积，固定其最优值后再求最小费用。若使用权重，则需依据目标取值范围证明其优先级；默认 `1e-6` 不对所有数据规模提供这一保证。

### 配置纯费用目标

::: code-group

<<< ../../../frameworks/_examples/bpp3d-business.kt#objective [Kotlin]

<<< ../../../frameworks/_examples/bpp3d-business.rs#objective [Rust]

:::

### 入口与生成预算 {#entry-budget}

Kotlin 在请求的 `cgConfig` 配置；Rust 在服务的 `ColumnGenerationConfig` 配置，还提供总列数上限。继续使用真实受限主问题与最终整数求解后端。

::: code-group

<<< ../../../frameworks/_examples/bpp3d-business.kt#entry [Kotlin]

<<< ../../../frameworks/_examples/bpp3d-business.rs#entry [Rust]

:::

## 4. 验收最终方案，而不是候选体积

### 最小规则：本单只允许 S 箱

Kotlin 的 `smallOnly` 同时收紧箱体实例、初始候选列和 `generators`，下面展示生成器接口方法与请求注入。规则在优化前生效，最后仍逐箱复核。

::: code-group

<<< ../../../frameworks/_examples/bpp3d-business.kt#rule [Kotlin]

<<< ../../../frameworks/_examples/bpp3d-business.rs#rule [Rust]

:::

Rust 在求解前过滤可用箱型与初始候选层，再用公开的最终几何校验接口进行结果复核：

<<< ../../../frameworks/_examples/bpp3d-business.rs#injection [Rust]

::: warning 生成规则与结果校验器的区别
`PackingGeometryContract` 的自定义检查在装箱分析时执行；它只拒绝交付，不会自动把规则加入受限主问题或定价。因此完整源码在求解前也过滤了箱体和初始候选。新增混装、跨层或总预算规则时，应在对应生成/建模路径约束，最后再用校验器复核。
:::

- 恰好装入 10 件，不允许同一件货物在多个选中层中重复计数。
- 所有实际放置在箱内，不重叠，朝向、承重和启用的支撑规则合法。
- 按实际使用箱体重算箱数、96/128 m³ 消耗和费用；余量分别为 16/48 m³。
- 受限主问题与最终整数求解阶段均使用正确扩展及后端；`finalSolved` 或对应诊断、实际装箱结果和实际放置共同检查。

目标系数与实际求解范围分开记录；系数配置正确不代表候选完备，也不代表新场景已被证明全局最优。

若最小箱体单件也装不下，先查方向和候选；若候选有了但最终无方案，再查可用箱体、需求和跨层几何。按[接入指南](../integration)保存原始诊断，不要仅通过增加迭代次数排错。

源码回归命令统一放在[仓库验证参考](../verification)；先完成业务输入、调用与结果复核，再按需验证。


## 完整应用源码

片段来自同一个源码文件；保留导入声明和辅助函数后，从应用调用 `runBusiness` / `run_business`。构造函数也可独立用于输入校验。后端依赖沿用第一课；这些入口不创建隐式求解后端。

:::: details 展开完整 Kotlin / Rust 源码

::: code-group

<<< ../../../frameworks/_examples/bpp3d-business.kt [Kotlin]

<<< ../../../frameworks/_examples/bpp3d-business.rs [Rust]

:::

::::
