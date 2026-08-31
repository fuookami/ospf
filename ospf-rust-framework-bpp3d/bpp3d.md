# ospf-rust-framework-bpp3d 迁移交接文档

## 1. 目标

将 `E:\workspace\ospf\ospf-kotlin\ospf-kotlin-framework-bpp3d` 迁移到 Rust workspace 中的新 crate：

```text
E:\workspace\ospf\ospf-rust\ospf-rust-framework-bpp3d
```

迁移目标不是逐文件翻译 Kotlin，而是用 Rust 风格复刻 BPP3D 领域框架能力，并严格对齐当前 Rust 项目的 framework 架构规范：

1. 以 `ospf_rust_core::model::MetaModel` 为模型装配轴心。
2. 以 context / aggregation / model component / pipeline 组织优化建模能力。
3. application 层只负责流程编排、solver 选择、状态映射、CSV/renderer 协议适配、trace/KPI 组织，不直接堆积变量、约束、目标和 token 解析逻辑。
4. 普通 RMP、列生成 LP、最终 MILP 尽量共享同一套 context / aggregation / pipeline 注册路径。
5. public API 保持泛型数值和物理量边界，求解器适配边界内再落到 `f64`。
6. 不照搬 Kotlin 的 `QuantityPoint*`、`QuantityVector*`、`QuantityPlacement*` 类型体系；Rust 版必须基于泛型、trait、关联类型和 newtype 重新设计正交 typed geometry adapter。

第一阶段目标是建立可编译、可测试、可扩展的 Rust BPP3D 领域框架骨架，并完成 typed geometry adapter、基础几何语义、item domain 和 layer assignment 最小 RMP 的可验证闭环。

## 2. 当前前置条件评估

Rust 基础能力层已经具备迁移条件：

| 能力 | Rust 现状 | 迁移结论 |
| --- | --- | --- |
| 元模型 | `ospf_rust_core::model::MetaModel<V>` | 可承接 Kotlin `MetaModel<FltX>` |
| 求解器入口 | `ospf_rust_framework::solver::ColumnGenerationSolver` | 可承接 LP / MILP / typed solve |
| 对偶解 | `LinearDualSolution`、`MetaDualSolution` | 可承接 shadow price 提取 |
| 管道扩展 | `Pipeline`、`CGPipeline` | 可承接 limits / objective / extractor |
| 影子价格 | `ShadowPriceMap`、`BasicShadowPriceMap` | 可作为 BPP3D shadow price map 基础 |
| 函数符号 | `BinaryzationFunction`、`SlackFunction`、`MaskingFunction`、`OrFunction`、`UnivariateLinearPiecewiseFunction` | 可承接 Kotlin layer assignment 和连续半径 PWL |
| 纯几何 | `ospf_rust_math::geometry` | 已有 Axis、Point、Vector、Cuboid、Cylinder、Box、Placement、Projection 等纯几何底座 |
| 物理量 | `Quantity<V, U>`、`UnitTrait`、单位系统 | 可承接长度、重量、体积、半径、容量等物理量 |
| workspace | 已新增 `ospf-rust-framework-bpp3d` | 可继续实现 |

主要缺口不在底层 math / quantities，而在 BPP3D 领域层：

1. Rust 风格 typed geometry adapter。
2. shape-aware packing geometry 语义。
3. 横向圆柱 X/Z 支撑覆盖门禁。
4. BPP3D 领域变量集合、列集合和 token 映射。
5. CSV/renderer 协议兼容层。
6. layer generation / packing / application 的稳定 context 化。

## 3. Kotlin 子模块到 Rust 包映射

| Kotlin 子模块 | Rust 包路径 | 说明 |
| --- | --- | --- |
| `bpp3d-infrastructure` | `crate::infrastructure` | typed geometry adapter、packing shape、orientation、projection、support guard、renderer DTO |
| `bpp3d-domain-item-context` | `crate::domain::item` | item、package、material、bin、layer、demand statistics、continuous radius |
| `bpp3d-domain-bla-context` | `crate::domain::bla` | Bottom-up-left-justified 算法 |
| `bpp3d-domain-block-loading-context` | `crate::domain::block_loading` | block generation、DFS/MLHS、space splitting |
| `bpp3d-domain-layer-assignment-context` | `crate::domain::layer_assignment` | RMP/final MILP 赋值模型、load/capacity、limits、shadow price |
| `bpp3d-domain-layer-generation-context` | `crate::domain::layer_generation` | layer generation request/result、candidate generators |
| `bpp3d-domain-packing-context` | `crate::domain::packing` | final packing、material packing、geometry guard、renderer adapter |
| `bpp3d-application` | `crate::application` | column generation algorithm/service/analyzer、CSV schema、depth boundary policy |

