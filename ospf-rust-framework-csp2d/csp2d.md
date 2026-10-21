# CSP2D 迁移交接

## 当前状态

`ospf-rust-framework-csp2d` 已按 Kotlin 的八个子模块边界落地为单 crate 多模块：infrastructure、
material、pattern-generation、produce、yield、wasting-minimization 与 application
（sheet-assignment 的能力并入 material 与 produce）。

二维几何来自共享 crate `ospf-rust-framework-geometry2d`，与 BPP2D 共用同一实现。

`cargo test -p ospf-rust-framework-csp2d` 通过，共 82 条契约测试。

## 已完成事项

### infrastructure

- 错误类型 `Csp2dValidationError` / `Csp2dSolvingError` / `Csp2dCapabilityError` /
  `Csp2dInternalError`。
- `Region2`：面积、边界、退化判定、包含关系、`to_box`。
- 刀缝语义：`cut` 把区域切成两块并**切除**刀缝带，右/上块起点为 `position + kerf`；
  零刀缝退化为普通切分；任一侧退化返回 `Err`。
- `CutOrientation2` / `CutLine2` / `CutNode2` / `Leaf2` / `CutTree2`：断头台方案显式建模为切分树。
- `CutTree2::validate`：拒绝子刀线层号错误、超出工艺级数上限、子树区域超出父级切分区域。
- `CuttingProcess2`：Freeform / GuillotineTwoStage / GuillotineThreeStage。

### domain/material

- 标识类型：`ProductId2` / `MaterialId2` / `MachineId2` / `CostarId2` / `PatternId2` / `PlanId2`。
- `Material2` 双模式：`height` 为空即卷材，否则定尺板材；`region(segment_height)` 在卷材未给
  段高时返回 `None`。
- `Machine2`、`Costar2`。
- `Product2`（面积、校验、方向集合、`size_of`）、`DemandValue2` / `DemandMode2`、
  `ProductDemand2` / `ProductDemands2`。
- `Pattern2`：料域 + 成品 + 副产物 + 可选切分树 + 工艺；面积口径（成品/余料/利用率）、
  `product_amounts`、`validate`、`identity_key`。
- `CuttingPlan2`：二维方案是「一块料上的图案」，提供 `consumed_area` 与卷材 `consumed_height`。
- `remnant_reaches`、`fits_in`。

### domain/pattern_generation

- `PatternGenerationRequest2`：物料 + 产品及剩余件数 + 刀缝 + 工艺 + 副产物 + 卷材最大段高 +
  候选上限 + 历史图案 + 影子价格 + 余料权重；`validate` 五项门禁。
- `region_height_bound`：定尺取板高；卷材按「件高 + 刀缝」累加并按最大段高截断。
- `ShelfPatternGenerator2`：逐行推进、刀缝节距、前缀候选、候选上限保留末尾。
- 断头台切分树：按层级递增的平衡二叉树，级数等于层级；所需深度超过工艺上限时放弃构造。
- `PatternGenerator2` trait 与 `shelf_pattern` 便捷构造。

### domain/produce

- `Csp2dMasterDemand`（下界必须覆盖 + 可选业务上界）。
- `Csp2dPatternAssignment`：图案使用量、使用次数上界、覆盖系数（件数/面积）、消耗量
  （面积/段高）、余料量、覆盖判定与取整。
- `Csp2dMasterProblem`：物料用量与可用量、余料/面积/段高三类目标项、覆盖率。
- `residual_products`：由「需求 − 覆盖」推导下一轮生成的产品列表。
- `Csp2dIterationRecord`。

### domain/yield 与 wasting_minimization

- `YieldContext`：欠产与超产分析、良率比、超产上界校验。
- `WasteContext`：逐方案余料面积与率、总余料面积、余料惩罚目标项、`pattern_waste_ratio`。
- 注意 `yield` 是 Rust 保留字，模块以 `r#yield` 声明。

### application

- `Csp2dProblem` / `Csp2dConfiguration`：物料、产品、需求、设备、副产物、刀缝、工艺与配置；
  面积需求按单件面积折算件数。
- `Csp2dSolvers`：MILP 路径与列生成路径共用装配。
- `Csp2dSolution` / `Csp2dTerminationReason`。
- 工艺与切分树不一致的图案在进入主问题前被丢弃。

## 与 Kotlin 的有意差异

1. **切分树构造**：Kotlin 未提供从货架式排样自动生成切分树的实现；Rust 补齐了按层级递增的
   平衡树，使两级工艺的图案可以直接通过校验。
2. **列生成驱动**：Kotlin 明确本阶段由剩余需求驱动而非检验数；Rust 保持同一口径，并在代码与
   README 中显式声明未实现 reduced-cost pricing。
3. **空方案使用上界**：空图案固定为 1，避免「需求 / 1」给出与需求等量的上界。
4. **覆盖判定取整**：先四舍五入再比较，与最终整数解口径一致，避免浮点噪声导致的假阴性。

## 已完成（第二轮）

1. **检验数定价**（`domain/produce/pricing.rs`）：`rc = 消耗量 × 单价 − Σ 覆盖系数 × π`，
   支持面积/段高两种计价、改进列筛选（升序）、未入列图案估算。列生成循环已由
   「剩余需求驱动」升级为「剩余需求生成候选 + 检验数筛选」。
   注意：影子价格目前由覆盖缺口**估计**，不是 LP 对偶。
2. **板料分配**（`domain/sheet_assignment`）：定尺按张数、卷材按段高的分配口径独立成模块，
   含利用率、余料率、可用量校验与按编号查询。
3. **三级断头台**：按「条 → 块 → 件」构造，行切第 1 级、块切第 2 级、竖切第 3 级；
   级数随刀向耦合，叶子携带 pieceId。
4. **副产物填充**（`pattern_generation/costar_filler.rs`）：面积降序贪心、刀缝双侧膨胀、
   不改变成品覆盖量；生成器在请求带 costars 时自动填充。
5. **可注入主问题求解接口**（`application/master_solver.rs`）：四态终态、只有最优允许交付
   使用次数、内置启发式与注入求解器可切换，非最优或维度不符时回退启发式。
6. **`maxCandidates` 语义修正**：真正限制产出图案数而非行数，并产出前缀候选。

## 未完成事项

1. **真实 LP 对偶**：影子价格仍是覆盖缺口估计，未接真实 LP 对偶。这是 reduced-cost
   pricing 的最后一环，需要 LP 后端与对偶提取（core 侧尚未暴露对偶 API）。
2. **真实求解器**：接口已就位（`Csp2dMasterSolver`），但尚未提供 Gurobi/SCIP 的现成实现。
3. **卷材长度推导**：段高口径已可用，但「按需求反推最优卷材长度」尚未实现。
4. **CSV 数据集**：CSP2D 侧未提供数据集加载器（BPP2D 侧已有）。
5. **副产物旋转**：填充器刻意不做旋转（纹理方向属业务决策），未提供可配置的旋转许可。

## 验收结果

- `RUSTC_BOOTSTRAP=1 cargo test -p ospf-rust-framework-csp2d`：122 条通过。
- `RUSTC_BOOTSTRAP=1 cargo test --workspace`：零失败，22 条因外部服务或求解器条件跳过（既有门控）。
