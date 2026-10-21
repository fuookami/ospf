# ospf-rust-framework-bpp2d

:us: [English](README.md) | :cn: 简体中文

## 简介

`ospf-rust-framework-bpp2d` 是 Kotlin `ospf-kotlin-framework-bpp2d` 的 Rust 迁移。它提供可复用的二维
装箱框架，支持矩形件与圆形件，覆盖列生成、刀缝、余料报告与多板材类型的加权目标。

二维几何与套裁框架共用 [`ospf-rust-framework-geometry2d`](../ospf-rust-framework-geometry2d/README_ch.md)，
因此装箱与套裁在面积口径和重叠语义上不会各自漂移。

## 作用范围

本 crate 拥有可复用的 BPP2D 内核：物品/板材/层模型、BLA 与块装载排样、候选层生成、层分配建模边界、
终态装箱与几何校验、余料报告，以及列生成应用流程。

明确非目标：

1. 业务请求 DTO、租户上下文、公式语言或部署策略。
2. 外部 renderer 实现。
3. solver 安装与许可管理；求解后端仍留在 `ospf-rust-core` / `ospf-rust-framework` 的 feature 门控之后。

## 模块结构

| Rust 模块 | Kotlin 边界 | 职责 |
| --- | --- | --- |
| [`src/infrastructure`](src/infrastructure) | `bpp2d-infrastructure` | 领域错误与渲染 DTO；再导出共享二维几何。 |
| [`src/domain/item`](src/domain/item) | `bpp2d-domain-item-context` | 物品、板材、层、物料、需求统计与标识类型。 |
| [`src/domain/bla`](src/domain/bla) | `bpp2d-domain-bla-context` | 底部向上左对齐排样；圆形件显式跳过。 |
| [`src/domain/block_loading`](src/domain/block_loading) | `bpp2d-domain-block-loading-context` | 空间模型、简单/复杂块、DFS 与多行切分、旋转守卫。 |
| [`src/domain/layer_generation`](src/domain/layer_generation) | `bpp2d-domain-layer-generation-context` | 逐板材类型生成候选层、圆形网格、图样、混合形状与历史回放。 |
| [`src/domain/layer_assignment`](src/domain/layer_assignment) | `bpp2d-domain-layer-assignment-context` | 建模边界：需求覆盖、检验数定价、分层目标与容量。 |
| [`src/domain/packing`](src/domain/packing) | `bpp2d-domain-packing-context` | 终态几何守卫、装箱器、余料分析与渲染适配。 |
| [`src/application`](src/application) | `bpp2d-application` | 列生成编排、标准执行器、尾板优化与 CSV 数据集。 |

## 架构概览

建模以 `MetaModel` 为轴心。RMP 与最终 MILP 共享同一套层分配上下文、需求覆盖口径与目标族。
应用层负责求解器选择、生成循环、trace/KPI 组装与装箱/渲染边界，自身不堆积变量与约束细节。

## 核心概念

1. **层**是一块板材上的排布；`SheetLayer` 组合同一块板材上的多个层。
2. **列身份**包含板材类型、来源、包围尺寸与放置集合：同一排布放在不同板材上是不同的列。
3. **需求覆盖**是必须满足的下界；若把需求只当上界，最小化目标会退化为「一块板材都不用」。
4. **检验数**为 `rc_j = area_j · c_area − Σ π_i · a_ij`，只有 `rc_j < −tolerance` 才算改进列。
5. **目标分层**：板材面积消耗为主目标（等价余料最少），板数权重极小仅用于平局裁决，类型成本默认 0。

## 公开 API

| API | 职责 | 稳定性 |
| --- | --- | --- |
| `LayerGenerationContext` | 逐板材类型的组合式候选生成。 | migration |
| `LayerAssignmentContext` | 列生成五阶段生命周期。 | migration |
| `ColumnGenerationAlgorithm` | RMP → 定价 → 加列 → 最终 MILP 编排。 | migration |
| `ColumnGenerationApplicationService` | 端到端入口。 | migration |
| `PackingGeometryGuard` / `Packer` / `FreeSpaceAnalyzer` | 终态几何校验、装箱与余料报告。 | migration |
| `TailSheetOptimizer` | 按剩余需求重选尾板列。 | migration |
| `CsvDatasetLoader` | `grouped-layer` 与 `material-width-amount` 数据集。 | migration |

## 建模扩展点

1. `Bpp2dLayerGenerator` —— 新增候选生成策略，无需改动循环。
2. `LayerAssignmentObjectiveConfig` —— 重配面积、板数与类型成本权重，或提供类型成本表。
3. `PackingRotationRule` —— 注入几何层无法知道的旋转许可。
4. `ColumnGenerationConfig` —— 迭代上限、每轮候选数、时间上限与检验数容差。

## 泛型数值边界

1. 领域层泛型于 `V` 并保留编译时单位 `U`。
2. `f64` 只出现在 `layer_assignment`（建模边界）与应用层。
3. 泛型代码不得调用 `to_f64()`；计数与缩放走 `count_fitting` 与 `scaled_quantity`。

## 物理量边界

宽高、坐标与面积使用 `Quantity<V, U>`；利用率与检验数是裸值。面积由 `quantity_multiply` 产生，
因此长度平方不会被算成立方。

## 求解生命周期

`register` → RMP 求解 → `add_columns` → 影子价格刷新 → 定价 → 最终 MILP → `extract_solution`。
退役以「从聚合移除并把被退役列交回应用层」表达。

## 输出

`ColumnGenerationResult` 携带列、板材、迭代统计、各轮 LP 目标、耗时、装箱结果与渲染 DTO。

## 使用方式

```toml
[dependencies]
ospf-rust-framework-bpp2d = { path = "../ospf-rust-framework-bpp2d" }
```

## 相关文档

- [ospf-rust README](../README_ch.md) 列出全部 crate 及其 Kotlin 边界。
- [geometry2d](../ospf-rust-framework-geometry2d/README_ch.md) 提供共享二维几何。
- [bpp2d.md](bpp2d.md) 记录迁移交接与已知差距。

## 本地验证

```powershell
$env:RUSTC_BOOTSTRAP = "1"
cargo test -p ospf-rust-framework-bpp2d
```
