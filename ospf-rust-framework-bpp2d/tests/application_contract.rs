//! 应用层契约测试 / Application layer contract tests
//!
//! 对应 Kotlin `Bpp2dPricingAndDatasetTest` 与列生成编排契约：配置默认值、列去重、
//! RMP → 定价 → 加列 → 最终 MILP 循环、无求解器时的退化行为、尾板优化、
//! CSV schema 门禁与端到端装配。
//! Mirrors Kotlin `Bpp2dPricingAndDatasetTest` and the column-generation orchestration contract: config
//! defaults, column deduplication, the RMP → pricing → add-column → final-MILP loop, the degenerate behaviour
//! without a solver, tail sheet optimisation, CSV schema gates and the end-to-end assembly.

use std::collections::BTreeMap;

use ospf_rust_framework_bpp2d::application::{
    ColumnGenerationAlgorithm, ColumnGenerationApplicationService, ColumnGenerationConfig,
    ColumnGenerationFailureStage, ColumnGenerationFinalSolver, ColumnGenerationInput,
    ColumnGenerationPackingAnalyzer, ColumnGenerationRmpSolver,
    ColumnGenerationStandardExecutorConfig, CsvDatasetLoader, CsvDatasetSchema, CsvShapeType,
    TailSheetOptimizer, deduplicate_layers,
};
use ospf_rust_framework_bpp2d::application::service::ColumnGenerationLayerRequestBuilder;
use ospf_rust_framework_bpp2d::domain::item::model::{
    ActualItem, Bpp2dDemandMode, ItemId, Layer, SheetType, SheetTypeId,
};
use ospf_rust_framework_bpp2d::domain::layer_assignment::{
    Bpp2dAssignmentDemandEntry, Bpp2dDemandShadowPriceKey,
};
use ospf_rust_framework_bpp2d::domain::layer_generation::{
    Bpp2dLayerGenerationRequest, Bpp2dLayerGenerationResult, Bpp2dLayerGenerator,
    LayerGenerationContext,
};
use ospf_rust_framework_geometry2d::{
    PackingShape2, Placement2, QuantityPoint2, Sheet2Shape, rectangle_shape2,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;

/// 层分配需求模式 → 层生成需求模式 / Layer-assignment demand mode → layer-generation demand mode
fn mode_of(entry: &Bpp2dAssignmentDemandEntry) -> Bpp2dDemandMode {
    match entry.mode {
        ospf_rust_framework_bpp2d::domain::layer_assignment::Bpp2dAssignmentDemandMode::ItemAmount => {
            Bpp2dDemandMode::ItemAmount
        }
        ospf_rust_framework_bpp2d::domain::layer_assignment::Bpp2dAssignmentDemandMode::ItemArea => {
            Bpp2dDemandMode::ItemArea
        }
    }
}

fn meters(value: f64) -> Quantity<f64, Meter> {
    Quantity::new_ct(value)
}

fn sheet_type() -> SheetType<f64, Meter> {
    SheetType::new(meters(10.0), meters(8.0), meters(100.0), SheetTypeId::new("S1"))
}

fn item(id: &str, width: f64, height: f64) -> ActualItem<f64, Meter> {
    ActualItem::new(
        ItemId::new(id),
        id,
        PackingShape2::Rectangle(rectangle_shape2(meters(width), meters(height)).expect("rect")),
        true,
        meters(1.0),
    )
}

fn layer(from: &str, amount: usize) -> Layer<f64, Meter> {
    let shape = PackingShape2::Rectangle(rectangle_shape2(meters(2.0), meters(2.0)).expect("rect"));
    let units = (0..amount)
        .map(|index| {
            Placement2::new(shape.clone(), QuantityPoint2 {
                x: meters(index as f64 * 2.0),
                y: meters(0.0),
            })
            .with_item_id("a")
        })
        .collect();
    Layer::new(0, from, Some(sheet_type()), Sheet2Shape::new(meters(10.0), meters(8.0)))
        .with_units(units)
}

// ============================================================================
// 配置 / Config
// ============================================================================

/// 列生成配置默认值 / Column generation config defaults
#[test]
fn config_defaults() {
    let config = ColumnGenerationConfig::default();
    assert_eq!(config.iteration_limit, 50);
    assert_eq!(config.max_columns_per_iteration, 256);
    assert_eq!(config.time_limit_ms, 0);
    assert!(config.final_milp_enabled);
    assert_eq!(config.reduced_cost_tolerance, 1e-6);
}

/// 执行器配置默认值与 BPP2D 口径一致 / Executor config defaults match the BPP2D convention
#[test]
fn executor_config_defaults() {
    let config = ColumnGenerationStandardExecutorConfig::default();
    assert_eq!(config.sheet_area_weight, 1.0);
    assert_eq!(config.sheet_amount_weight, 1e-6);
    assert_eq!(config.sheet_type_cost_weight, 0.0);
    assert_eq!(config.rmp_area_coefficient, 1.0);
    assert!(config.sheet_type_costs.is_empty());
}

/// 执行器配置可转换为层分配目标配置 / The executor config converts into a layer-assignment objective config
#[test]
fn executor_config_converts_to_objective_config() {
    let config = ColumnGenerationStandardExecutorConfig::default();
    let objective = config.to_objective_config();
    assert_eq!(objective.sheet_area_weight, 1.0);
    assert_eq!(objective.sheet_amount_weight, 1e-6);
    assert_eq!(objective.rmp_area_coefficient, 1.0);
}

// ============================================================================
// 列去重 / Column deduplication
// ============================================================================

/// 列去重按身份键 / Columns are deduplicated by identity key
#[test]
fn deduplicate_by_identity() {
    let first = layer("pattern", 3);
    let second = layer("pattern", 3);
    let third = layer("bla", 3);

    let deduped = deduplicate_layers(vec![first, second, third]);
    assert_eq!(deduped.len(), 2);
}

// ============================================================================
// 列生成循环 / Column generation loop
// ============================================================================

/// 简单 RMP 求解器：返回固定对偶 / A simple RMP solver returning fixed duals
#[derive(Debug)]
struct FixedDualSolver {
    dual: f64,
    objective: f64,
}

impl ColumnGenerationRmpSolver for FixedDualSolver {
    fn solve(
        &self,
        state: &ospf_rust_framework_bpp2d::application::ColumnGenerationState<f64, Meter>,
        demand_entries: &[Bpp2dAssignmentDemandEntry],
    ) -> ospf_rust_base::error::Ret<
        ospf_rust_framework_bpp2d::application::ColumnGenerationLpResult,
    > {
        // 层分配需求项 → 层生成需求模式键：两者都按「模式 + 物品」索引。
        // Layer-assignment demand entry → layer-generation demand mode key: both index by mode plus item.
        let mut shadow_prices = BTreeMap::new();
        for entry in demand_entries {
            shadow_prices.insert(
                ospf_rust_framework_bpp2d::domain::item::model::DemandModeKey::new(
                    mode_of(entry),
                    entry.item_id.as_str(),
                ),
                self.dual,
            );
        }
        Ok(ospf_rust_framework_bpp2d::application::ColumnGenerationLpResult {
            status: ospf_rust_framework_bpp2d::application::Bpp2dLpTerminalStatus::Optimal,
            shadow_prices,
            objective: Some(self.objective),
            info: BTreeMap::from([(
                "columns".to_string(),
                state.columns.len().to_string(),
            )]),
            additional_shadow_prices: BTreeMap::new(),
        })
    }
}

/// 简单最终求解器：每列 1 份 / A simple final solver taking one copy per column
#[derive(Debug)]
struct OneCopySolver;

impl ColumnGenerationFinalSolver for OneCopySolver {
    fn solve(
        &self,
        state: &ospf_rust_framework_bpp2d::application::ColumnGenerationState<f64, Meter>,
        _demand_entries: &[Bpp2dAssignmentDemandEntry],
    ) -> ospf_rust_base::error::Ret<
        ospf_rust_framework_bpp2d::application::ColumnGenerationFinalResult<f64, Meter>,
    > {
        let copies: Vec<f64> = state.columns.iter().map(|_| 1.0).collect();
        let selection =
            ospf_rust_framework_bpp2d::domain::layer_assignment::select_columns_by_copies(
                &state.columns,
                &copies,
            );
        Ok(
            ospf_rust_framework_bpp2d::application::ColumnGenerationFinalResult {
                columns: selection.layers.clone(),
                sheets: Vec::new(),
                objective: Some(copies.len() as f64),
                info: BTreeMap::new(),
            },
        )
    }
}

/// 只生成一次候选的生成器 / A generator producing candidates once
#[derive(Debug)]
struct OnceGenerator;

impl Bpp2dLayerGenerator<f64, Meter> for OnceGenerator {
    fn name(&self) -> &str {
        "once"
    }

    fn generate(
        &self,
        request: &Bpp2dLayerGenerationRequest<f64, Meter>,
    ) -> Vec<Bpp2dLayerGenerationResult<f64, Meter>> {
        if !request.existing_layers.is_empty() {
            return Vec::new();
        }
        vec![Bpp2dLayerGenerationResult::new(
            layer("pattern", 5),
            "pattern",
        )]
    }
}

/// 空生成器 / An empty generator
#[derive(Debug)]
struct EmptyGenerator;

impl Bpp2dLayerGenerator<f64, Meter> for EmptyGenerator {
    fn name(&self) -> &str {
        "empty"
    }

    fn generate(
        &self,
        _request: &Bpp2dLayerGenerationRequest<f64, Meter>,
    ) -> Vec<Bpp2dLayerGenerationResult<f64, Meter>> {
        Vec::new()
    }
}

fn request() -> Bpp2dLayerGenerationRequest<f64, Meter> {
    Bpp2dLayerGenerationRequest::new(0, vec![(item("a", 2.0, 2.0), 20)])
        .with_sheet(sheet_type())
        .with_max_candidates(64)
}

/// 循环在无新列时终止 / The loop stops when no new column appears
#[test]
fn loop_stops_without_new_columns() {
    let algorithm = ColumnGenerationAlgorithm::new(
        Box::new(OnceGenerator),
        vec![Bpp2dAssignmentDemandEntry::item_amount("a", 20)],
    )
    .with_rmp_solver(Box::new(FixedDualSolver {
        dual: 5.0,
        objective: 1.0,
    }));

    let result = algorithm
        .solve(request(), &ColumnGenerationConfig::default())
        .expect("solve");

    // 第一轮加 1 列；第二轮先解一次 RMP，再发现生成器返回空列而停止。
    // 因此迭代计数为 1，但 LP 被求解两次 —— 这正是列生成的标准顺序（先解 RMP 才能定价）。
    // Round one adds a column; round two solves the RMP once more before discovering that the generator
    // returns nothing and stops. The iteration count is therefore 1 while the LP was solved twice, which is
    // exactly the standard column-generation order (the RMP must be solved before pricing).
    assert_eq!(result.iteration_count, 1);
    assert_eq!(result.lp_solved_times, 2);
    assert_eq!(result.column_count(), 1);
    assert!(!result.terminated_by_iteration_limit);
}

/// 未配置 RMP 求解器时循环不执行 / The loop does not run without an RMP solver
#[test]
fn loop_skips_without_rmp_solver() {
    let algorithm = ColumnGenerationAlgorithm::new(
        Box::new(OnceGenerator),
        vec![Bpp2dAssignmentDemandEntry::item_amount("a", 20)],
    );

    let result = algorithm
        .solve(request(), &ColumnGenerationConfig::default())
        .expect("solve");

    assert_eq!(result.iteration_count, 0);
    assert_eq!(result.lp_solved_times, 0);
    assert!(result.columns.is_empty());
}

/// 到达迭代上限时标记 / Reaching the iteration limit is flagged
#[test]
fn iteration_limit_is_flagged() {
    /// 每轮都产出新列的生成器 / A generator producing a new column every round
    #[derive(Debug)]
    struct AlwaysNew;

    impl Bpp2dLayerGenerator<f64, Meter> for AlwaysNew {
        fn name(&self) -> &str {
            "always-new"
        }

        fn generate(
            &self,
            request: &Bpp2dLayerGenerationRequest<f64, Meter>,
        ) -> Vec<Bpp2dLayerGenerationResult<f64, Meter>> {
            // 用轮次区分来源，保证每轮都是新列。
            // Use the round to vary the source so every round yields a new column.
            vec![Bpp2dLayerGenerationResult::new(
                layer(&format!("round-{}", request.iteration), 5),
                "always-new",
            )]
        }
    }

    let algorithm = ColumnGenerationAlgorithm::new(
        Box::new(AlwaysNew),
        vec![Bpp2dAssignmentDemandEntry::item_amount("a", 20)],
    )
    .with_rmp_solver(Box::new(FixedDualSolver {
        dual: 5.0,
        objective: 1.0,
    }));

    let result = algorithm
        .solve(
            request(),
            &ColumnGenerationConfig::default().with_iteration_limit(3),
        )
        .expect("solve");

    assert_eq!(result.iteration_count, 3);
    assert!(result.terminated_by_iteration_limit);
}

/// 最终 MILP 被调用并回填板材 / The final MILP runs and fills the sheets
#[test]
fn final_milp_runs() {
    let algorithm = ColumnGenerationAlgorithm::new(
        Box::new(OnceGenerator),
        vec![Bpp2dAssignmentDemandEntry::item_amount("a", 20)],
    )
    .with_rmp_solver(Box::new(FixedDualSolver {
        dual: 5.0,
        objective: 1.0,
    }))
    .with_final_solver(Box::new(OneCopySolver));

    let result = algorithm
        .solve(request(), &ColumnGenerationConfig::default())
        .expect("solve");

    assert!(result.final_solved);
    assert_eq!(result.column_count(), 1);
}

/// 关闭最终 MILP 时不调用最终求解器 / The final solver is skipped when disabled
#[test]
fn final_milp_can_be_disabled() {
    let mut config = ColumnGenerationConfig::default();
    config.final_milp_enabled = false;
    let algorithm = ColumnGenerationAlgorithm::new(
        Box::new(OnceGenerator),
        vec![Bpp2dAssignmentDemandEntry::item_amount("a", 20)],
    )
    .with_rmp_solver(Box::new(FixedDualSolver {
        dual: 5.0,
        objective: 1.0,
    }))
    .with_final_solver(Box::new(OneCopySolver));

    let result = algorithm.solve(request(), &config).expect("solve");
    assert!(!result.final_solved);
}

// ============================================================================
// 尾板优化 / Tail sheet optimisation
// ============================================================================

/// 尾板优先选覆盖更多剩余需求的列 / The tail column prefers more covered residual demand
#[test]
fn tail_prefers_more_coverage() {
    let optimizer = TailSheetOptimizer::new();
    let mut demands = BTreeMap::new();
    demands.insert(ItemId::new("a"), 5u64);

    let small = layer("small", 2);
    let large = layer("large", 4);

    let selected = optimizer
        .select_tail(&[small, large.clone()], &demands)
        .expect("tail");
    assert_eq!(selected.identity_key(), large.identity_key());
}

/// 覆盖相同时优先超出更少的列 / With equal coverage the smaller overshoot wins
#[test]
fn tail_prefers_less_waste() {
    let optimizer = TailSheetOptimizer::new();
    let mut demands = BTreeMap::new();
    demands.insert(ItemId::new("a"), 2u64);

    let exact = layer("exact", 2);
    let over = layer("over", 4);

    let selected = optimizer
        .select_tail(&[over, exact.clone()], &demands)
        .expect("tail");
    assert_eq!(selected.identity_key(), exact.identity_key());
}

/// 裁剪按剩余需求逐件保留 / Trimming keeps pieces up to the residual demand
#[test]
fn trim_respects_residual_demand() {
    let optimizer = TailSheetOptimizer::new();
    let candidate = layer("candidate", 4);
    let mut remaining = BTreeMap::new();
    remaining.insert(ItemId::new("a"), 2u64);

    let trimmed = optimizer.trim_layer(&candidate, &mut remaining);
    assert_eq!(trimmed.units.len(), 2);
    assert_eq!(remaining.get(&ItemId::new("a")), Some(&0));
}

/// 空需求时裁剪掉全部件 / An empty residual demand trims everything
#[test]
fn trim_with_empty_demand_drops_all() {
    let optimizer = TailSheetOptimizer::new();
    let candidate = layer("candidate", 3);
    let mut remaining = BTreeMap::new();

    let trimmed = optimizer.trim_layer(&candidate, &mut remaining);
    assert!(trimmed.units.is_empty());
}

// ============================================================================
// CSV 数据集 / CSV datasets
// ============================================================================

/// grouped-layer schema 正常解析 / The grouped-layer schema parses
#[test]
fn csv_grouped_layer_parses() {
    let text = "item_id,width_meter,height_meter,amount,allow_rotate\na,2,2,5,true\nb,3,1,2,false\n";
    let records = CsvDatasetLoader::new()
        .load_str(text, CsvDatasetSchema::GroupedLayer)
        .expect("parse");

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].item_id, "a");
    assert_eq!(records[0].amount, 5);
    assert!(records[0].allow_rotate);
    assert_eq!(records[0].shape_type, CsvShapeType::Rectangle);
    assert!(!records[1].allow_rotate);
}

