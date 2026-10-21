//! 检验数定价契约测试 / Reduced-cost pricing contract tests
//!
//! 覆盖定价的四条口径：检验数公式（消耗量 × 单价 − Σ 覆盖系数 × 影子价格）、只有负检验数算改进、
//! 面积与段高两种计价、以及未入列图案的估算路径。
//! Covers the four pricing conventions: the reduced-cost formula, "only negative counts as improving", the area
//! and segment-height pricing modes, and the estimate path for patterns outside the master.

use std::collections::BTreeMap;

use ospf_rust_framework_csp2d::domain::material::{
    CuttingPlan2, DemandValue2, Material2, Pattern2, material_id2, pattern_id2, product_id2,
};
use ospf_rust_framework_csp2d::domain::produce::{
    Csp2dMasterDemand, Csp2dPatternAssignment, Csp2dPricingConfig, DEFAULT_REDUCED_COST_TOLERANCE,
    estimate_reduced_cost, estimated_shadow_prices, improving_columns, reduced_cost_of, reduced_costs,
};
use ospf_rust_framework_csp2d::infrastructure::{CuttingProcess2, region2_of};
use ospf_rust_framework_geometry2d::{
    PackingShape2, Placement2, QuantityPoint2, rectangle_shape2,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;

fn meters(value: f64) -> Quantity<f64, Meter> {
    Quantity::new_ct(value)
}

fn sheet_material(width: f64, height: f64) -> Material2<f64, Meter> {
    Material2::sheet(material_id2("m1"), "sheet", meters(width), meters(height), 100)
}

fn plan(
    id: &str,
    width: f64,
    height: f64,
    part: f64,
    amount: usize,
    product_id: &str,
) -> CuttingPlan2<f64, Meter> {
    let shape = PackingShape2::Rectangle(rectangle_shape2(meters(part), meters(part)).expect("rect"));
    let placements = (0..amount)
        .map(|index| {
            Placement2::new(
                shape.clone(),
                QuantityPoint2 {
                    x: meters(index as f64 * part),
                    y: meters(0.0),
                },
            )
            .with_item_id(product_id)
        })
        .collect();
    let pattern = Pattern2::new(
        pattern_id2(id),
        sheet_material(width, height),
        region2_of(meters(0.0), meters(0.0), meters(width), meters(height)),
        placements,
        CuttingProcess2::Freeform,
    );
    CuttingPlan2::from_pattern(pattern)
}

fn assignment(plans: Vec<CuttingPlan2<f64, Meter>>) -> Csp2dPatternAssignment {
    Csp2dPatternAssignment::new(plans, 100).expect("assignment")
}

fn demand(product: &str, amount: u64) -> Csp2dMasterDemand {
    Csp2dMasterDemand::new(product_id2(product), DemandValue2::Amount(amount))
}

// ============================================================================
// 检验数公式 / Reduced-cost formula
// ============================================================================

/// 无覆盖收益时检验数等于消耗成本 / Without revenue the reduced cost equals the consumption cost
#[test]
fn reduced_cost_without_revenue() {
    // 料域 10×8 = 80，单价 1，无影子价格 → rc = 80。
    let assignment = assignment(vec![plan("p1", 10.0, 8.0, 2.0, 5, "a")]);
    let config = Csp2dPricingConfig::default();
    let entry = reduced_cost_of(&assignment, 0, &[demand("a", 20)], &BTreeMap::new(), &config);

    assert!((entry.consumption - 80.0).abs() < 1e-9);
    assert!((entry.revenue - 0.0).abs() < 1e-9);
    assert!((entry.reduced_cost - 80.0).abs() < 1e-9);
    assert!(!entry.is_improving(config.tolerance));
}

/// 覆盖收益抵掉消耗后检验数转负 / Revenue exceeding consumption makes the reduced cost negative
#[test]
fn reduced_cost_with_high_revenue() {
    let assignment = assignment(vec![plan("p1", 10.0, 8.0, 2.0, 5, "a")]);
    let config = Csp2dPricingConfig::default();
    let mut prices = BTreeMap::new();
    // 5 件 × 20 = 100 > 80。
    prices.insert(product_id2("a"), 20.0);
    let entry = reduced_cost_of(&assignment, 0, &[demand("a", 20)], &prices, &config);

    assert!((entry.revenue - 100.0).abs() < 1e-9);
    assert!((entry.reduced_cost - (-20.0)).abs() < 1e-9);
    assert!(entry.is_improving(config.tolerance));
}

/// 消耗量单价按比例放大检验数 / The consumption price scales the reduced cost
#[test]
fn consumption_price_scales_reduced_cost() {
    let assignment = assignment(vec![plan("p1", 10.0, 8.0, 2.0, 5, "a")]);
    let config = Csp2dPricingConfig {
        consumption_price: 2.0,
        ..Csp2dPricingConfig::default()
    };
    let entry = reduced_cost_of(&assignment, 0, &[demand("a", 20)], &BTreeMap::new(), &config);

    assert!((entry.reduced_cost - 160.0).abs() < 1e-9);
}

/// 恰好为零不算改进 / An exact zero is not an improvement
#[test]
fn zero_reduced_cost_is_not_improving() {
    let assignment = assignment(vec![plan("p1", 10.0, 8.0, 2.0, 5, "a")]);
    let config = Csp2dPricingConfig::default();
    let mut prices = BTreeMap::new();
    // 5 件 × 16 = 80，恰好抵消。
    prices.insert(product_id2("a"), 16.0);
    let entry = reduced_cost_of(&assignment, 0, &[demand("a", 20)], &prices, &config);

    assert!(entry.reduced_cost.abs() < 1e-9);
    assert!(!entry.is_improving(config.tolerance));
}

/// 面积需求按「单件面积 × 件数」计价 / An area demand is priced by unit area times count
#[test]
fn area_demand_pricing() {
    let assignment = assignment(vec![plan("p1", 10.0, 8.0, 2.0, 5, "a")]);
    let config = Csp2dPricingConfig::default();
    let mut prices = BTreeMap::new();
    prices.insert(product_id2("a"), 1.0);
    let area_demand = Csp2dMasterDemand::new(product_id2("a"), DemandValue2::Area(80.0));
    let entry = reduced_cost_of(&assignment, 0, &[area_demand], &prices, &config);

    // 单件 4 × 5 件 = 20。
    assert!((entry.revenue - 20.0).abs() < 1e-9);
}

/// 卷材按段高计价 / A coil is priced by segment height
#[test]
fn height_pricing_uses_segment_height() {
    let coil = Material2::coil(material_id2("c1"), "coil", meters(10.0), 100);
    let region = coil.region(Some(meters(4.0))).expect("region");
    let shape = PackingShape2::Rectangle(rectangle_shape2(meters(2.0), meters(2.0)).expect("rect"));
    let pattern = Pattern2::new(
        pattern_id2("p-coil"),
        coil,
        region,
        vec![
            Placement2::new(shape, QuantityPoint2 { x: meters(0.0), y: meters(0.0) })
                .with_item_id("a"),
        ],
        CuttingProcess2::Freeform,
    );
    let assignment = assignment(vec![CuttingPlan2::from_pattern(pattern)]);
    let config = Csp2dPricingConfig {
        price_by_height: true,
        ..Csp2dPricingConfig::default()
    };
    let entry = reduced_cost_of(&assignment, 0, &[demand("a", 10)], &BTreeMap::new(), &config);

    // 段高 4（而非面积 40）。
    assert!((entry.consumption - 4.0).abs() < 1e-9);
}

// ============================================================================
// 改进列筛选 / Improving-column selection
// ============================================================================

/// 全部列的检验数按顺序返回 / Reduced costs are returned for every column in order
#[test]
fn reduced_costs_cover_every_column() {
    let assignment = assignment(vec![
        plan("p1", 10.0, 8.0, 2.0, 5, "a"),
        plan("p2", 10.0, 8.0, 4.0, 2, "a"),
    ]);
    let config = Csp2dPricingConfig::default();
    let entries = reduced_costs(&assignment, &[demand("a", 20)], &BTreeMap::new(), &config);

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].plan_index, 0);
    assert_eq!(entries[1].plan_index, 1);
}

