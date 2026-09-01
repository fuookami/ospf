# OSPF Rust Framework BPP3D

:us: [English](README.md) | :cn: 简体中文

## 简介

`ospf-rust-framework-bpp3d` 是 Kotlin `ospf-kotlin-framework-bpp3d` 的 Rust 迁移目标。它提供可复用三维装箱领域能力，包括 BPP3D infrastructure、item 建模、BLA/block-loading/layer-generation context、layer-assignment pipeline、packing、CSV 协议适配、renderer DTO，以及 application 层 column-generation 编排。

## 作用范围

本 crate 拥有可复用 BPP3D framework 内核，把装箱几何、候选生成、层分配、最终装箱、fixture 协议和 renderer DTO 边界保留在共享领域 crate 内。

明确非目标：

1. 业务专用请求 DTO、租户运行时上下文、公式语言或项目部署策略。
2. 外部 renderer 源码。
3. solver 安装、许可证管理或非 BPP3D 后端插件所有权。

## 模块结构

| Rust 模块或目录 | Kotlin 边界 | 职责 |
| --- | --- | --- |
| [`src/infrastructure`](src/infrastructure/README_ch.md) | `bpp3d-infrastructure` | 几何原语、orientation、packing shape、PWL approximation helper 和 renderer DTO。 |
| [`src/domain/item`](src/domain/item/README_ch.md) | `bpp3d-domain-item-context` | item、package、material、bin、layer、pattern、demand coverage、shape metadata 和 continuous-radius model component。 |
| [`src/domain/bla`](src/domain/bla/README_ch.md) | `bpp3d-domain-bla-context` | bottom-up-left-justified placement 与投影候选构造。 |
| [`src/domain/block_loading`](src/domain/block_loading/README_ch.md) | `bpp3d-domain-block-loading-context` | simple/complex block generation、DFS 空间分裂、MLHS search 和 block-loading diagnostics。 |
| [`src/domain/layer_generation`](src/domain/layer_generation/README_ch.md) | `bpp3d-domain-layer-generation-context` | Block、BL、circle-packing、Pattern、Pile 和 Historical 层候选生成。 |
| [`src/domain/layer_assignment`](src/domain/layer_assignment/README_ch.md) | `bpp3d-domain-layer-assignment-context` | RMP/final MILP 变量、动态列、约束、目标、需求/深度/启用限制和 shadow price。 |
| [`src/domain/packing`](src/domain/packing/README_ch.md) | `bpp3d-domain-packing-context` | 最终 packed-bin 转换、geometry guard、placement/block trace replay、material summary 和 render adaptation。 |
| [`src/application`](src/application/README_ch.md) | `bpp3d-application` 与 layer-selection flow | solver backend 选择、CSV 物化、column-generation 生命周期、fixture report、KPI/trace diagnostics 和 renderer output。 |

## 架构概览

建模路径以 `MetaModel` 为轴心：RMP 和 final MILP 共享 `LayerAssignmentContext`、`LayerAssignmentAggregation`、变量组件、需求/深度/启用约束和目标 pipeline。application service 负责 solver backend、CSV 物化、trace/KPI 诊断和 renderer 适配；placement/block trace replay 继续保留在 packing domain。

Kotlin `bpp3d-domain-layer-selection-context` 在 Rust 中映射到 application 编排，因为 Rust 把 layer selection 作为围绕可复用 domain context 的 solver 生命周期处理。


## 圆柱体几何语义

框架支持轴对齐圆柱体，具有以下语义：

### 垂直圆柱体（Axis3.Y）

- 完整支持圆形填充网格候选和保守长方体支撑堆候选。
- 底部重叠/支撑检查使用真实足迹几何。
- 渲染器 DTO 中的 `radius` 和 `diameter` 字段。

### 水平圆柱体（Axis3.X / Axis3.Z）