/// 圆形件可由半径给出 / A circle may specify its radius
#[test]
fn csv_circle_from_radius() {
    let text = "item_id,width_meter,radius_meter,amount\nc,2,1,4\n";
    let records = CsvDatasetLoader::new()
        .load_str(text, CsvDatasetSchema::GroupedLayer)
        .expect("parse");

    assert_eq!(records[0].shape_type, CsvShapeType::Circle);
    assert_eq!(records[0].radius, Some(1.0));
}

/// material-width-amount schema 要求板材尺寸 / The material-width-amount schema requires sheet sizes
#[test]
fn csv_material_width_amount_requires_sheet_size() {
    let text = "item_id,width_meter,amount,sheet_width_meter,sheet_height_meter\na,2,5,10,8\n";
    let records = CsvDatasetLoader::new()
        .load_str(text, CsvDatasetSchema::MaterialWidthAmount)
        .expect("parse");
    assert_eq!(records[0].sheet_width, Some(10.0));

    let missing = "item_id,width_meter,amount\na,2,5\n";
    assert!(
        CsvDatasetLoader::new()
            .load_str(missing, CsvDatasetSchema::MaterialWidthAmount)
            .is_err()
    );
}

/// 重复列被拒绝 / Duplicate columns are rejected
#[test]
fn csv_rejects_duplicate_columns() {
    let text = "item_id,width_meter,width_meter,amount\na,2,2,5\n";
    let error = CsvDatasetLoader::new()
        .load_str(text, CsvDatasetSchema::GroupedLayer)
        .expect_err("must be rejected");
    assert!(error.to_string().contains("重复") || error.to_string().contains("repeat"));
}

