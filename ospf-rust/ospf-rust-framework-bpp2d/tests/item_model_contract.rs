//! 物品模型契约测试 / Item model contract tests
//!
//! 对应 Kotlin `ItemModelContractTest`：形状面积口径、层面积与利用率、板材装载率、
//! 列身份含板材类型、旋转候选、图纸类型参与身份。
//! Mirrors Kotlin `ItemModelContractTest`: shape area conventions, layer area and utilisation, sheet
//! loading rate, sheet type in column identity, rotation candidates and sheet types in identity.

use ospf_rust_framework_bpp2d::domain::item::model::*;
use ospf_rust_framework_geometry2d::{
    PackingShape2, PackingShape2Type, Placement2, Sheet2Shape, circle_shape2, point2_of,
    rectangle_shape2,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::derived::Meter;
use std::f64::consts::PI;

fn meters(value: f64) -> Quantity<f64, Meter> {
    Quantity::new_ct(value)
}

fn rectangle_item(width: f64, height: f64) -> ActualItem<f64, Meter> {
    ActualItem::new(
        ItemId::new("item"),
        "item",
        PackingShape2::Rectangle(rectangle_shape2(meters(width), meters(height)).expect("rect")),
        true,
        meters(1.0),
    )
}

fn sheet_type(code: &str, is_main: bool) -> SheetType<f64, Meter> {
    SheetType::new(meters(10.0), meters(8.0), meters(100.0), SheetTypeId::new(code))
        .with_main(is_main)
}

// ============================================================================
// 形状与面积 / Shape and area
// ============================================================================

/// 物品面积取形状实际面积 / Item area is the shape's actual area
#[test]
fn item_area_matches_shape_area() {
    let item = rectangle_item(3.0, 4.0);
    let area = shape_area(item.shape());

    assert!((area.value - 12.0).abs() < 1e-9);
    assert_eq!(area.unit.dimension_symbol(), "L^2");
}

/// 圆形物品面积是 πr² / A circular item's area is πr²
#[test]
fn circular_item_area_is_pi_r_squared() {
    let item = ActualItem::new(
        ItemId::new("circle"),
        "circle",
        PackingShape2::Circle(circle_shape2(meters(1.0)).expect("circle")),
        false,
        meters(1.0),
    );

    assert!((shape_area(item.shape()).value - PI).abs() < 1e-9);
    assert!((item.shape().bounding_width().value - 2.0).abs() < 1e-9);
}

// ============================================================================
// 层 / Layer
// ============================================================================

fn two_unit_layer(sheet_code: &str) -> Layer<f64, Meter> {
    let shape = Sheet2Shape::new(meters(10.0), meters(8.0));
    Layer::new(0, "pattern", Some(sheet_type(sheet_code, false)), shape).with_units(vec![
        Placement2::new(
            PackingShape2::Rectangle(rectangle_shape2(meters(2.0), meters(2.0)).expect("rect")),
            point2_of(meters(0.0), meters(0.0)),
        )
        .with_item_id("a"),
        Placement2::new(
            PackingShape2::Rectangle(rectangle_shape2(meters(3.0), meters(2.0)).expect("rect")),
            point2_of(meters(2.0), meters(0.0)),
        )
        .with_item_id("b"),
    ])
}

/// 层面积是放置面积之和 / Layer area is the sum of placement areas
#[test]
fn layer_area_sums_placements() {
    let layer = two_unit_layer("S1");
    assert!((layer.area().value - 10.0).abs() < 1e-9, "2×2 + 3×2 = 10");
    assert_eq!(layer.amount(), 2);
}

/// 层件数按物品编号聚合 / Layer amounts are aggregated per item identifier
#[test]
fn layer_item_amounts_aggregate_by_id() {
    let layer = two_unit_layer("S1");
    let amounts = layer.item_amounts();

    assert_eq!(amounts.get(&ItemId::new("a")), Some(&1));
    assert_eq!(amounts.get(&ItemId::new("b")), Some(&1));
}

/// 层利用率是占用面积与外接面积之比 / Layer utilisation is occupied area over bounding area
#[test]
fn layer_utilization_uses_bounding_area() {
    let layer = two_unit_layer("S1");
    // 10 / (10 × 8) = 0.125
    assert!((layer.utilization() - 0.125).abs() < 1e-9);
}

/// 空层返回零面积而不是崩溃 / An empty layer yields zero area instead of panicking
#[test]
fn empty_layer_yields_zero_area() {
    let shape = Sheet2Shape::new(meters(10.0), meters(8.0));
    let layer = Layer::new(0, "empty", None, shape);

    assert_eq!(layer.area().value, 0.0);
    assert_eq!(layer.utilization(), 0.0);
    assert_eq!(layer.amount(), 0);
}

// ============================================================================
// 列身份 / Column identity
// ============================================================================

/// 同一排布放在不同板材上是不同列 / The same layout on different sheets is a different column
///
/// 这是多板材类型能力的前提：若板材类型不参与身份，同排布会被错误去重，多类型就无法产出。
/// This is the precondition for multi sheet types: if the sheet type did not take part in the identity, the
/// same layout would be deduplicated and multiple types could never be produced.
#[test]
fn layer_identity_includes_sheet_type() {
    let small = two_unit_layer("S1");
    let large = two_unit_layer("S2");

    assert_ne!(small.identity_key(), large.identity_key());
}

/// 完全相同的层得到相同身份键 / Identical layers share the identity key
#[test]
fn identical_layers_share_identity() {
    let left = two_unit_layer("S1");
    let right = two_unit_layer("S1");

    assert_eq!(left.identity_key(), right.identity_key());
}

/// 不同排布得到不同身份键 / Different layouts get different identity keys
#[test]
fn different_layouts_differ_in_identity() {
    let left = two_unit_layer("S1");
    let mut right = two_unit_layer("S1");
    right.units[0].position = point2_of(meters(5.0), meters(5.0));

    assert_ne!(left.identity_key(), right.identity_key());
}

// ============================================================================
// 板材 / Sheet
// ============================================================================

/// 板材类型面积与装载率 / Sheet type area and loading rate
#[test]
fn sheet_type_area_and_loading_rate() {
    let sheet = sheet_type("S1", true);
    assert!((sheet.area().value - 80.0).abs() < 1e-9);

    let layer = two_unit_layer("S1");
    let rate = sheet_loading_rate(&layer.area(), &sheet);
    assert!((rate - 10.0 / 80.0).abs() < 1e-9);
}

/// 主板材标记独立于类型编码 / The main flag is independent of the type code
#[test]
fn main_sheet_flag_is_preserved() {
    assert!(sheet_type("S1", true).is_main);
    assert!(!sheet_type("S2", false).is_main);
}

// ============================================================================
// 候选与旋转 / Candidates and rotation
// ============================================================================

/// 可旋转矩形物品给出两个候选 / A rotatable rectangle item yields two candidates
#[test]
fn rotatable_item_yields_two_candidates() {
    let item = rectangle_item(2.0, 3.0);
    assert_eq!(item_candidate_shapes(&item).len(), 2);
}

/// 圆形物品与正方形只给一个候选 / Circular items and squares yield one candidate
#[test]
fn circle_item_yields_one_candidate() {
    let item = ActualItem::new(
        ItemId::new("circle"),
        "circle",
        PackingShape2::Circle(circle_shape2(meters(1.0)).expect("circle")),
        true,
        meters(1.0),
    );
    assert_eq!(item_candidate_shapes(&item).len(), 1);
    assert_eq!(item_shape_type(&item), PackingShape2Type::Circle);
}

/// 禁止旋转的矩形只给直立候选 / A non-rotatable rectangle yields the upright candidate only
#[test]
fn non_rotatable_item_yields_one_candidate() {
    let item = ActualItem::new(
        ItemId::new("fixed"),
        "fixed",
        PackingShape2::Rectangle(rectangle_shape2(meters(2.0), meters(3.0)).expect("rect")),
        false,
        meters(1.0),
    );
    assert_eq!(item_candidate_shapes(&item).len(), 1);
}

// ============================================================================
// 需求 / Demands
// ============================================================================

/// 需求条目的稳定键区分模式与键 / A demand entry's stable key separates mode and key
#[test]
fn demand_entries_have_stable_keys() {
    let amount = Bpp2dDemandEntry::item_amount("a", 5);
    assert!(amount.stable_key().contains("ItemAmount"));
    assert!(amount.stable_key().contains('a'));
    assert_eq!(amount.domain(), Bpp2dDemandDomain::Item);

    let area = Bpp2dDemandEntry::new(
        Bpp2dDemandMode::MaterialArea,
        Bpp2dDemandKey::material("m1"),
        Bpp2dDemandValue::Quantity(2.5),
    );
    assert_eq!(area.domain(), Bpp2dDemandDomain::Material);
    assert_ne!(amount.stable_key(), area.stable_key());
}

/// 件数需求按键聚合 / Piece-count demands aggregate by key
#[test]
fn item_amounts_aggregate() {
    let totals = aggregate_item_amounts(vec![
        Bpp2dDemandEntry::item_amount("a", 2),
        Bpp2dDemandEntry::item_amount("a", 3),
        Bpp2dDemandEntry::item_amount("b", 1),
    ]);

    assert_eq!(totals.get(&Bpp2dDemandKey::item("a")), Some(&5));
    assert_eq!(totals.get(&Bpp2dDemandKey::item("b")), Some(&1));
}

// ============================================================================
// 组合 / Combinations
// ============================================================================

/// 板材层组合按层身份聚合 / Sheet layer combinations aggregate by layer identity
#[test]
fn sheet_layer_combination_aggregates() {
    let layer = two_unit_layer("S1");
    let combination = SheetLayer::new(
        sheet_type("S1", true),
        Sheet2Shape::new(meters(10.0), meters(8.0)),
        vec![layer.clone()],
    );

    assert_eq!(combination.amount(), 2);
    assert!((combination.area().value - 10.0).abs() < 1e-9);
    assert!((combination.loading_rate() - 0.125).abs() < 1e-9);
    assert!(combination.identity_key().contains("S1"));
}