## 4. 推荐模块结构

当前 crate 已创建如下骨架，后续实现应在此基础上细化：

```text
src/
  lib.rs
  infrastructure/
    mod.rs
    geometry.rs
    renderer.rs
  domain/
    mod.rs
    item/
      mod.rs
      model.rs
      service.rs
    bla/
      mod.rs
      service.rs
    block_loading/
      mod.rs
      model.rs
      service.rs
    layer_assignment/
      mod.rs
      model.rs
      service.rs
    layer_generation/
      mod.rs
    packing/
      mod.rs
      model.rs
      service.rs
  application/
    mod.rs
    service.rs
```

如实现增长较快，允许继续拆分为：

```text
infrastructure/typed_geometry/
infrastructure/packing_shape/
domain/item/model/
domain/layer_assignment/model/
domain/layer_assignment/service/limits/
domain/layer_generation/service/
application/csv/
```

但对外 re-export 应保持稳定，避免调用方依赖内部文件拆分。

## 5. 关键迁移原则

### 5.1 不照搬 Kotlin Quantity* 类型体系

Kotlin 的 `QuantityPoint2/3`、`QuantityVector2/3`、`QuantityPlacement2/3` 很大程度是为了弥补 Kotlin 类型系统难以正交组合数值类型、单位类型、几何维度和领域语义的问题。

Rust 版必须重新设计：

1. `ospf_rust_math::geometry` 保持纯标量几何。
2. `ospf_rust_quantities` 负责 `Quantity<V, U>` 和单位系统。
3. `ospf-rust-framework-bpp3d` 定义少量正交 typed geometry newtype / struct。
4. BPP3D 领域语义通过组合 typed geometry，而不是把 item/bin/layer 语义混入基础几何点和放置。

推荐方向：

```rust,ignore
pub struct MetricPoint3<V, U>
where
    U: UnitTrait,
{
    pub x: Quantity<V, U>,
    pub y: Quantity<V, U>,
    pub z: Quantity<V, U>,
}

pub struct MetricSize3<V, U>
where
    U: UnitTrait,
{
    pub width: Quantity<V, U>,
    pub height: Quantity<V, U>,
    pub depth: Quantity<V, U>,
}

pub struct MetricPlacement3<V, U, S>
where
    U: UnitTrait,
{
    pub position: MetricPoint3<V, U>,
    pub shape: S,
}
```

避免的方向：

```rust,ignore
// 不推荐：照搬 Kotlin 迁移期类型命名和继承层级。
pub struct QuantityPlacement3<...> { ... }
```

### 5.2 shape-aware geometry 归 BPP3D

`ospf-rust-math::geometry::Placement3::overlapped` 是 bounding-box 语义。BPP3D 的真实几何门禁必须放在 BPP3D infrastructure：

1. 圆-圆 footprint。
2. 圆-矩形 footprint。
3. 矩形-矩形 footprint。
4. 竖直圆柱真实 footprint。
5. 横向圆柱 bounding rectangle + support coverage。
6. final packing / rendering geometry guard。

### 5.3 建模层遵循 framework 架构

领域建模组件必须通过 context / aggregation / model component / pipeline 注册到 `MetaModel`：

1. context：应用层入口，组装 aggregation 和 extra pipeline。
2. aggregation：组合多个 model component，协调注册顺序。
3. model component：持有变量、中间表达式、派生表达式和结果提取引用。
4. pipeline / limit：单一约束族、目标族、惩罚项或 shadow price 提取。

### 5.4 泛型和物理量边界

public API 优先使用泛型数值 `V` 和 `Quantity<V, U>`。只有 solver adapter、model registration、solution extraction 边界可以使用 `f64`。

长度、重量、体积、容量、半径、需求数量不得在 public API 里以裸 `f64` 随意混算。

### 5.5 注释与文档

Rust 公共 API 注释必须中英双语，中文在前，英文在后。新增 README 时保持 `README.md` 与 `README_ch.md` 互链。

## 6. 阶段计划

### 阶段 0：基础校验与迁移矩阵

目标：确认 Kotlin 行为基线，避免迁移范围失控。

事项：