/// 未知列被拒绝 / Unknown columns are rejected
#[test]
fn csv_rejects_unknown_columns() {
    let text = "item_id,width_meter,amount,unknown_col\na,2,5,1\n";
    let error = CsvDatasetLoader::new()
        .load_str(text, CsvDatasetSchema::GroupedLayer)
        .expect_err("must be rejected");
    assert!(error.to_string().contains("未知") || error.to_string().contains("unknown"));
}

/// 缺必填列被拒绝 / Missing required columns are rejected
#[test]
fn csv_rejects_missing_required_columns() {
    let text = "item_id,amount\na,5\n";
    let error = CsvDatasetLoader::new()
        .load_str(text, CsvDatasetSchema::GroupedLayer)
        .expect_err("must be rejected");
    assert!(error.to_string().contains("必填") || error.to_string().contains("required"));
}

/// 非法枚举被拒绝 / Illegal enums are rejected
#[test]
fn csv_rejects_illegal_enums() {
    let text = "item_id,width_meter,height_meter,amount,shape_type\na,2,2,5,triangle\n";
    let error = CsvDatasetLoader::new()
        .load_str(text, CsvDatasetSchema::GroupedLayer)
        .expect_err("must be rejected");
    assert!(error.to_string().contains("shape_type"));
}

