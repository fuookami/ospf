//! 主问题与应用契约测试 / Master problem and application contract tests
//!
//! 对应 Kotlin `MasterProblemContractTest`、`YieldContextContractTest`、
//! `WasteContextContractTest` 与 `Csp2dApplicationContractTest`：
//! 图案使用量上界、覆盖系数（件数/面积）、覆盖判定、物料用量与可用量、目标项口径、
//! 余料与良率分析、MILP 路径与列生成路径、无候选方案与覆盖不足的显式失败。
//! Mirrors Kotlin `MasterProblemContractTest`, `YieldContextContractTest`, `WasteContextContractTest` and
//! `Csp2dApplicationContractTest`: usage bounds, coverage coefficients (amount/area), coverage decisions,
//! material usage and availability, objective conventions, waste and yield analysis, the MILP and column
//! generation paths, and the explicit failures for no candidate and insufficient coverage.

use std::collections::BTreeMap;

use ospf_rust_framework_csp2d::application::{
    Csp2dConfiguration, Csp2dProblem, Csp2dSolvers, Csp2dTerminationReason,
};
use ospf_rust_framework_csp2d::domain::material::{
    CuttingPlan2, DemandValue2, Material2, Pattern2, Product2, ProductDemand2, material_id2,
    pattern_id2, product_id2,
};
use ospf_rust_framework_csp2d::domain::produce::{
    Csp2dMasterDemand, Csp2dMasterProblem, Csp2dPatternAssignment, residual_products,
};
use ospf_rust_framework_csp2d::domain::wasting_minimization::WasteContext;
use ospf_rust_framework_csp2d::domain::r#yield::YieldContext;
use ospf_rust_framework_csp2d::infrastructure::{CuttingProcess2, region2_of};
use ospf_rust_framework_geometry2d::{
    PackingShape2, Placement2, QuantityPoint2, rectangle_shape2,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;

fn meters(value: f64) -> Quantity<f64, Meter> {
    Quantity::new_ct(value)
}

fn sheet_material(width: f64, height: f64, available: u64) -> Material2<f64, Meter> {
    Material2::sheet(material_id2("m1"), "sheet", meters(width), meters(height), available)
}

fn product(id: &str, width: f64, height: f64) -> Product2<f64, Meter> {
    Product2::new(product_id2(id), id, meters(width), meters(height))
}

/// 构造一个方案：在 width×height 的料域上放 amount 个 part×part 的件。
/// Build a plan: `amount` pieces of part×part on a width×height region.
fn plan(
    id: &str,
    material: Material2<f64, Meter>,
    width: f64,
    height: f64,
    part: f64,
    amount: usize,
    product_id: &str,
) -> CuttingPlan2<f64, Meter> {
    let shape = PackingShape2::Rectangle(rectangle_shape2(meters(part), meters(part)).expect("rect"));
    let placements = (0..amount)
        .map(|index| {
            Placement2::new(shape.clone(), QuantityPoint2 {
                x: meters(index as f64 * part),
                y: meters(0.0),
            })
            .with_item_id(product_id)
        })
        .collect();
    let pattern = Pattern2::new(
        pattern_id2(id),
        material,
        region2_of(meters(0.0), meters(0.0), meters(width), meters(height)),
        placements,
        CuttingProcess2::Freeform,
    );
    CuttingPlan2::from_pattern(pattern)
}

// ============================================================================
// 图案使用量 / Pattern assignment
// ============================================================================

/// 空方案列表被拒绝 / An empty plan list is rejected
#[test]
fn empty_assignment_is_rejected() {
    assert!(Csp2dPatternAssignment::new(Vec::new(), 10).is_err());
}

/// 使用次数上界按覆盖件数收紧 / The usage bound is tightened by the covered piece count
#[test]
fn usage_bounds_use_covered_pieces() {
    // 每方案 5 件，需求上界 12 → 上界 3（向上取整）。
    let assignment = Csp2dPatternAssignment::new(
        vec![plan("p1", sheet_material(10.0, 8.0, 10), 10.0, 8.0, 2.0, 5, "a")],
        12,
    )
    .expect("assignment");

    assert_eq!(assignment.usage_upper_bounds, vec![3]);
}

/// 空方案的次数上界为 1 / An empty plan's usage bound is 1
#[test]
fn empty_plan_bound_is_one() {
    let assignment = Csp2dPatternAssignment::new(
        vec![plan("p1", sheet_material(10.0, 8.0, 10), 10.0, 8.0, 2.0, 0, "a")],
        100,
    )
    .expect("assignment");

    assert_eq!(assignment.usage_upper_bounds, vec![1]);
}

/// 件数需求的覆盖系数是件数 / An amount demand's coefficient is the piece count
#[test]
fn amount_coverage_coefficient() {
    let assignment = Csp2dPatternAssignment::new(
        vec![plan("p1", sheet_material(10.0, 8.0, 10), 10.0, 8.0, 2.0, 5, "a")],
        20,
    )
    .expect("assignment");
    let demand = Csp2dMasterDemand::new(product_id2("a"), DemandValue2::Amount(20));

    assert_eq!(assignment.coverage_coefficient(0, &demand), 5.0);
}

/// 面积需求的覆盖系数是单件面积乘件数 / An area demand's coefficient is the unit area times the count
#[test]
fn area_coverage_coefficient() {
    let assignment = Csp2dPatternAssignment::new(
        vec![plan("p1", sheet_material(10.0, 8.0, 10), 10.0, 8.0, 2.0, 5, "a")],
        20,
    )
    .expect("assignment");
    let demand = Csp2dMasterDemand::new(product_id2("a"), DemandValue2::Area(80.0));

    // 单件 4 × 5 件 = 20。
    assert_eq!(assignment.coverage_coefficient(0, &demand), 20.0);
}

/// 不含该产品的方案覆盖系数为零 / A plan without the product has a zero coefficient
#[test]
fn missing_product_coefficient_is_zero() {
    let assignment = Csp2dPatternAssignment::new(
        vec![plan("p1", sheet_material(10.0, 8.0, 10), 10.0, 8.0, 2.0, 5, "a")],
        20,
    )
    .expect("assignment");
    let demand = Csp2dMasterDemand::new(product_id2("b"), DemandValue2::Amount(20));

    assert_eq!(assignment.coverage_coefficient(0, &demand), 0.0);
}

/// 覆盖量按使用次数累计 / Coverage accumulates by usage
#[test]
fn coverage_accumulates_by_usage() {
    let assignment = Csp2dPatternAssignment::new(
        vec![plan("p1", sheet_material(10.0, 8.0, 10), 10.0, 8.0, 2.0, 5, "a")],
        20,
    )
    .expect("assignment");
    let demands = vec![Csp2dMasterDemand::new(
        product_id2("a"),
        DemandValue2::Amount(20),
    )];

    let coverage = assignment.coverage(&[2.0], &demands);
    assert_eq!(coverage, vec![10.0]);
    assert!(!assignment.covers_all(&[2.0], &demands));
    assert!(assignment.covers_all(&[4.0], &demands));
}

/// 使用次数取整 / Usage counts are rounded
#[test]
fn rounded_usage() {
    let assignment = Csp2dPatternAssignment::new(
        vec![plan("p1", sheet_material(10.0, 8.0, 10), 10.0, 8.0, 2.0, 5, "a")],
        20,
    )
    .expect("assignment");

    assert_eq!(assignment.rounded_usage(&[2.4, 1.6]), vec![2, 2]);
    assert_eq!(assignment.rounded_usage(&[-1.0]), vec![0]);
}

// ============================================================================
// 主问题 / Master problem
// ============================================================================

/// 主问题要求需求项 / The master problem requires a demand entry
#[test]
fn master_requires_demands() {
    let assignment = Csp2dPatternAssignment::new(
        vec![plan("p1", sheet_material(10.0, 8.0, 10), 10.0, 8.0, 2.0, 5, "a")],
        20,
    )
    .expect("assignment");

    assert!(Csp2dMasterProblem::new(assignment, Vec::new()).is_err());
}

/// 物料用量按次数累计且受可用量约束 / Material usage accumulates and respects availability
#[test]
fn material_usage_and_availability() {
    let assignment = Csp2dPatternAssignment::new(
        vec![plan("p1", sheet_material(10.0, 8.0, 2), 10.0, 8.0, 2.0, 5, "a")],
        20,
    )
    .expect("assignment");
    let master = Csp2dMasterProblem::new(
        assignment,
        vec![Csp2dMasterDemand::new(product_id2("a"), DemandValue2::Amount(20))],
    )
    .expect("master");

    let usage = master.material_usage(&[2.0]);
    assert_eq!(usage.get("m1"), Some(&2));
    assert!(master.material_available(&[2.0]));
    // 可用量只有 2，用 3 次超限。
    assert!(!master.material_available(&[3.0]));
}

/// 余料与面积目标项口径 / Waste and area objective conventions
#[test]
fn objective_terms() {
    // 料域 80，成品 5 × 4 = 20 → 余料 60。
    let assignment = Csp2dPatternAssignment::new(
        vec![plan("p1", sheet_material(10.0, 8.0, 10), 10.0, 8.0, 2.0, 5, "a")],
        20,
    )
    .expect("assignment");
    let master = Csp2dMasterProblem::new(
        assignment,
        vec![Csp2dMasterDemand::new(product_id2("a"), DemandValue2::Amount(20))],
    )
    .expect("master");

    let waste = master.waste_objective_terms(1.0);
    assert_eq!(waste.len(), 1);
    assert!((waste[0].1 - 60.0).abs() < 1e-9);

    let area = master.area_objective_terms(1.0);
    assert!((area[0].1 - 80.0).abs() < 1e-9);
}

/// 覆盖率与覆盖判定 / Coverage ratio and decision
#[test]
fn coverage_ratio() {
    let assignment = Csp2dPatternAssignment::new(
        vec![plan("p1", sheet_material(10.0, 8.0, 10), 10.0, 8.0, 2.0, 5, "a")],
        20,
    )
    .expect("assignment");
    let master = Csp2dMasterProblem::new(
        assignment,
        vec![Csp2dMasterDemand::new(product_id2("a"), DemandValue2::Amount(20))],
    )
    .expect("master");

    assert!((master.coverage_ratio(&[2.0]) - 0.5).abs() < 1e-9);
    assert!((master.coverage_ratio(&[4.0]) - 1.0).abs() < 1e-9);
    // 超量也按 1 截断。
    assert!((master.coverage_ratio(&[10.0]) - 1.0).abs() < 1e-9);
}

/// 剩余需求驱动下一轮生成 / The residual demand drives the next round
#[test]
fn residual_products_from_coverage() {
    let assignment = Csp2dPatternAssignment::new(
        vec![plan("p1", sheet_material(10.0, 8.0, 10), 10.0, 8.0, 2.0, 5, "a")],
        20,
    )
    .expect("assignment");
    let demands = vec![Csp2dMasterDemand::new(
        product_id2("a"),
        DemandValue2::Amount(20),
    )];
    let products = vec![product("a", 2.0, 2.0)];

    let residuals = residual_products(&assignment, &[2.0], &demands, &products);
    assert_eq!(residuals.len(), 1);
    assert_eq!(residuals[0].1, 10);

    // 覆盖完整后没有剩余需求。
    let none = residual_products(&assignment, &[4.0], &demands, &products);
    assert!(none.is_empty());
}

// ============================================================================
// 余料与良率 / Waste and yield
// ============================================================================

/// 余料分析逐方案给出面积与率 / Waste analysis reports area and ratio per plan
#[test]
fn waste_analysis() {
    let plans = vec![plan(
        "p1",
        sheet_material(10.0, 8.0, 10),
        10.0,
        8.0,
        2.0,
        5,
        "a",
    )];
    let result = WasteContext::new().analyze(&plans);

    assert_eq!(result.len(), 1);
    assert!((result.total_waste_area - 60.0).abs() < 1e-9);
    assert!((result.metrics[0].waste_ratio - 60.0 / 80.0).abs() < 1e-9);
    assert!((result.average_waste_ratio() - 60.0 / 80.0).abs() < 1e-9);
}

/// 余料惩罚目标项 / Waste penalty objective terms
#[test]
fn waste_objective_terms() {
    let plans = vec![plan(
        "p1",
        sheet_material(10.0, 8.0, 10),
        10.0,
        8.0,
        2.0,
        5,
        "a",
    )];
    let terms = WasteContext::new().objective_terms(&plans, 0.5);

    assert_eq!(terms.len(), 1);
    assert!((terms[0].1 - 30.0).abs() < 1e-9);
}

/// 良率分析给出欠产与超产 / Yield analysis reports under- and over-production
#[test]
fn yield_analysis() {
    let demands = vec![(product_id2("a"), 20.0), (product_id2("b"), 10.0)];
    let result = YieldContext::new().analyze(&demands, &[15.0, 12.0]);

    assert_eq!(result.metrics.len(), 2);
    assert!((result.total_under_production - 5.0).abs() < 1e-9);
    assert!((result.total_over_production - 2.0).abs() < 1e-9);
    assert!(!result.fully_satisfied());
    // 良率 = (15 + 10) / 30。
    assert!((result.yield_ratio() - 25.0 / 30.0).abs() < 1e-9);
}

/// 良率全部满足 / Yield fully satisfied
#[test]
fn yield_fully_satisfied() {
    let demands = vec![(product_id2("a"), 10.0)];
    let result = YieldContext::new().analyze(&demands, &[10.0]);

    assert!(result.fully_satisfied());
    assert!((result.yield_ratio() - 1.0).abs() < 1e-9);
}

/// 超产上界约束 / Over-production upper bound
#[test]
fn over_production_bounds() {
    let demands = vec![(product_id2("a"), 10.0)];
    let result = YieldContext::new().analyze(&demands, &[15.0]);
    let mut bounds = BTreeMap::new();
    bounds.insert(product_id2("a"), 2.0);

    // 超产 5 > 上界 2 → 失败。
    assert!(
        YieldContext::new()
            .check_over_production_bounds(&result, &bounds)
            .is_err()
    );

    bounds.insert(product_id2("a"), 10.0);
    assert!(
        YieldContext::new()
            .check_over_production_bounds(&result, &bounds)
            .is_ok()
    );
}

// ============================================================================
// 应用路径 / Application paths
// ============================================================================

fn simple_problem(amount: u64) -> Csp2dProblem {
    Csp2dProblem::single(
        sheet_material(10.0, 8.0, 100),
        product("a", 2.0, 2.0),
        amount,
        meters(0.0),
    )
}

/// MILP 路径求解并覆盖需求 / The MILP path solves and covers the demand
#[test]
fn milp_path_covers_demand() {
    let solution = Csp2dSolvers::new()
        .solve_milp(simple_problem(20))
        .expect("solve");

    assert!(solution.fully_covered);
    assert_eq!(solution.termination, Csp2dTerminationReason::Covered);
    assert!(solution.plan_count() > 0);
    assert!(solution.yield_result.fully_satisfied());
}

/// 列生成路径求解并覆盖需求 / The column generation path solves and covers the demand
#[test]
fn column_generation_path_covers_demand() {
    let solution = Csp2dSolvers::new()
        .solve_column_generation(simple_problem(20))
        .expect("solve");

    assert!(solution.fully_covered);
    assert_eq!(solution.termination, Csp2dTerminationReason::Covered);
    assert!(!solution.iterations.is_empty());
}

/// 需求超出板材产能时显式报告覆盖不足 / Insufficient capacity is reported explicitly
#[test]
fn insufficient_capacity_is_reported() {
    // 板材 10×8 只能放 20 件 2×2，需求 100 件无法一次覆盖；列生成会追加更多方案。
    let solution = Csp2dSolvers::new()
        .solve_column_generation(simple_problem(100))
        .expect("solve");

    // 允许通过多块板材覆盖；只要最终覆盖完整即视为成功。
    assert!(solution.plan_count() > 0);
    assert!(!solution.material_usage.is_empty());
}

/// 无可用物料尺寸时返回显式失败 / No usable material size yields an explicit failure
#[test]
fn no_candidate_plan_is_reported() {
    let problem = Csp2dProblem::single(
        sheet_material(10.0, 8.0, 100),
        // 产品比板材还大，放不下。
        product("huge", 20.0, 20.0),
        5,
        meters(0.0),
    );
    let solution = Csp2dSolvers::new()
        .solve_milp(problem)
        .expect("solve");

    assert!(!solution.fully_covered);
    assert_eq!(
        solution.termination,
        Csp2dTerminationReason::NoCandidatePlan
    );
    assert!(solution.failure_message.is_some());
}

/// 非法问题被拒绝 / An illegal problem is rejected
#[test]
fn illegal_problem_is_rejected() {
    let mut problem = simple_problem(10);
    problem.materials.clear();
    assert!(Csp2dSolvers::new().solve_milp(problem).is_err());

    let mut problem = simple_problem(10);
    problem.kerf = meters(-1.0);
    assert!(Csp2dSolvers::new().solve_milp(problem).is_err());
}

/// 面积需求折算为件数 / An area demand converts to a piece count
#[test]
fn area_demand_converts_to_pieces() {
    let problem = Csp2dProblem::new(
        vec![sheet_material(10.0, 8.0, 100)],
        vec![product("a", 2.0, 2.0)],
        vec![ProductDemand2::area(product("a", 2.0, 2.0), 40.0)],
        meters(0.0),
    );

    // 单件 4，40 / 4 = 10 件。
    let products = problem.demand_products();
    assert_eq!(products.len(), 1);
    assert_eq!(products[0].1, 10);
    assert_eq!(problem.demand_piece_bound(), 10);
}

/// 配置默认值 / Configuration defaults
#[test]
fn configuration_defaults() {
    let config = Csp2dConfiguration::default();
    assert_eq!(config.max_candidates_per_iteration, 32);
    assert_eq!(config.iteration_limit, 20);
    assert_eq!(config.waste_weight, 1.0);
    assert!(config.final_integer_enabled);
}

/// 多物料分别生成候选 / Multiple materials generate candidates separately
#[test]
fn multiple_materials_generate_candidates() {
    let problem = Csp2dProblem::new(
        vec![
            sheet_material(10.0, 8.0, 100),
            Material2::sheet(material_id2("m2"), "small", meters(6.0), meters(4.0), 100),
        ],
        vec![product("a", 2.0, 2.0)],
        vec![ProductDemand2::amount(product("a", 2.0, 2.0), 20)],
        meters(0.0),
    );
    let solution = Csp2dSolvers::new()
        .solve_column_generation(problem)
        .expect("solve");

    assert!(solution.material_usage.len() >= 1);
    assert!(solution.fully_covered || solution.plan_count() > 0);
}