- **固定/离散半径**：轴感知放置的圆形填充网格路径。
- **支撑堆**：单全长支撑、重复同形多支撑区间、异构支撑区间。
- **悬挂候选**：覆盖圆柱轴的单或重复窄长方体支撑线悬挂。
- **支撑要求**：必须在容器底部或具有覆盖完整圆柱轴和底部支撑线的长方体支撑区间。
- **层约束**：单个 `BinLayer` 不能混合多个圆柱轴。

### 连续半径建模

- **PWL 近似**：`UnivariateLinearPiecewiseFunction` 符号用于通过 `π·r²·h` 的体积近似。
- **保守包络**：`rMax` 包络边界用于生产就绪的连续半径变量。
- **生产就绪路径**：`RealVar` 求解器变量，具有约束边界和目标等式约束。
- **渲染器输出**：求解器选择的半径应用于 `actualVolume` 和 `radius`/`diameter` DTO 字段。
- **PWL 诊断**：渲染器项目信息中的 `pwl_volume`、`pwl_error`、`pwl_segments`、`pwl_within_envelope`。
## 核心概念

1. `BinLayer` 是 layer assignment 中的被选择列。
2. layer generation 由 BLA、block loading、circle packing、Pattern、Pile 和 Historical 来源提供候选层。
3. layer assignment 通过 `MetaModel` 注册 RMP/final MILP 变量和约束/目标 pipeline。
4. packing 把选中的层转换为 packed bin，并在 renderer output 前校验 shape-aware geometry。
5. CSV fixture suite 提供 Kotlin 对比基线和 solver backend diagnostics。

## Public API

| API | 职责 | 稳定性 |
| --- | --- | --- |
| `application::ColumnGenerationApplicationService` | application-facing column-generation 编排。 | migration |
| `application::MetaModelSolverBackend` | RMP/final executor 的可插拔 solver backend 边界。 | migration |
| `application::ColumnGenerationStandardExecutors` | 标准 RMP/final executor 接线。 | migration |
| `application::CsvDatasetLoader` | 启用 `serde` 时的 CSV dataset 加载。 | migration |
| `application::Bpp3dRunReport` | 结构化运行与对比报告。 | migration |
| `domain::layer_assignment::*` | layer-assignment context、aggregation、limit、objective 和动态列。 | migration |
| `domain::layer_generation::*` | 层候选生成 trait、request、diagnostics 和策略实现。 | migration |
| `domain::packing::*` | packing 转换、geometry guard、packed-bin solution 和 render adaptation。 | migration |
| `domain::packing::PackingGeometryContract` | 可注入的最终装箱几何校验。 | migration |
| `infrastructure::*` | 共享 geometry、orientation、shape、PWL 和 renderer DTO 类型。 | stable within migration |

## 建模扩展点

solver 行为通过 `MetaModelSolverBackend` 或 RMP/final executor trait 接入。新的 layer candidate source 应实现 layer-generation trait。业务专用的最终几何校验通过 `PackingGeometryContract` 和 `ColumnGenerationApplicationService::with_geometry_guard` 注入。请求级业务规则应通过 package-rule policy 等 domain policy 注入，不要把任意闭包塞进可序列化模型。

新增约束、目标和结果提取应放在 domain context、aggregation、model component 或 pipeline 中。application 代码负责组合这些扩展点，而不是复制 domain 建模逻辑。

列生成生命周期扩展：

1. `ColumnGenerationRmpExecutor::execute_result` 和 `ColumnGenerationFinalExecutor::execute_result` 将注册/求解失败以 `Result` 传播，并标注 `RestrictedMasterProblem` 或 `FinalMilp` 阶段。
2. `ColumnGenerationRmpModelExtension`、`ColumnGenerationFinalModelExtension` 可在标准 RMP/final context 上追加模型内容；RMP 扩展可返回类型化 `additional_shadow_prices`。
3. `ColumnGenerationApplicationService::create_algorithm_with` 支持替换算法装配；算法还提供可失败的初始列和候选过滤入口。

## 泛型数值边界

面向 domain 的 API 在表达可复用几何、物理量和需求语义时保留泛型数值。solver-facing RMP/final 执行当前在注册和 adapter 边界使用 `MetaModel<f64>`。裸 `f64` 转换应保留在 solver、CSV、reporting 或 renderer 边界。