/// 非正数被拒绝 / Non-positive numbers are rejected
#[test]
fn csv_rejects_non_positive_numbers() {
    let text = "item_id,width_meter,height_meter,amount\na,0,2,5\n";
    assert!(
        CsvDatasetLoader::new()
            .load_str(text, CsvDatasetSchema::GroupedLayer)
            .is_err()
    );
}

/// 行内列数不匹配被拒绝 / Mismatched field counts are rejected
#[test]
fn csv_rejects_field_count_mismatch() {
    let text = "item_id,width_meter,height_meter,amount\na,2,2\n";
    let error = CsvDatasetLoader::new()
        .load_str(text, CsvDatasetSchema::GroupedLayer)
        .expect_err("must be rejected");
    assert!(error.to_string().contains("列") || error.to_string().contains("fields"));
}

/// 同物品几何不一致被拒绝 / Inconsistent geometry for one item is rejected
#[test]
fn csv_rejects_inconsistent_item_geometry() {
    let text = "item_id,width_meter,height_meter,amount\na,2,2,5\na,3,2,1\n";
    let error = CsvDatasetLoader::new()
        .load_str(text, CsvDatasetSchema::GroupedLayer)
        .expect_err("must be rejected");
    assert!(error.to_string().contains("几何") || error.to_string().contains("geometry"));
}