1. 读取 Kotlin BPP3D 各子模块 README、daily 和测试。
2. 建立迁移矩阵：Kotlin 类型、Rust 目标类型、迁移阶段、测试来源、是否第一版支持。
3. 明确第一版不支持项。
4. 记录 CSV 和 renderer 外部协议，不允许迁移过程中静默改变字段语义。

建议第一版不支持：

1. 任意三维圆柱旋转。
2. 未 guard 的 coordinate-less horizontal cylinder hanging。
3. cuboid-only DFS/MLHS 对圆柱的完全 shape-polymorphic 支持。
4. solver-native continuous radius 完整优化闭环之外的未注册 interval-only 变量。
5. 所有 Gurobi dataset suite 一次性全量通过。

验收：

1. `bpp3d.md` 迁移矩阵补充完整。
2. 每个 Kotlin 子模块都有 Rust 目标包。
3. 不支持项在 README 或模块文档中明确说明。

### 阶段 1：typed geometry adapter

目标：设计 Rust 风格 typed geometry，不搬 Kotlin `Quantity*` 层级。

目标类型：

1. `MetricPoint2<V, U>`
2. `MetricPoint3<V, U>`
3. `MetricVector2<V, U>`
4. `MetricVector3<V, U>`
5. `MetricSize2<V, U>`
6. `MetricSize3<V, U>`
7. `MetricAabb2<V, U>`
8. `MetricAabb3<V, U>`
9. `MetricPlacement2<V, U, S>`
10. `MetricPlacement3<V, U, S>`

必须能力：

1. 同单位加减。
2. 同单位比较。
3. AABB contains / overlaps / intersect。
4. 显式转换到 `ospf_rust_math::geometry` 标量几何。
5. 显式从标量几何和单位构造 typed geometry。
6. 禁止不同单位裸混算。

验收：

1. typed geometry 单元测试通过。
2. 所有转换都在 adapter 中集中完成。
3. public API 不出现 Kotlin 迁移期 `QuantityPoint*` 命名。

### 阶段 2：BPP3D infrastructure

目标：迁移 `bpp3d-infrastructure` 的领域几何语义。

Kotlin 来源：

1. `PackingShape.kt`
2. `Placement.kt`
3. `Projection.kt`
4. `Cuboid.kt`
5. `Cylinder.kt`
6. `Container.kt`
7. `Orientation.kt`
8. `OrientationAxisPermutationMapping.kt`
9. `ProjectivePlaneGeometryMapping.kt`
10. `HorizontalCylinderSupportCoverage.kt`
11. `ConservativeRadiusEnvelope.kt`
12. `PWLRadiusApproximationConfig.kt`
13. `PWLRadiusSquaredApproximation.kt`
14. `dto/RendererDTO.kt`

Rust 目标：

1. `PackingShape3`
2. `PackingShapeType`
3. `PackingAlgorithmShapeType`
4. `ShapeFootprint2`
5. `ShapePlacement3`
6. `Orientation`
7. `CuboidView`
8. `ProjectivePlane`
9. `PackingGeometryGuard`
10. `HorizontalCylinderSupportCoverage`
11. `ConservativeRadiusEnvelope`
12. `PwlRadiusApproximationConfig`
13. `PwlRadiusSquaredApproximation`
14. `RenderLoadingPlanDto`
15. `RenderLoadingPlanItemDto`
16. `SchemaDto`

重点设计：

1. 复用 `ospf_rust_math::geometry::Axis3`、`AxisPlane3`、`Cylinder3`、`Cuboid3`。
2. `ShapePlacement3` 使用真实 footprint 计算，不依赖 `Placement3::overlapped` 的 bounding-box 语义。
3. horizontal cylinder support coverage 必须集中为一个共享 contract。
4. renderer DTO 通过 `serde` feature 开启序列化。

测试迁移：

1. `CuboidCoreTest.kt`
2. `ContainerShapeTest.kt`
3. `ContainerGeometryContractTest.kt`
4. `OrientationTest.kt`
5. `PackingShapeTest.kt`
6. `PlacementTest.kt`
7. `ProjectionTest.kt`
8. `ProjectionPlacementContractTest.kt`
9. `Bpp3dGeometryWrapperContractTest.kt`
10. `HorizontalCylinderSupportCoverageTest.kt`
11. `ConservativeRadiusEnvelopeTest.kt`
12. `PWLRadiusSquaredApproximationTest.kt`
13. `RendererDTOTest.kt`

验收：

