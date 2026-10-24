//! 物料与图案契约测试 / Material and pattern contract tests
//!
//! 对应 Kotlin `MaterialPatternContractTest`：双模式物料（卷材/定尺）、料域构造、产品校验与方向、
//! 图案面积口径（成品/副产物/余料/利用率）、图案校验（退化、工艺与切分树一致、空放置）、
//! 方案消耗口径与余料阈值。
//! Mirrors Kotlin `MaterialPatternContractTest`: dual-mode materials (coil/sheet), region construction,
//! product validation and orientations, pattern area conventions (finished/by-product/remnant/utilisation),
//! pattern validation (degenerate, process versus cut tree, empty placement), plan consumption and remnant
//! thresholds.

use ospf_rust_framework_csp2d::domain::material::{
    Costar2, CuttingPlan2, DemandMode2, DemandValue2, Material2, Material2Kind, Pattern2,
    Product2, ProductDemand2, ProductDemands2, costar_id2, material_id2, pattern_id2, plan_id2,
    product_id2, remnant_reaches,
};
use ospf_rust_framework_csp2d::infrastructure::{
    CutLine2, CutOrientation2, CutTree2, CuttingProcess2, region2_of,
};
use ospf_rust_framework_geometry2d::{PackingShape2, Placement2, QuantityPoint2, rectangle_shape2};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::{Meter, SquareMeter};
use ospf_rust_quantities::unit::physical_unit::CTUnit;

fn meters(value: f64) -> Quantity<f64, Meter> {
    Quantity::new_ct(value)
}

fn sheet_material(width: f64, height: f64) -> Material2<f64, Meter> {
    Material2::sheet(
        material_id2("m1"),
        "sheet",
        meters(width),
        meters(height),
        10,
    )
}

fn coil_material(width: f64) -> Material2<f64, Meter> {
    Material2::coil(material_id2("c1"), "coil", meters(width), 5)
}

fn product(id: &str, width: f64, height: f64) -> Product2<f64, Meter> {
    Product2::new(product_id2(id), id, meters(width), meters(height))
}

fn placement(width: f64, height: f64, x: f64, y: f64, item: &str) -> Placement2<f64, Meter> {
    Placement2::new(
        PackingShape2::Rectangle(rectangle_shape2(meters(width), meters(height)).expect("rect")),
        QuantityPoint2 {
            x: meters(x),
            y: meters(y),
        },
    )
    .with_item_id(item)
}

fn pattern(material: Material2<f64, Meter>, width: f64, height: f64) -> Pattern2<f64, Meter> {
    Pattern2::new(
        pattern_id2("p1"),
        material,
        region2_of(meters(0.0), meters(0.0), meters(width), meters(height)),
        vec![placement(2.0, 2.0, 0.0, 0.0, "prod-1")],
        CuttingProcess2::Freeform,
    )
}

// ============================================================================
// 双模式物料 / Dual-mode material
// ============================================================================

/// 定尺板材与卷材由板高区分 / Sheet and coil are distinguished by the sheet height
#[test]
fn material_kind_is_decided_by_height() {
    let sheet = sheet_material(10.0, 8.0);
    let coil = coil_material(10.0);

    assert_eq!(sheet.kind(), Material2Kind::Sheet);
    assert!(!sheet.is_coil());
    assert_eq!(coil.kind(), Material2Kind::Coil);
    assert!(coil.is_coil());
}

/// 定尺板材有面积，卷材没有 / A sheet has an area; a coil does not
#[test]
fn sheet_area_only_exists_for_sheets() {
    assert!((sheet_material(10.0, 8.0).sheet_area().expect("area").value - 80.0).abs() < 1e-9);
    assert!(coil_material(10.0).sheet_area().is_none());
}