/// 空数据集被拒绝 / An empty dataset is rejected
#[test]
fn csv_rejects_empty_dataset() {
    assert!(
        CsvDatasetLoader::new()
            .load_str("", CsvDatasetSchema::GroupedLayer)
            .is_err()
    );
    assert!(
        CsvDatasetLoader::new()
            .load_str("item_id,width_meter,amount\n", CsvDatasetSchema::GroupedLayer)
            .is_err()
    );
}

// ============================================================================
// 端到端装配 / End-to-end assembly
// ============================================================================

/// 层请求构建器继承状态 / The request builder inherits the state
#[test]
fn request_builder_inherits_state() {
    let builder = ColumnGenerationLayerRequestBuilder {
        sheet: sheet_type(),
        sheets: vec![sheet_type()],
        items: vec![(item("a", 2.0, 2.0), 20)],
        demand_entries: vec![Bpp2dAssignmentDemandEntry::item_amount("a", 20)],
        kerf: None,
    };
    let mut state =
        ospf_rust_framework_bpp2d::application::ColumnGenerationState::<f64, Meter>::new();
    state.iteration = 3;
    state.columns = vec![layer("existing", 2)];
    state.shadow_prices.insert(
        ospf_rust_framework_bpp2d::domain::item::model::DemandModeKey::new(
            Bpp2dDemandMode::ItemAmount,
            "a",
        ),
        4.0,
    );

    let request = builder.build(&state, 32);
    assert_eq!(request.iteration, 3);
    assert_eq!(request.existing_layers.len(), 1);
    assert_eq!(request.max_candidates, 32);
    assert_eq!(request.shadow_prices.len(), 1);
}