/// 改进列按检验数升序返回 / Improving columns are sorted by ascending reduced cost
#[test]
fn improving_columns_are_sorted() {
    let assignment = assignment(vec![
        // 覆盖 2 件、消耗 80：收益 2 × 60 = 120 → rc = −40。
        plan("p1", 10.0, 8.0, 2.0, 2, "a"),
        // 覆盖 5 件、消耗 80：收益 5 × 60 = 300 → rc = −220。
        plan("p2", 10.0, 8.0, 2.0, 5, "a"),
    ]);
    let config = Csp2dPricingConfig::default();
    let mut prices = BTreeMap::new();
    prices.insert(product_id2("a"), 60.0);

    let improving = improving_columns(&assignment, &[demand("a", 20)], &prices, &config);
    assert_eq!(improving.len(), 2);
    assert!(improving[0].reduced_cost <= improving[1].reduced_cost);
    assert!((improving[0].reduced_cost - (-220.0)).abs() < 1e-9);
}

/// 没有改进列时返回空 / No improving column yields an empty result
#[test]
fn no_improving_columns() {
    let assignment = assignment(vec![plan("p1", 10.0, 8.0, 2.0, 5, "a")]);
    let config = Csp2dPricingConfig::default();
    let improving = improving_columns(&assignment, &[demand("a", 20)], &BTreeMap::new(), &config);

    assert!(improving.is_empty());
}