/// 定尺板材的料域就是整块板 / A sheet's region is the whole sheet
#[test]
fn sheet_region_is_the_whole_sheet() {
    let region = sheet_material(10.0, 8.0).region(None).expect("region");

    assert_eq!(region.width.value, 10.0);
    assert_eq!(region.height.value, 8.0);
    assert!((region.area().value - 80.0).abs() < 1e-9);
}

/// 卷材料域必须给段高 / A coil region requires a segment height
///
/// 未给段高时返回 `None`，不构造非法区域。
/// Returns `None` without a segment height instead of building an illegal region.
#[test]
fn coil_region_requires_segment_height() {
    let coil = coil_material(10.0);

    assert!(coil.region(None).is_none());
    let region = coil.region(Some(meters(4.0))).expect("region");
    assert_eq!(region.width.value, 10.0);
    assert_eq!(region.height.value, 4.0);
}

/// 物料校验拒绝非正尺寸与零可用数量 / Material validation rejects non-positive sizes and zero availability
#[test]
fn material_validation() {
    assert!(sheet_material(10.0, 8.0).validate().is_ok());

    let mut bad = sheet_material(10.0, 8.0);
    bad.width = meters(0.0);
    assert!(bad.validate().is_err());

    let mut unavailable = sheet_material(10.0, 8.0);
    unavailable.available_batches = 0;
    assert!(unavailable.validate().is_err());
}

// ============================================================================
// 产品与需求 / Products and demands
// ============================================================================

/// 产品面积与校验 / Product area and validation
#[test]
fn product_area_and_validation() {
    let product = product("p", 3.0, 4.0);
    assert!((product.area().value - 12.0).abs() < 1e-9);
    assert!(product.validate().is_ok());

    let mut bad = product.clone();
    bad.height = meters(-1.0);
    assert!(bad.validate().is_err());
}

/// 允许旋转时给出两个方向 / An allow-rotate product yields two orientations
#[test]
fn orientations_follow_allow_rotate() {
    let rotatable = product("p", 3.0, 4.0);
    assert_eq!(rotatable.orientations(), vec![false, true]);

    let fixed = product("q", 3.0, 4.0).with_allow_rotate(false);
    assert_eq!(fixed.orientations(), vec![false]);
}

/// 按方向给出宽高 / Size follows the orientation
#[test]
fn size_follows_orientation() {
    let product = product("p", 3.0, 4.0);

    let (width, height) = product.size_of(false);
    assert_eq!(width.value, 3.0);
    assert_eq!(height.value, 4.0);

    let (width, height) = product.size_of(true);
    assert_eq!(width.value, 4.0);
    assert_eq!(height.value, 3.0);
}

/// 需求量值口径与求解器数值 / Demand value mode and solver number
#[test]
fn demand_value_modes() {
    let amount = DemandValue2::Amount(5);
    assert_eq!(amount.mode(), DemandMode2::Amount);
    assert_eq!(amount.to_solver(), 5.0);
    assert!(amount.is_positive());

    let area = DemandValue2::Area(12.5);
    assert_eq!(area.mode(), DemandMode2::Area);
    assert_eq!(area.to_solver(), 12.5);

    assert!(!DemandValue2::Amount(0).is_positive());
}

/// 需求集合的件数上限 / The demand set's piece cap
#[test]
fn demand_set_piece_cap() {
    let demands = ProductDemands2::new()
        .with(ProductDemand2::amount(product("a", 2.0, 2.0), 10))
        // 面积 100 / 单件 4 = 25 件。
        .with(ProductDemand2::area(product("b", 2.0, 2.0), 100.0));

    assert_eq!(demands.piece_cap(), 25);
    assert_eq!(ProductDemands2::<f64, Meter>::new().piece_cap(), 0);
}

// ============================================================================
// 图案面积口径 / Pattern area conventions
// ============================================================================