/// 端到端服务在无求解器时仍返回结果 / The end-to-end service returns a result without solvers
#[test]
fn application_service_without_solvers() {
    let input = ColumnGenerationInput {
        sheet: sheet_type(),
        sheets: vec![sheet_type()],
        items: vec![(item("a", 2.0, 2.0), 20)],
        demands: vec![Bpp2dAssignmentDemandEntry::item_amount("a", 20)],
        kerf: None,
        sheet_shape: Sheet2Shape::new(meters(10.0), meters(8.0)),
        item_catalog: BTreeMap::new(),
    };

    let result = ColumnGenerationApplicationService::new()
        .solve(input, &ColumnGenerationConfig::default(), None, None)
        .expect("solve");

    // 无求解器时列生成为空，装箱空板材集合仍应成功。
    assert_eq!(result.iteration_count, 0);
    assert!(result.render.is_some());
}

/// 装箱分析器可由结果生成渲染 / The packing analyzer renders a result
#[test]
fn packing_analyzer_renders() {
    let analyzer = ColumnGenerationPackingAnalyzer::new();
    let algorithm = ColumnGenerationAlgorithm::new(
        Box::new(EmptyGenerator),
        vec![Bpp2dAssignmentDemandEntry::item_amount("a", 20)],
    );
    let result = algorithm
        .solve(request(), &ColumnGenerationConfig::default())
        .expect("solve");
    let packing = analyzer.analyze(&result, None).expect("packing");
    let dto = analyzer.render(&packing);

    assert!(dto.sheets.is_empty());
    assert_eq!(dto.total_amount, 0);
}