1. Cuboid + Axis3.Y Cylinder 几何测试通过。
2. Axis3.X / Axis3.Z 横向圆柱 bounding shape、actual volume、support coverage 测试通过。
3. PWL 半径平方近似误差测试通过。
4. renderer fixture 序列化结构兼容 Kotlin。

### 阶段 3：item domain

目标：迁移 `bpp3d-domain-item-context`。

Kotlin 来源：

1. `Item.kt`
2. `Package.kt`
3. `Material.kt`
4. `Bin.kt`
5. `Layer.kt`
6. `PackageAttribute.kt`
7. `Pattern.kt`
8. `Schema.kt`
9. `DemandStatistics.kt`
10. `DemandReducedCost.kt`
11. `QuantityDomainModels.kt`
12. `ContinuousRadiusModelComponent.kt`
13. `ContinuousRadiusSelectionExtractor.kt`
14. `CylinderShapeContract.kt`
15. `PlacementFactory.kt`
16. `PlacementPlaneMapping.kt`

Rust 目标：

1. `PackageShape`
2. `PackageShapeSpec`
3. `Package`
4. `PackingProgram`
5. `Item`
6. `ActualItem`
7. `PatternedItem`
8. `Material`
9. `Bin`
10. `BinLayer`
11. `DemandStatistics`
12. `DemandReducedCost`
13. `CylinderShapeContract`
14. `ContinuousRadiusModelComponent`

重点设计：

1. `PackageShapeSpec::Cylinder` 不要沿用 Kotlin 的 `VerticalCylinder` 命名误导；Rust 版应清晰表达 axis-aware cylinder。
2. 连续半径路径拆分：
   - fixed radius
   - discrete radius candidates
   - native continuous radius
   - PWL continuous radius
   - blocked/gap report
3. `ContinuousRadiusModelComponent` 必须注册 native/PWL 变量并提取结果，但 application 不能直接拼 PWL Big-M 约束。

测试迁移：

1. `PackageShapeSpecTest.kt`
2. `CylinderShapeContractTest.kt`
3. `ContinuousRadiusModelComponentTest.kt`
4. `ContinuousRadiusSelectionExtractorTest.kt`
5. `DemandStatisticsTest.kt`
6. `DemandReducedCostTest.kt`
7. `QuantityDomainModelExampleTest.kt`
8. `MaterialDemandEntriesTest.kt`

验收：

1. package/item/material/bin/layer 基础模型测试通过。
2. continuous radius registration plan 诊断信息与 Kotlin 语义对齐。
3. fixed/discrete/native/PWL/blocked 五类路径互斥清晰。

### 阶段 4：BPP3D 建模 adapter

目标：补齐 Kotlin DSL 到 Rust `MetaModel` 的承接层。

必须实现的内部能力：

1. 一维变量集合，例如 `VariableArray1<K, T>`。
2. 二维变量集合，例如 `VariableArray2<K1, K2, T>`。
3. layer column 到变量 index/token 的双向映射。
4. load / capacity / demand 中间表达式集合。
5. 注册后结果提取 helper。
6. 列生成时新增列和最终 MILP 注册路径复用。

建议位置：

```text
domain/layer_assignment/model/variable_array.rs
domain/layer_assignment/model/expression_array.rs
domain/layer_assignment/model/solution_extraction.rs
```

验收：

1. adapter 能在测试中向 `MetaModel<f64>` 注册一维/二维变量。
2. adapter 能注册中间线性表达式和函数符号。
3. adapter 能从 solution vector 提取领域结果。
4. application 层不暴露 token 解析细节。

### 阶段 5：layer assignment RMP/final MILP

目标：迁移 `bpp3d-domain-layer-assignment-context`。

Kotlin 来源：

1. `model/Assignment.kt`
2. `model/Load.kt`
3. `model/Capacity.kt`
4. `model/LayerAggregation.kt`
5. `model/Bpp3dSolverValueAdapter.kt`
6. `model/ScaledBpp3dSolverValueAdapter.kt`
7. `service/SolutionAnalyzer.kt`
8. `service/limits/*`

Rust 目标：

1. `ImpreciseAssignment`
2. `PreciseAssignment`
3. `LayerAggregation`
4. `Load`
5. `Capacity`
6. `Bpp3dSolverValueAdapter`
7. `SolutionAnalyzer`
8. `DemandConstraint`
9. `BinCapacityConstraint`
10. `BinDepthConstraint`
11. `BinAmountMinimization`
12. `VolumeMinimization`
13. `RestAmountMinimization`
14. `TailBinAssignmentConstraint`
15. `BetterLayerMaximization`

