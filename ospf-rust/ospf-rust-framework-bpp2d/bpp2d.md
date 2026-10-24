# BPP2D 迁移交接

## 当前状态

`ospf-rust-framework-bpp2d` 已按与 BPP3D 同构的分层落地：infrastructure、item、bla、block-loading、
layer-generation、layer-assignment、packing 与 application。二维几何来自共享 crate
`ospf-rust-framework-geometry2d`，与 CSP2D 共用同一实现。

`cargo test -p ospf-rust-framework-bpp2d` 通过，共 145 条契约测试。

## 已完成事项

### infrastructure

- `Bpp2dValidationError` / `Bpp2dSolvingError` / `Bpp2dCapabilityError` / `Bpp2dInternalError`
  与对应构造 helper。
- 渲染 DTO：`RenderPlacementDto` / `RenderLayerDto` / `RenderSheetDto` / `RenderLoadingPlanDto`，
  启用 `serde` 时使用 `camelCase` 字段名。
- 再导出共享二维几何，确保本 crate 内只有一份几何实现。

### domain/item

- 标识类型 `SheetTypeId` / `ItemId` / `MaterialNo` / `BatchNo`。
- `Material` / `MaterialKey` / `MaterialType`。
- `AbstractItem` trait、`ActualItem`、`PatternItem`、`ItemKey` 与 `item_key` /
  `item_candidate_shapes` / `item_shape_type`。
- `SheetType`（面积、主板材标记）、`Sheet`、`sheet_loading_rate`。
- `Layer`（件数、实际面积、外接面积、利用率、件数映射、形状计数、身份键）、`SheetLayer`。
- `Bpp2dDemandMode` / `Bpp2dDemandKey` / `Bpp2dDemandValue` / `Bpp2dDemandEntry` /
  `DemandModeKey` 与件数聚合。

### domain/bla

- `BottomUpLeftJustifiedAlgorithm`：沿 X/Y 滑动、加权距离排序、候选点裁剪到板材内。
- 圆形件跳过并记入 `skipped_circle_items`；放不下的物品记入 `skipped_unfitted_items`。
- 刀缝双向检查：任一侧膨胀后仍相交即判间隙不足。
- `BlaContext` 提供上下文入口。

### domain/block_loading

- `Space2`（enabled / split_x / split_y / is_degenerate / remaining_height_from）与 `Block2`。
- `SimpleBlockGenerator`（整行重复铺满）、`ComplexBlockGenerator`（按行拼接异构块）。
- `DepthFirstSearchAlgorithm2D`、`MultiRowHeuristicAlgorithm2D` 与 `SpaceSplit2Result`。
- `require_rotation_allowed` 旋转守卫。

### domain/layer_generation

- `Bpp2dLayerGenerationRequest` / `Bpp2dLayerGenerationResult` / `Bpp2dLayerGenerator`。
- `order_candidate_sheets`：主板材优先、按类型去重。
- 生成器：块装载局部/全局、空间切分、BLA、图样、圆形网格、混合形状、历史回放。
- `LayerGenerationContext`：逐板材类型展开、按物品形状筛选生成器、按上限截断、按面积排序。
- `CircleGridGenerator`：直角网格与六角网格，行距 `26/30·d`。

### domain/layer_assignment

- `LayerAggregation` 按列身份去重。
- 需求项、`layer_demand_coefficient`、`Load`（RMP 松弛 / 最终整数）与覆盖累计。
- 检验数定价：`layer_reduced_cost` / `is_improving_column` / `filter_improving_columns`。
- 目标分层：`sheet_area_terms` / `sheet_amount_terms` / `sheet_type_cost_terms` 与
  `LayerAssignmentObjectiveConfig`。
- 余料统计 `layer_waste_area` / `layer_waste_ratio`。
- `max_copies_per_column`、`SheetCapacityConstraint`、`ColumnSelection` 与选择统计。
- `LayerAssignmentContext` 覆盖注册、加列、退役、影子价格刷新、最终注册与目标交付五阶段。

### domain/packing

- `PackingGeometryGuard`：板材内 → 旋转合法 → 两两不重叠；刀缝双向膨胀。
- `PackingRotationRule`：AllowAll / SymmetricOnly / ByItemCatalog。
- `FreeSpaceAnalyzer` 与 `Remnant2`：水平切片、互不重叠、面积降序。
- `LoadingOrderCalculator`：先下后上、先左后右、稳定排序。
- `Packer`、`PackedItem` / `PackedLayer` / `PackedSheet`、`MaterialSummary`、`PackingResult`。
- `PackingRendererAdapter` 输出渲染 DTO。

### application

- `ColumnGenerationConfig` / `ColumnGenerationState` / `ColumnGenerationResult` 与失败阶段。
- `ColumnGenerationAlgorithm`：RMP → 取对偶 → 生成候选 → 加列 → 重复 → 最终 MILP。
- `ColumnGenerationStandardExecutorConfig`、`StandardRmpSolver`、`StandardFinalSolver`、
  `ColumnGenerationLayerRequestBuilder`。
- `TailSheetOptimizer` 与 `TailSheetEvaluation`。
- `CsvDatasetLoader`：`grouped-layer` 与 `material-width-amount` 两种 schema，含 8 类门禁。
- `ColumnGenerationPackingAnalyzer` 与 `ColumnGenerationApplicationService`。

## 与 Kotlin 的有意差异

1. **旋转候选收紧**：简单块/复杂块/DFS/多行的旋转候选要求 `config.with_rotation` 与
   `item.allow_rotate` 同时成立。Kotlin 只检查前者，会让未允许旋转的物品产出旋转块，
   与 `requireRotationAllowed` 的守卫语义自相矛盾。
2. **六角网格行距修正**：Kotlin 写成「26d / 30d」，得到无量纲的 0.8667，只有 d=1 时碰巧正确。
   Rust 改为缩放节距量本身，得到 `0.8667·d`。修正前在 10×8 板上相邻圆心距仅 1.32（直径 2），
   实际产出重叠圆。
3. **候选上限语义**：`max_candidates` 限制的是产出图案数而非行数；行仍须铺满整个料域。
4. **空列份数上界**：空列固定为 1。用「需求 / 1」会给出与需求等量的上界，让不装货的列反复出现。
5. **最终 MILP 空解**：提取不到业务产出时是硬失败，不被 `allow_partial_solution` 放宽成 Partial。

## 未完成事项

1. 求解器接入：`StandardRmpSolver` / `StandardFinalSolver` 的求解口径由注入函数决定；LP 终态契约
   （`Bpp2dLpTerminalStatus` 四态 + `require_pricing_ready`）已就位，但尚未提供 Gurobi/SCIP 的现成
   适配与 feature-gated 集成测试。
2. CSV 数据集只实现解析与门禁，尚未提供「数据集 → 问题」的物化路径。
3. `LayerAssignmentContext` 的退役只做聚合侧移除，模型侧变量固定为 0 由应用层决定，尚无端到端用例。
4. 图样生成器的 `pattern-narrow` 变体目前与整板变体放置相同，尚未真正减少列数。
5. 混合形状生成器只尝试「一行矩形 + 上方圆网格」一种组合，未覆盖多行混合。
6. 应用层尚无真实求解器驱动的端到端回归；当前测试使用假求解器。

## 验收结果

- `RUSTC_BOOTSTRAP=1 cargo test -p ospf-rust-framework-bpp2d`：145 条通过。
- `RUSTC_BOOTSTRAP=1 cargo test --workspace`：零失败，22 条因外部服务或求解器条件跳过（既有门控）。