// ============================================================================
// 未入列图案的估算 / Estimating a pattern outside the master
// ============================================================================

/// 未入列图案用 usize::MAX 标记 / A pattern outside the master is marked with usize::MAX
#[test]
fn estimate_marks_outside_index() {
    let candidate = plan("new", 10.0, 8.0, 2.0, 5, "a");
    let config = Csp2dPricingConfig::default();
    let mut prices = BTreeMap::new();
    prices.insert(product_id2("a"), 20.0);

    let entry = estimate_reduced_cost(&candidate, &[demand("a", 20)], &prices, &config);
    assert_eq!(entry.plan_index, usize::MAX);
    assert!((entry.reduced_cost - (-20.0)).abs() < 1e-9);
}

/// 估算与入列口径一致 / The estimate matches the in-master convention
#[test]
fn estimate_matches_column_convention() {
    let candidate = plan("p1", 10.0, 8.0, 2.0, 5, "a");
    let assignment = assignment(vec![candidate.clone()]);
    let config = Csp2dPricingConfig::default();
    let mut prices = BTreeMap::new();
    prices.insert(product_id2("a"), 20.0);
    let demands = vec![demand("a", 20)];

    let from_column = reduced_cost_of(&assignment, 0, &demands, &prices, &config);
    let from_estimate = estimate_reduced_cost(&candidate, &demands, &prices, &config);

    assert!((from_column.reduced_cost - from_estimate.reduced_cost).abs() < 1e-9);
    assert!((from_column.consumption - from_estimate.consumption).abs() < 1e-9);
}

// ============================================================================
// 影子价格估计 / Shadow price estimation
// ============================================================================

/// 覆盖完整时估计影子价格为零 / A fully covered demand has a zero estimated price
#[test]
fn estimated_prices_vanish_when_covered() {
    let assignment = assignment(vec![plan("p1", 10.0, 8.0, 2.0, 5, "a")]);
    let demands = vec![demand("a", 5)];
    // 使用 1 次覆盖 5 件，恰好满足。
    let prices = estimated_shadow_prices(&assignment, &[1.0], &demands);

    assert!((prices.get(&product_id2("a")).copied().unwrap_or(1.0)).abs() < 1e-9);
}

/// 覆盖不足时估计影子价格为正 / An uncovered demand has a positive estimated price
#[test]
fn estimated_prices_positive_when_uncovered() {
    let assignment = assignment(vec![plan("p1", 10.0, 8.0, 2.0, 5, "a")]);
    let demands = vec![demand("a", 20)];
    // 使用 1 次只覆盖 5 / 20，缺口比例 0.75 → 估计 0.75 × 20 = 15。
    let prices = estimated_shadow_prices(&assignment, &[1.0], &demands);

    let price = prices.get(&product_id2("a")).copied().unwrap_or(0.0);
    assert!((price - 15.0).abs() < 1e-9, "price = {price}");
}

/// 容差常量是极小正数 / The tolerance constant is a tiny positive number
#[test]
fn default_tolerance_is_tiny() {
    assert!(DEFAULT_REDUCED_COST_TOLERANCE > 0.0);
    assert!(DEFAULT_REDUCED_COST_TOLERANCE < 1e-6);
}
