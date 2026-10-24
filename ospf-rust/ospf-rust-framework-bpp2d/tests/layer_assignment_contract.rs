//! 层分配契约测试 / Layer assignment contract tests
//!
//! 对应 Kotlin `LayerAssignmentContractTest` 与 `MixedSheetTypeContractTest`：
//! 需求覆盖语义、列身份去重、检验数定价、目标分层（面积为主、板数与类型成本为次）、
//! 每列份数上界、容量校验与列选择统计。
//! Mirrors Kotlin `LayerAssignmentContractTest` and `MixedSheetTypeContractTest`: demand coverage semantics,
//! column identity deduplication, reduced-cost pricing, layered objectives (area primary, sheet count and type
//! cost secondary), per-column copy bounds, capacity validation and column-selection statistics.

use std::collections::BTreeMap;

use ospf_rust_framework_bpp2d::domain::item::model::{
    ActualItem, ItemId, Layer, SheetType, SheetTypeId,
};
use ospf_rust_framework_bpp2d::domain::layer_assignment::{
    Bpp2dAssignmentDemandEntry, Bpp2dDemandShadowPriceKey, LayerAssignmentContext,
    LayerAssignmentObjectiveConfig, LayerAggregation, LayerAssignmentContext as Context,
    SheetCapacityConstraint, demand_cap, filter_improving_columns, is_improving_column,
    layer_demand_coefficient, layer_reduced_cost, layer_waste_area, layer_waste_ratio,
    max_copies_per_column, select_columns_by_copies, sheet_amount_terms, sheet_area_terms,
    sheet_type_cost_terms, sheets_used_by_copies, total_area_used, total_waste_area,
};
use ospf_rust_framework_geometry2d::{
    PackingShape2, Placement2, QuantityPoint2, Sheet2Shape, rectangle_shape2,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;

fn meters(value: f64) -> Quantity<f64, Meter> {
    Quantity::new_ct(value)
}

fn sheet_type(code: &str) -> SheetType<f64, Meter> {
    SheetType::new(meters(10.0), meters(8.0), meters(100.0), SheetTypeId::new(code))
}

/// 构造一层：在板材上放 amount 个 width×height 的矩形。
/// Build a layer holding `amount` rectangles of width×height on the sheet.
fn layer(code: &str, width: f64, height: f64, amount: usize, item: &str) -> Layer<f64, Meter> {
    let shape = PackingShape2::Rectangle(rectangle_shape2(meters(width), meters(height)).expect("rect"));
    let mut units = Vec::new();
    for index in 0..amount {
        let x = meters((index as f64) * width);
        units.push(
            Placement2::new(shape.clone(), QuantityPoint2 {
                x,
                y: meters(0.0),
            })
            .with_item_id(item),
        );
    }
    Layer::new(
        0,
        "test",
        Some(sheet_type(code)),
        Sheet2Shape::new(meters(width * amount as f64), meters(height)),
    )
    .with_units(units)
}

// ============================================================================
// 层聚合 / Layer aggregation
// ============================================================================

/// 层聚合按身份去重 / Layer aggregation deduplicates by identity
#[test]
fn aggregation_deduplicates_by_identity() {
    let mut aggregation = LayerAggregation::<f64, Meter>::new();
    let first = layer("S1", 2.0, 2.0, 3, "a");

    let added = aggregation.add_columns(vec![first.clone(), first.clone()]);
    assert_eq!(added.len(), 1);
    assert_eq!(aggregation.len(), 1);

    // 再次加入同一层不产生新增。
    assert!(aggregation.add_columns(vec![first]).is_empty());
    assert_eq!(aggregation.len(), 1);
}

/// 同一排布换板材是不同列 / The same layout on another sheet is a different column
#[test]
fn aggregation_distinguishes_sheet_types() {
    let mut aggregation = LayerAggregation::<f64, Meter>::new();
    let small = layer("S1", 2.0, 2.0, 3, "a");
    let large = layer("S2", 2.0, 2.0, 3, "a");

    assert_eq!(aggregation.add_columns(vec![small, large]).len(), 2);
}

// ============================================================================
// 需求覆盖 / Demand coverage
// ============================================================================

/// 件数需求的系数是层内件数 / An amount demand's coefficient is the piece count in the layer
#[test]
fn amount_demand_coefficient_counts_pieces() {
    let l = layer("S1", 2.0, 2.0, 5, "a");
    let entry = Bpp2dAssignmentDemandEntry::item_amount("a", 10);

    assert!((layer_demand_coefficient(&l, &entry) - 5.0).abs() < 1e-9);
}

/// 面积需求的系数是单件面积乘件数 / An area demand's coefficient is the per-piece area times the count
#[test]
fn area_demand_coefficient_uses_unit_area() {
    let l = layer("S1", 2.0, 3.0, 4, "a");
    let entry = Bpp2dAssignmentDemandEntry::item_area("a", 96.0);

    // 单件 6，4 件 → 24。
    assert!((layer_demand_coefficient(&l, &entry) - 24.0).abs() < 1e-9);
}

/// 层内不含该物品时系数为零 / The coefficient is zero when the layer holds none of the item
#[test]
fn missing_item_yields_zero_coefficient() {
    let l = layer("S1", 2.0, 2.0, 3, "a");
    let entry = Bpp2dAssignmentDemandEntry::item_amount("b", 10);

    assert_eq!(layer_demand_coefficient(&l, &entry), 0.0);
}

/// 需求条目提供稳定的约束名 / A demand entry exposes a stable constraint name
#[test]
fn demand_entry_constraint_name() {
    let entry = Bpp2dAssignmentDemandEntry::item_amount("a", 10);
    assert_eq!(entry.constraint_name(), "demand_item_amount_a");
    assert!(entry.demand.is_positive());
    assert!(entry.upper_bound.is_none());
}

/// 需求上限取所有条目中的最大值 / The demand cap takes the maximum over all entries
#[test]
fn demand_cap_takes_maximum() {
    let entries = vec![
        Bpp2dAssignmentDemandEntry::item_amount("a", 10),
        Bpp2dAssignmentDemandEntry::item_amount("b", 25),
        Bpp2dAssignmentDemandEntry::item_area("c", 12.5),
    ];
    assert_eq!(demand_cap(&entries), 25);
    assert_eq!(demand_cap(&[]), 0);
}

// ============================================================================
// 检验数 / Reduced cost
// ============================================================================

/// 无对偶时检验数等于面积消耗 / Without duals the reduced cost equals the area cost
#[test]
fn reduced_cost_without_duals_equals_area() {
    let l = layer("S1", 2.0, 2.0, 3, "a");
    let entries = vec![Bpp2dAssignmentDemandEntry::item_amount("a", 10)];
    let prices = Bpp2dDemandShadowPriceKey::new(entries[0].mode, "a");
    let _ = prices;

    let cost = layer_reduced_cost(&l, &entries, &BTreeMap::new(), 1.0);
    // 3 件 × 4 = 12。
    assert!((cost - 12.0).abs() < 1e-9);
}

/// 对偶足够大时列成为改进列 / A large enough dual makes the column improving
#[test]
fn dual_makes_column_improving() {
    let l = layer("S1", 2.0, 2.0, 3, "a");
    let entries = vec![Bpp2dAssignmentDemandEntry::item_amount("a", 10)];
    let mut prices = BTreeMap::new();
    prices.insert(
        Bpp2dDemandShadowPriceKey::new(entries[0].mode, "a"),
        5.0,
    );

    // 面积 12 − 对偶 5×3 = 15 → −3。
    let cost = layer_reduced_cost(&l, &entries, &prices, 1.0);
    assert!((cost + 3.0).abs() < 1e-9, "cost={cost}");
    assert!(is_improving_column(&l, &entries, &prices, 1.0, 1e-6));
    assert_eq!(
        filter_improving_columns(vec![l], &entries, &prices, 1.0, 1e-6).len(),
        1
    );
}

/// 对偶不足时列不是改进列 / An insufficient dual keeps the column out
#[test]
fn small_dual_is_not_improving() {
    let l = layer("S1", 2.0, 2.0, 3, "a");
    let entries = vec![Bpp2dAssignmentDemandEntry::item_amount("a", 10)];
    let mut prices = BTreeMap::new();
    prices.insert(
        Bpp2dDemandShadowPriceKey::new(entries[0].mode, "a"),
        1.0,
    );

    // 面积 12 − 对偶 1×3 = 3 → 正。
    assert!(!is_improving_column(&l, &entries, &prices, 1.0, 1e-6));
    assert!(
        filter_improving_columns(vec![l], &entries, &prices, 1.0, 1e-6).is_empty()
    );
}

/// 检验数恰好等于零时不算改进 / A zero reduced cost does not count as improving
#[test]
fn zero_reduced_cost_is_not_improving() {
    let l = layer("S1", 2.0, 2.0, 3, "a");
    let entries = vec![Bpp2dAssignmentDemandEntry::item_amount("a", 10)];
    let mut prices = BTreeMap::new();
    prices.insert(
        Bpp2dDemandShadowPriceKey::new(entries[0].mode, "a"),
        4.0,
    );

    // 面积 12 − 对偶 4×3 = 0。
    assert_eq!(layer_reduced_cost(&l, &entries, &prices, 1.0), 0.0);
    assert!(!is_improving_column(&l, &entries, &prices, 1.0, 1e-6));
}

// ============================================================================
// 目标分层 / Layered objectives
// ============================================================================

/// 面积目标按板材面积作系数 / Area terms weigh by the sheet area
#[test]
fn area_terms_use_sheet_area() {
    let l = layer("S1", 2.0, 2.0, 3, "a");
    let terms = sheet_area_terms(&[l], 1.0);

    assert_eq!(terms.len(), 1);
    assert_eq!(terms[0].0, 0);
    // 板材 10×8 = 80。
    assert!((terms[0].1 - 80.0).abs() < 1e-9);
}

/// 板数目标权重为零时不产出项 / A zero sheet-amount weight yields no term
#[test]
fn sheet_amount_terms_respect_weight() {
    let l = layer("S1", 2.0, 2.0, 3, "a");
    assert!(sheet_amount_terms(&[l.clone()], 0.0).is_empty());

    let terms = sheet_amount_terms(&[l], 1e-6);
    assert_eq!(terms.len(), 1);
    assert!((terms[0].1 - 1e-6).abs() < 1e-18);
}

/// 类型成本按板材类型计价且跳过未配置项 / Type costs price by sheet type and skip unconfigured types
#[test]
fn type_cost_terms_price_per_type() {
    let small = layer("S1", 2.0, 2.0, 3, "a");
    let large = layer("S2", 2.0, 2.0, 3, "a");
    let mut costs = BTreeMap::new();
    costs.insert(SheetTypeId::new("S2"), 5.0);

    let terms = sheet_type_cost_terms(&[small, large], &costs, 1.0);
    // 只保留配置了成本的 S2，且系数为 5。
    assert_eq!(terms.len(), 1);
    assert_eq!(terms[0].0, 1);
    assert!((terms[0].1 - 5.0).abs() < 1e-9);
}

/// 默认目标配置以面积为主 / The default objective config is area-primary
#[test]
fn default_objective_config_is_area_primary() {
    let config = LayerAssignmentObjectiveConfig::default();
    assert_eq!(config.sheet_area_weight, 1.0);
    assert_eq!(config.sheet_amount_weight, 1e-6);
    assert_eq!(config.sheet_type_cost_weight, 0.0);
    assert!(config.sheet_type_costs.is_empty());
}

/// 汇总目标项包含面积与板数两类 / The aggregated terms carry both area and sheet-count kinds
#[test]
fn aggregated_terms_include_area_and_amount() {
    let l = layer("S1", 2.0, 2.0, 3, "a");
    let config = LayerAssignmentObjectiveConfig::default();
    let terms = config.all_terms(&[l]);

    assert_eq!(terms.len(), 2, "area + amount");
    assert!((terms[0].1 - 80.0).abs() < 1e-9);
    assert!((terms[1].1 - 1e-6).abs() < 1e-18);
}

// ============================================================================
// 余料统计 / Waste statistics
// ============================================================================

/// 余料面积与余料率按板材口径 / Waste area and ratio follow the sheet convention
#[test]
fn waste_statistics_follow_sheet() {
    // 板材 80，占用 3×2×2 = 12。
    let l = layer("S1", 2.0, 2.0, 3, "a");
    assert!((layer_waste_area(&l) - 68.0).abs() < 1e-9);
    assert!((layer_waste_ratio(&l) - 68.0 / 80.0).abs() < 1e-9);
}

/// 未绑定板材的列余料为零 / A column without a sheet reports zero waste
#[test]
fn waste_without_sheet_is_zero() {
    let l = Layer::<f64, Meter>::new(
        0,
        "free",
        None,
        Sheet2Shape::new(meters(4.0), meters(4.0)),
    );
    assert_eq!(layer_waste_area(&l), 0.0);
    assert_eq!(layer_waste_ratio(&l), 0.0);
}

// ============================================================================
// 份数上界与容量 / Copy bounds and capacity
// ============================================================================

/// 每列份数上界按覆盖件数收紧 / The per-column copy bound is tightened by covered pieces
#[test]
fn max_copies_use_covered_pieces() {
    let three = layer("S1", 2.0, 2.0, 3, "a");
    let empty = Layer::<f64, Meter>::new(0, "empty", None, Sheet2Shape::new(meters(1.0), meters(1.0)));

    // 需求 10、每列 3 件 → 4（向上取整）；空列退化为 1。
    assert_eq!(max_copies_per_column(&[three, empty], 10), vec![4, 1]);
    // 需求为 0 时上界仍至少为 1，避免把列锁死。
    assert_eq!(
        max_copies_per_column(&[layer("S1", 2.0, 2.0, 3, "a")], 0),
        vec![1]
    );
}

/// 容量约束拒绝超出板材面积的列 / The capacity constraint rejects a column over the sheet area
#[test]
fn capacity_rejects_oversized_column() {
    // 列占用 6×10 = 60，板材 80 → 通过；把板材面积压到 50 → 拒绝。
    let l = layer("S1", 6.0, 10.0, 1, "a");
    let ok = SheetCapacityConstraint::from_layers(&[l.clone()], 80.0, 100.0);
    assert!(ok.validate().is_ok());

    let bad = SheetCapacityConstraint::from_layers(&[l], 50.0, 100.0);
    assert!(bad.validate().is_err());
}

/// 容量约束拒绝超出承重的列 / The capacity constraint rejects a column over the load limit
#[test]
fn capacity_rejects_overloaded_column() {
    let l = layer("S1", 2.0, 2.0, 3, "a");
    let constraint = SheetCapacityConstraint::from_layers(&[l], 80.0, 5.0)
        .with_layer_loads(vec![9.0]);

    assert!(constraint.validate().is_err());
}

/// 由份数推导板材数 / Sheet count derives from the copies
#[test]
fn sheet_count_from_copies() {
    assert_eq!(sheets_used_by_copies(&[2.0, 1.0, 0.0]), 3);
    assert_eq!(sheets_used_by_copies(&[2.4]), 3);
    assert_eq!(sheets_used_by_copies(&[-1.0, 1.0]), 1);
}

// ============================================================================
// 列选择 / Column selection
// ============================================================================

/// 零份数列被丢弃 / Zero-copy columns are dropped
#[test]
fn selection_drops_zero_copies() {
    let a = layer("S1", 2.0, 2.0, 3, "a");
    let b = layer("S1", 3.0, 2.0, 2, "b");
    let selection = select_columns_by_copies(&[a, b], &[2.0, 0.0]);

    assert_eq!(selection.layers.len(), 1);
    assert_eq!(selection.copies, vec![2]);
    assert_eq!(selection.sheet_count(), 2);
}

/// 面积与余料按份数累计 / Area and waste accumulate by copies
#[test]
fn selection_accumulates_area_and_waste() {
    let l = layer("S1", 2.0, 2.0, 3, "a");
    let selection = select_columns_by_copies(&[l], &[2.0]);

    // 板材 80 × 2 = 160；余料 68 × 2 = 136。
    assert!((total_area_used(&selection) - 160.0).abs() < 1e-9);
    assert!((total_waste_area(&selection) - 136.0).abs() < 1e-9);
}

/// 空选择返回零统计 / An empty selection yields zero statistics
#[test]
fn empty_selection_yields_zero() {
    let selection = select_columns_by_copies::<f64, Meter>(&[], &[]);
    assert!(selection.is_empty());
    assert_eq!(selection.sheet_count(), 0);
    assert_eq!(total_area_used(&selection), 0.0);
    assert_eq!(total_waste_area(&selection), 0.0);
}

// ============================================================================
// 上下文生命周期 / Context lifecycle
// ============================================================================

/// RMP 注册要求存在需求项 / RMP registration requires a demand entry
#[test]
fn rmp_requires_demand_entries() {
    let context = Context::new(LayerAssignmentObjectiveConfig::default());
    assert!(context.register_rmp(Vec::new(), Vec::new(), false).is_err());
}

/// 最终 MILP 注册要求存在候选列 / Final MILP registration requires candidate columns
#[test]
fn final_requires_layers() {
    let context = Context::new(LayerAssignmentObjectiveConfig::default());
    assert!(
        context
            .register_final(Vec::new(), Vec::new(), true)
            .is_err()
    );
}

/// 上下文完成注册、加列、影子价格与最终注册五阶段 / The context covers all five stages
#[test]
fn context_covers_five_stages() {
    let mut context =
        LayerAssignmentContext::new(LayerAssignmentObjectiveConfig::default());
    let entries = vec![Bpp2dAssignmentDemandEntry::item_amount("a", 10)];
    let initial = layer("S1", 2.0, 2.0, 3, "a");

    let mut artifacts = context
        .register_rmp(vec![initial.clone()], entries.clone(), false)
        .expect("rmp");
    assert_eq!(artifacts.aggregation.len(), 1);
    assert!(!artifacts.area_terms.is_empty());

    let extra = layer("S1", 3.0, 2.0, 2, "a");
    let added = context.add_columns(&mut artifacts.aggregation, 1, vec![extra.to_owned()]);
    assert_eq!(added.len(), 1);
    assert_eq!(context.iteration, 1);

    // 对偶缺失时映射为空，不用零值伪造。
    let prices = context.refresh_shadow_prices(&entries, &[]);
    assert!(prices.is_empty());

    let prices = context.refresh_shadow_prices(&entries, &[7.0]);
    assert!((context.shadow_prices()[&Bpp2dDemandShadowPriceKey::new(entries[0].mode, "a")] - 7.0).abs() < 1e-9);
    assert_eq!(prices.len(), 1);

    let final_artifacts = context
        .register_final(artifacts.aggregation.layers().to_vec(), entries.clone(), true)
        .expect("final");
    assert_eq!(final_artifacts.layers.len(), 2);
    assert_eq!(final_artifacts.max_copies.len(), 2);

    let terms = context.final_objective_terms(&final_artifacts.layers);
    assert!(!terms.is_empty());
}

/// 退役列从聚合中移除 / Retiring columns removes them from the aggregation
#[test]
fn retire_columns_removes_from_aggregation() {
    let mut context =
        LayerAssignmentContext::new(LayerAssignmentObjectiveConfig::default());
    let entries = vec![Bpp2dAssignmentDemandEntry::item_amount("a", 10)];
    let mut artifacts = context
        .register_rmp(vec![layer("S1", 2.0, 2.0, 3, "a")], entries, false)
        .expect("rmp");

    let retired = artifacts.aggregation.layers().to_vec();
    let removed = context.retire_columns(&mut artifacts.aggregation, &retired);

    assert_eq!(removed.len(), 1);
    assert!(artifacts.aggregation.is_empty());
}

/// 按板材类型统计列数 / Columns are counted per sheet type
#[test]
fn columns_counted_per_sheet_type() {
    let context = LayerAssignmentContext::new(LayerAssignmentObjectiveConfig::default());
    let layers = vec![
        layer("S1", 2.0, 2.0, 3, "a"),
        layer("S1", 3.0, 2.0, 2, "a"),
        layer("S2", 2.0, 2.0, 3, "a"),
    ];

    let counts = context.columns_by_sheet_type(&layers);
    assert_eq!(counts.get(&SheetTypeId::new("S1")), Some(&2));
    assert_eq!(counts.get(&SheetTypeId::new("S2")), Some(&1));
}

/// 覆盖量按份数累计 / Coverage accumulates by copies
#[test]
fn coverage_accumulates_by_copies() {
    let context = LayerAssignmentContext::new(LayerAssignmentObjectiveConfig::default());
    let l = layer("S1", 2.0, 2.0, 3, "a");
    let entry = Bpp2dAssignmentDemandEntry::item_amount("a", 10);

    // 每列 3 件、2 份 → 6。
    assert!((context.coverage(&[l], &[2.0], &entry) - 6.0).abs() < 1e-9);
}

// ============================================================================
// 物品辅助 / Item helpers
// ============================================================================

/// 层内件数映射可用于需求系数 / The layer's item amounts feed the demand coefficient
#[test]
fn layer_item_amounts_feed_coefficients() {
    let item = ActualItem::new(
        ItemId::new("a"),
        "a",
        PackingShape2::Rectangle(rectangle_shape2(meters(2.0), meters(2.0)).expect("rect")),
        false,
        meters(1.0),
    );
    let l = layer("S1", 2.0, 2.0, 4, item.id.as_str());
    let amounts = l.item_amounts();

    assert_eq!(amounts.get(&ItemId::new("a")), Some(&4));
    let entry = Bpp2dAssignmentDemandEntry::item_amount("a", 8);
    assert!((layer_demand_coefficient(&l, &entry) - 4.0).abs() < 1e-9);
}