## 物理量边界

宽、高、深、重量、体积、半径、物料尺寸、包件尺寸、需求覆盖和容量类数量应通过 `Quantity<V, U>` 或 infrastructure geometry wrapper 表达。renderer DTO 和 CSV 协议可以暴露标量字段，但标量转换属于序列化边界。

## 求解生命周期

当前 application flow：

1. 加载或构造 BPP3D request 和候选来源集合。
2. 生成 BLA/block/circle/Pattern/Pile/Historical 层候选。
3. 注册 layer-assignment RMP 变量、约束、目标和 shadow-price metadata。
4. 求解 RMP，提取稳定 demand shadow price，生成或刷新列，并注册新的 layer 变量。
5. 在共享 layer-assignment context 上求解 final MILP。
6. 提取 selected layer，并通过 packing replay placement/block trace。
7. 校验最终几何并输出 solution、KPI、diagnostics 和 renderer DTO。

## 输出

`ColumnGenerationResult` 记录 selected layer、packing analysis、solver status 和生命周期 diagnostics。`Bpp3dRunReport` 与 fixture-suite report 记录 dataset comparison、solver availability、failure、demand coverage、packed-bin output、selected-layer diagnostics 和 feature-matrix 信息。renderer DTO 包含长方体和圆柱的 shape metadata 与 `actualVolume` 输出。

## 使用方式

```rust,ignore
use ospf_rust_framework_bpp3d::application::{
    ColumnGenerationApplicationService, ColumnGenerationConfig,
};

let service = ColumnGenerationApplicationService::new(ColumnGenerationConfig::default());
let result = service.solve(request)?;
```

CSV 与 fixture-suite 入口需要启用 `serde` feature。

## 本地验证

```powershell
cargo check -p ospf-rust-framework-bpp3d
cargo test -p ospf-rust-framework-bpp3d --lib
cargo check -p ospf-rust-framework-bpp3d --features serde
cargo test -p ospf-rust-framework-bpp3d --features serde --lib
```

真实 solver manifest suite 由 feature gate 控制，并需要本地 solver 可用。

## 当前边界

当前迁移覆盖：

1. solver-agnostic `MetaModel` RMP/final executor、可插拔 backend wrapper，以及一轮 RMP -> shadow-price generation -> column refresh -> final flow。
2. 层候选 demand coverage metadata、稳定 demand shadow-price key、selected-layer 提取，以及基于 placement/block trace 的 packed-bin/render replay。
3. BLA local/global 层候选、simple-block 层候选、多轮 axis `ComplexBlockGenerator`、有界 DFS 空间分裂、MLHS branch/depth 候选排序、固定/离散半径圆柱 circle-packing 网格候选，以及 request-aware Pattern/Pile/Historical 生成器。
4. 轴感知长方体和圆柱几何、带 guard 的横向圆柱 final validation、PWL continuous-radius metadata/fixture，以及 renderer `actualVolume` 输出。
5. CSV schema guard、Kotlin Gurobi grouped-layer/material-width-amount 适配、manifest 和递归目录 fixture 加载、backend survey/no-run/fake fallback 报告，以及 feature-matrix diagnostics。
6. PatternedItem 保守 demand coverage、PackageAttribute 校验/装箱 diagnostics，以及 RestAmount/TailBinLoadingRate/BinLoadingOrder 的 `MetaModel` 注册入口。

manifest 已镜像 Kotlin Gurobi CSV 样例全集，共 22 个 fixture，包括 19 个 Kotlin 派生 dataset 样例和 3 个本地回归 fixture。剩余差距以本节的当前边界清单为准；更新迁移覆盖时应同步对照 Kotlin BPP3D README。

## 相关模块

- [根 README](../README_ch.md)
- [BPP3D domain README](src/domain/README_ch.md)
- [BPP3D application README](src/application/README_ch.md)
- [Kotlin BPP3D README](../../ospf-kotlin/ospf-kotlin-framework-bpp3d/README_ch.md)