/// 成品面积、余料与利用率 / Finished area, remnant and utilisation
#[test]
fn pattern_area_conventions() {
    let pattern = pattern(sheet_material(10.0, 8.0), 10.0, 8.0);

    assert!((pattern.used_area().value - 4.0).abs() < 1e-9);
    assert!((pattern.costar_area().value - 0.0).abs() < 1e-9);
    // 料域 80 − 成品 4 = 76。
    assert!((pattern.waste_area().value - 76.0).abs() < 1e-9);
    // 利用率只看成品：4 / 80。
    assert!((pattern.utilization() - 4.0 / 80.0).abs() < 1e-9);
}

/// 副产物计入余料但不计入利用率 / By-products count toward the remnant but not the utilisation
#[test]
fn costars_affect_waste_not_utilization() {
    let costar = Costar2::new(costar_id2("c"), "c", meters(2.0), meters(2.0));
    assert!((costar.area().value - 4.0).abs() < 1e-9);

    let pattern = pattern(sheet_material(10.0, 8.0), 10.0, 8.0)
        .with_costars(vec![placement(2.0, 2.0, 4.0, 0.0, "costar-1")]);

    assert!((pattern.costar_area().value - 4.0).abs() < 1e-9);
    // 余料 80 − 4 − 4 = 72。
    assert!((pattern.waste_area().value - 72.0).abs() < 1e-9);
    // 利用率仍只看成品 4。
    assert!((pattern.utilization() - 4.0 / 80.0).abs() < 1e-9);
}

/// 成品件数映射 / Finished part counts
#[test]
fn pattern_product_amounts() {
    let mut pattern = pattern(sheet_material(10.0, 8.0), 10.0, 8.0);
    pattern.placements = vec![
        placement(2.0, 2.0, 0.0, 0.0, "prod-1"),
        placement(2.0, 2.0, 2.0, 0.0, "prod-1"),
        placement(2.0, 2.0, 4.0, 0.0, "prod-2"),
    ];

    let amounts = pattern.product_amounts();
    assert_eq!(amounts.get(&product_id2("prod-1")), Some(&2));
    assert_eq!(amounts.get(&product_id2("prod-2")), Some(&1));
}

/// 图案身份键区分排布与物料 / The pattern identity key separates layouts and materials
#[test]
fn pattern_identity_key() {
    let first = pattern(sheet_material(10.0, 8.0), 10.0, 8.0);
    let mut moved = first.clone();
    moved.placements[0].position = QuantityPoint2 {
        x: meters(5.0),
        y: meters(5.0),
    };

    assert_ne!(first.identity_key(), moved.identity_key());
    assert_eq!(first.identity_key(), first.clone().identity_key());
}

// ============================================================================
// 图案校验 / Pattern validation
// ============================================================================

/// 合法图案通过校验 / A legal pattern validates
#[test]
fn legal_pattern_validates() {
    assert!(
        pattern(sheet_material(10.0, 8.0), 10.0, 8.0)
            .validate(&meters(0.0))
            .is_ok()
    );
}

/// 退化料域被拒绝 / A degenerate region is rejected
#[test]
fn degenerate_region_is_rejected() {
    let pattern = pattern(sheet_material(10.0, 8.0), 0.0, 8.0);
    assert!(pattern.validate(&meters(0.0)).is_err());
}

/// 断头台工艺缺少切分树被拒绝 / A guillotine process without a cut tree is rejected
#[test]
fn guillotine_without_cut_tree_is_rejected() {
    let mut pattern = pattern(sheet_material(10.0, 8.0), 10.0, 8.0);
    pattern.process = CuttingProcess2::GuillotineTwoStage;

    assert!(pattern.validate(&meters(0.0)).is_err());
}

/// 空放置被拒绝 / An empty placement list is rejected
#[test]
fn empty_placement_is_rejected() {
    let mut pattern = pattern(sheet_material(10.0, 8.0), 10.0, 8.0);
    pattern.placements.clear();

    assert!(pattern.validate(&meters(0.0)).is_err());
}