建模能力：

1. RMP 阶段 `x_iteration[layer]` 列变量。
2. final MILP 阶段 `x[bin, layer]` 赋值变量。
3. binaryzation 中间符号 `u` / `v`。
4. tail bin 标记。
5. demand load / over / less。
6. capacity load weight / volume。
7. shadow price key 和 extractor。

验收：

1. 最小 RMP 能注册模型。
2. final MILP 能注册 `PreciseAssignment`。
3. `DemandConstraint` 能添加上下界约束并从 dual 提取 shadow price。
4. 关键 limits 都以 pipeline/limit 形式注册，不在 application 拼约束。

### 阶段 6：layer generation / BLA / block loading

目标：迁移候选生成能力。

Kotlin 来源：

1. `bpp3d-domain-layer-generation-context`
2. `bpp3d-domain-bla-context`
3. `bpp3d-domain-block-loading-context`

Rust 目标：

1. `LayerGenerationContext`
2. `LayerGenerator`
3. `BlockLayerGenerator`
4. `BLLocalLayerGenerator`
5. `BLGlobalLayerGenerator`
6. `PatternLayerGenerator`
7. `PileLayerGenerator`
8. `CirclePackingLayerGenerator`
9. `HistoricalLayerGenerator`
10. `BottomUpLeftJustifiedAlgorithm`
11. `SimpleBlockGenerator`
12. `ComplexBlockGenerator`
13. `DepthFirstSearchAlgorithm`
14. `MultiLayerHeuristicSearchAlgorithm`

分阶段开放：

1. Cuboid baseline。
2. Axis3.Y vertical cylinder。
3. Axis3.X / Axis3.Z fixed/discrete radius guarded circle-packing。
4. Horizontal supported-stack / hanging guarded candidates。
5. 其余 block loading shape-polymorphic 改造。

验收：

1. BLA contract 测试通过。
2. simple/complex block generator contract 测试通过。
3. circle packing 支持固定/离散半径圆柱。
4. 手工伪造 unsupported horizontal cylinder candidate 会被拒绝。

### 阶段 7：packing context

目标：迁移 final packing、material packing、renderer adapter。

Kotlin 来源：

1. `PackingContext.kt`
2. `Aggregation.kt`
3. `service/Packer.kt`
4. `service/MaterialPacker.kt`
5. `service/PackingGeometryGuard.kt`
6. `service/PackingGeometryContract.kt`
7. `service/PackingRendererAdapter.kt`
8. `model/MaterialPackingPlan.kt`
9. `model/PackageSolutionLikeAdapter.kt`

Rust 目标：

1. `PackingContext`
2. `Packer`
3. `MaterialPacker`
4. `PackingGeometryGuard`
5. `PackingGeometryContract`
6. `PackingRendererAdapter`
7. `MaterialPackingPlan`
8. `PackageSolutionLikeAdapter`

验收：

1. known-coordinate cuboid/cylinder final packing 测试通过。
2. horizontal cylinder final guard 支持贴地或长方体支撑覆盖。
3. renderer schema 输出 `actualVolume`，不只使用 bounding cuboid volume。
4. PackerAndRendererAdapter 测试通过。

### 阶段 8：application 层

目标：迁移应用编排和输入/输出协议。

Kotlin 来源：

1. `ColumnGenerationAlgorithm.kt`
2. `ColumnGenerationApplicationService.kt`
3. `ColumnGenerationPackingAnalyzer.kt`
4. `ColumnGenerationStandardExecutors.kt`
5. `DepthBoundaryLayerOrientationPolicy.kt`
6. `LayerPlacementAdapter.kt`
7. Gurobi dataset tests and CSV fixtures

Rust 目标：

1. `ColumnGenerationAlgorithm`
2. `ColumnGenerationApplicationService`
3. `ColumnGenerationPackingAnalyzer`
4. `ColumnGenerationStandardExecutors`
5. `DepthBoundaryLayerOrientationPolicy`
6. `LayerPlacementAdapter`
7. CSV dataset loader
8. schema guard

编排步骤：

1. 构造 item/material/layer demand。
2. 构造 initial columns。
3. RMP LP solve。
4. shadow price extraction。
5. layer generation。
6. reduced cost filter。
7. column deduplication。
8. final MILP solve。
9. packing analyzer。
10. renderer schema 输出。