/// 失败阶段有稳定文本 / Failure stages expose stable text
#[test]
fn failure_stages_are_stable() {
    assert_eq!(
        ColumnGenerationFailureStage::InitialColumns.as_str(),
        "initial_columns"
    );
    assert_eq!(
        ColumnGenerationFailureStage::RestrictedMasterProblem.as_str(),
        "restricted_master_problem"
    );
    assert_eq!(
        ColumnGenerationFailureStage::CandidateFilter.as_str(),
        "candidate_filter"
    );
    assert_eq!(ColumnGenerationFailureStage::FinalMilp.as_str(), "final_milp");
    assert_eq!(
        ColumnGenerationFailureStage::SolutionAnalysis.as_str(),
        "solution_analysis"
    );
}

/// 默认生成器集合非空 / The default generator set is non-empty
#[test]
fn default_generators_are_available() {
    let generators = LayerGenerationContext::default_generators::<f64, Meter>();
    assert_eq!(generators.len(), 8);
}

// ============================================================================
// LP 终态契约 / LP terminal-state contract
// ============================================================================

/// 只有最优允许交付定价结果 / Only optimal may deliver a pricing result
#[test]
fn lp_terminal_status_allows_pricing_only_when_optimal() {
    use ospf_rust_framework_bpp2d::application::{Bpp2dLpTerminalStatus, require_pricing_ready};

    assert!(Bpp2dLpTerminalStatus::Optimal.allows_pricing());
    assert!(require_pricing_ready(Bpp2dLpTerminalStatus::Optimal).is_ok());

    for status in [
        Bpp2dLpTerminalStatus::Infeasible,
        Bpp2dLpTerminalStatus::Unbounded,
        Bpp2dLpTerminalStatus::Other,
    ] {
        assert!(!status.allows_pricing(), "{status:?} must not allow pricing");
        let error = require_pricing_ready(status).expect_err("must be rejected");
        assert!(
            error.to_string().contains("终态") || error.to_string().contains("terminal"),
            "{error}"
        );
    }
}

/// LP 结果默认终态为最优以兼容启发式注入 / The LP result defaults to optimal for heuristic injection
#[test]
fn lp_result_defaults_to_optimal() {
    use ospf_rust_framework_bpp2d::application::{
        Bpp2dLpTerminalStatus, ColumnGenerationLpResult,
    };

    let result = ColumnGenerationLpResult::default();
    assert_eq!(result.status, Bpp2dLpTerminalStatus::Optimal);
    assert!(result.status.is_optimal());
}