/// 带合法切分树的断头台图案通过校验 / A guillotine pattern with a valid cut tree validates
#[test]
fn guillotine_with_cut_tree_validates() {
    let material = sheet_material(10.0, 8.0);
    let root = region2_of(meters(0.0), meters(0.0), meters(10.0), meters(8.0));
    let (left, right) = root
        .cut(CutOrientation2::Vertical, &meters(4.0), &meters(0.0))
        .expect("cut");
    let tree = CutTree2::cut(
        CutLine2 {
            orientation: CutOrientation2::Vertical,
            position: meters(4.0),
            stage: 1,
        },
        root,
        meters(0.0),
        CutTree2::leaf(left, Some("prod-1".to_string())),
        CutTree2::leaf(right, None),
    )
    .expect("tree");

    let pattern = Pattern2::new(
        pattern_id2("p1"),
        material,
        region2_of(meters(0.0), meters(0.0), meters(10.0), meters(8.0)),
        vec![placement(2.0, 2.0, 0.0, 0.0, "prod-1")],
        CuttingProcess2::GuillotineTwoStage,
    )
    .with_cut_tree(tree);

    assert!(pattern.validate(&meters(0.0)).is_ok());
}

// ============================================================================
// 裁切方案 / Cutting plan
// ============================================================================

/// 方案消耗面积与余料 / Plan consumption and remnant
#[test]
fn plan_consumption_and_waste() {
    let plan = CuttingPlan2::new(plan_id2("plan-1"), pattern(sheet_material(10.0, 8.0), 10.0, 8.0));

    assert!((plan.consumed_area().value - 80.0).abs() < 1e-9);
    assert!((plan.waste_area().value - 76.0).abs() < 1e-9);
    // 定尺板材没有段高口径。
    assert!(plan.consumed_height().is_none());
}

/// 卷材方案给出消耗段高 / A coil plan reports the consumed segment height
#[test]
fn coil_plan_reports_segment_height() {
    let coil = coil_material(10.0);
    let region = coil.region(Some(meters(4.0))).expect("region");
    let pattern = Pattern2::new(
        pattern_id2("p-coil"),
        coil,
        region,
        vec![placement(2.0, 2.0, 0.0, 0.0, "prod-1")],
        CuttingProcess2::Freeform,
    );
    let plan = CuttingPlan2::from_pattern(pattern);

    assert_eq!(plan.consumed_height().expect("height").value, 4.0);
    assert!(plan.id.as_str().starts_with("plan-"));
}

/// 方案身份键与校验 / Plan identity key and validation
#[test]
fn plan_identity_and_validation() {
    let plan = CuttingPlan2::new(plan_id2("plan-1"), pattern(sheet_material(10.0, 8.0), 10.0, 8.0));

    assert!(plan.validate(&meters(0.0)).is_ok());
    assert!(plan.identity_key().contains("m1"));
}

// ============================================================================
// 余料阈值 / Remnant threshold
// ============================================================================

/// 余料阈值判定 / Remnant threshold decision
#[test]
fn remnant_threshold() {
    let area = Quantity::new(12.0, SquareMeter::INSTANT.clone());

    // 无阈值时任何正面积都可用。
    // U 只出现在阈值参数上，需要显式标注。
    // `U` only appears on the threshold parameter, so it must be annotated explicitly.
    assert!(remnant_reaches::<f64, SquareMeter>(&area, None));
    // 阈值 10：12 达到。
    let threshold = Quantity::new(10.0, SquareMeter::INSTANT.clone());
    assert!(remnant_reaches::<f64, SquareMeter>(&area, Some(&threshold)));
    // 阈值 20：12 未达到。
    let high = Quantity::new(20.0, SquareMeter::INSTANT.clone());
    assert!(!remnant_reaches::<f64, SquareMeter>(&area, Some(&high)));
    // 零面积永远不可用。
    let zero = Quantity::new(0.0, SquareMeter::INSTANT.clone());
    assert!(!remnant_reaches::<f64, SquareMeter>(&zero, None));
}