验收：

1. application 不直接拼领域变量、约束和目标。
2. CSV schema guard 拒绝重复列、未知列、非法 axis、非法 depth boundary policy。
3. depth boundary policy 只在 final/known-coordinate 阶段校验，不提前过滤生成候选。
4. mock solver column generation 测试通过。
5. Gurobi/SCIP 集成测试作为可选 feature。

## 7. 测试迁移清单

优先级 P0：

1. `CuboidCoreTest`
2. `ContainerShapeTest`
3. `OrientationTest`
4. `PackingShapeTest`
5. `PlacementTest`
6. `ProjectionTest`
7. `HorizontalCylinderSupportCoverageTest`
8. `PWLRadiusSquaredApproximationTest`
9. `RendererDTOTest`
10. `PackageShapeSpecTest`
11. `CylinderShapeContractTest`
12. `ContinuousRadiusModelComponentTest`
13. `DemandStatisticsTest`
14. `LayerAssignmentQuantityCompileContractTest`
15. `ItemDemandConstraintModeKeyTest`

优先级 P1：

1. `BottomUpLeftJustifiedAlgorithmContractTest`
2. `SimpleBlockGeneratorContractTest`
3. `ComplexBlockGeneratorContractTest`
4. `LayerGenerationQuantityContractTest`
5. `LayerGenerationProgramCandidateAdaptersTest`
6. `PackerAndRendererAdapterTest`
7. `MaterialPackerTest`
8. `PackageSolutionLikeAdapterTest`
9. `DepthBoundaryLayerOrientationPolicyTest`

优先级 P2：

1. `ColumnGenerationAlgorithmTest`
2. `ColumnGenerationApplicationIntegrationTest`
3. `ColumnGenerationPackingAnalyzerQuantityEntryPointTest`
4. `ColumnGenerationQuantityShapeSpecEntryPointTest`
5. `GurobiColumnGenerationTest`
6. 全量 CSV dataset suite

## 8. 验收命令

基础验收：

```powershell
cargo check -p ospf-rust-framework-bpp3d
cargo test -p ospf-rust-framework-bpp3d --no-run
cargo test -p ospf-rust-framework-bpp3d --lib
```

涉及 serde DTO：

```powershell
cargo test -p ospf-rust-framework-bpp3d --features serde
```

涉及 async 路径：

```powershell
cargo check -p ospf-rust-framework-bpp3d --features async
```

涉及 Gurobi：

```powershell
cargo test -p ospf-rust-framework-bpp3d --features gurobi10
```

涉及 SCIP：

```powershell
cargo test -p ospf-rust-framework-bpp3d --features scip
```

## 9. 完成定义

第一版完成定义：

1. crate 可编译并进入 workspace。
2. typed geometry adapter 完成，并无 Kotlin `Quantity*` 迁移期命名。
3. infrastructure 几何、PWL、renderer DTO P0 测试通过。
4. item domain 核心模型和 continuous radius 诊断测试通过。
5. layer assignment 最小 RMP 能注册到 `MetaModel`。
6. `DemandConstraint` 能添加约束并提取 shadow price。
7. README / README_ch 描述 public API、扩展点、暂不支持项。
8. `bpp3d.md` 中每个 P0 项都有完成状态。

完整迁移完成定义：

1. Kotlin README 范围内所有子模块均有 Rust 实现或明确不支持说明。
2. RMP LP、final MILP 和后续 column generation 共享 context / aggregation / pipeline 注册路径。
3. Cuboid、Axis3.Y Cylinder、guarded Axis3.X/Z Cylinder 的 generated/final/render 路径均有测试。
4. PWL continuous radius 注册、提取、renderer 回写闭环可验证。
5. application 层没有大量直接 `model.add_*` 或 token 解析逻辑。
6. CSV dataset suite 在对应 solver feature 下通过。

## 10. 执行建议

下一个会话接手时，建议按以下顺序开始：

1. 运行 `cargo check -p ospf-rust-framework-bpp3d` 确认骨架状态。
2. 先实现 `infrastructure::geometry` 的 Rust typed geometry adapter。
3. 迁移 infrastructure P0 几何测试。
4. 再迁移 `domain::item` 的 `PackageShapeSpec`、`Item`、`BinLayer`。
5. 在实现 `layer_assignment` 前先完成变量/表达式 adapter。
6. 每完成一个阶段更新本文件清单状态，避免后续实现者重复盘点。
