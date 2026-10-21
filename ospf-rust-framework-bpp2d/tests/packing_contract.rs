//! 装箱契约测试 / Packing contract tests
//!
//! 对应 Kotlin `PackerContractTest`、`PackingGeometryGuardContractTest` 与 `KerfAndRemnantTest`：
//! 几何守卫（越界、重叠、旋转规则）、刀缝双向膨胀、余料报告（互不重叠且面积守恒）、
//! 装载顺序与渲染 DTO。
//! Mirrors Kotlin `PackerContractTest`, `PackingGeometryGuardContractTest` and `KerfAndRemnantTest`:
//! the geometry guard (out-of-bounds, overlap, rotation rule), double-sided kerf inflation, remnant reporting
//! (disjoint with conserved area), loading order and render DTOs.

use ospf_rust_framework_bpp2d::domain::item::model::{
    BatchNo, ItemId, Layer, Sheet, SheetLayer, SheetType, SheetTypeId,
};
use ospf_rust_framework_bpp2d::domain::packing::{
    FreeSpaceAnalyzer, LoadingOrderCalculator, PackedLayer, PackingAggregation, PackingContext,
    PackingGeometryGuard, PackingRendererAdapter, PackingResult, Packer, PackingRotationRule,
    Remnant2, is_rotation_invariant, loading_rate_of,
};
use ospf_rust_framework_geometry2d::{
    Box2Geometry, PackingShape2, Placement2, QuantityPoint2, Sheet2Shape, circle_shape2,
    rectangle_shape2,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;

fn meters(value: f64) -> Quantity<f64, Meter> {
    Quantity::new_ct(value)
}

fn rect(width: f64, height: f64) -> PackingShape2<f64, Meter> {
    PackingShape2::Rectangle(rectangle_shape2(meters(width), meters(height)).expect("rect"))
}

fn circle(radius: f64) -> PackingShape2<f64, Meter> {
    PackingShape2::Circle(circle_shape2(meters(radius)).expect("circle"))
}

fn placement(shape: PackingShape2<f64, Meter>, x: f64, y: f64) -> Placement2<f64, Meter> {
    Placement2::new(shape, QuantityPoint2 {
        x: meters(x),
        y: meters(y),
    })
}

fn sheet_type() -> SheetType<f64, Meter> {
    SheetType::new(meters(10.0), meters(8.0), meters(100.0), SheetTypeId::new("S1"))
}

// ============================================================================
// 几何守卫 / Geometry guard
// ============================================================================

/// 板材内的合法放置通过校验 / A legal placement inside the sheet passes
#[test]
fn inside_placement_passes() {
    let sheet = Sheet2Shape::new(meters(10.0), meters(8.0));
    let placements = vec![placement(rect(2.0, 2.0), 0.0, 0.0)];

    assert!(
        PackingGeometryGuard::require_sheet_geometry(&sheet, &placements, "S1", "test", None)
            .is_ok()
    );
}

/// 越界放置被拒绝 / An out-of-bounds placement is rejected
#[test]
fn outside_placement_is_rejected() {
    let sheet = Sheet2Shape::new(meters(10.0), meters(8.0));
    let placements = vec![placement(rect(2.0, 2.0), 9.0, 0.0)];

    let error = PackingGeometryGuard::require_sheet_geometry(&sheet, &placements, "S1", "test", None)
        .expect_err("must be rejected");
    assert!(error.to_string().contains("越出板材") || error.to_string().contains("outside"));
}

/// 贴边放置合法 / A placement touching the boundary is legal
#[test]
fn touching_boundary_is_legal() {
    let sheet = Sheet2Shape::new(meters(10.0), meters(8.0));
    let placements = vec![placement(rect(2.0, 2.0), 8.0, 6.0)];

    assert!(
        PackingGeometryGuard::require_sheet_geometry(&sheet, &placements, "S1", "test", None)
            .is_ok()
    );
}

/// 相切不算重叠 / Tangency does not count as overlap
#[test]
fn tangent_placements_are_legal() {
    let sheet = Sheet2Shape::new(meters(10.0), meters(8.0));
    let placements = vec![
        placement(rect(2.0, 2.0), 0.0, 0.0),
        placement(rect(2.0, 2.0), 2.0, 0.0),
    ];

    assert!(
        PackingGeometryGuard::require_sheet_geometry(&sheet, &placements, "S1", "test", None)
            .is_ok()
    );
}

/// 内部相交被判为重叠 / Interior intersection is rejected as overlap
#[test]
fn overlapping_placements_are_rejected() {
    let sheet = Sheet2Shape::new(meters(10.0), meters(8.0));
    let placements = vec![
        placement(rect(2.0, 2.0), 0.0, 0.0),
        placement(rect(2.0, 2.0), 1.0, 1.0),
    ];

    let error = PackingGeometryGuard::require_sheet_geometry(&sheet, &placements, "S1", "test", None)
        .expect_err("must be rejected");
    assert!(error.to_string().contains("重叠") || error.to_string().contains("overlap"));
}

/// 刀缝双向膨胀拒绝过近的放置 / Double-sided kerf inflation rejects a narrow gap
///
/// 真实间隙 0.3 < 刀缝 0.5：单向膨胀同样能发现，但双向检查保证另一侧的不足也可见。
/// The real gap is 0.3, below the kerf 0.5: one-sided inflation also finds it, and the double-sided check
/// guarantees a narrow gap on either side is visible.
#[test]
fn kerf_rejects_narrow_gap() {
    let sheet = Sheet2Shape::new(meters(10.0), meters(8.0));
    let placements = vec![
        placement(rect(2.0, 2.0), 0.0, 0.0),
        placement(rect(2.0, 2.0), 2.3, 0.0),
    ];

    // 无刀缝时合法（相切以上）。
    assert!(
        PackingGeometryGuard::require_sheet_geometry(&sheet, &placements, "S1", "test", None)
            .is_ok()
    );
    // 带 0.5 刀缝时被拒绝。
    assert!(
        PackingGeometryGuard::require_sheet_geometry(
            &sheet,
            &placements,
            "S1",
            "test",
            Some(&meters(0.5))
        )
        .is_err()
    );
}

/// 刀缝足够时合法放置通过 / A wide enough gap passes with the kerf
#[test]
fn kerf_allows_wide_gap() {
    let sheet = Sheet2Shape::new(meters(10.0), meters(8.0));
    let placements = vec![
        placement(rect(2.0, 2.0), 0.0, 0.0),
        placement(rect(2.0, 2.0), 2.6, 0.0),
    ];

    assert!(
        PackingGeometryGuard::require_sheet_geometry(
            &sheet,
            &placements,
            "S1",
            "test",
            Some(&meters(0.5))
        )
        .is_ok()
    );
}

/// 未允许旋转的放置被拒绝 / A placement with a disallowed rotation is rejected
#[test]
fn disallowed_rotation_is_rejected() {
    let sheet = Sheet2Shape::new(meters(10.0), meters(8.0));
    let mut rotated = placement(rect(2.0, 3.0), 0.0, 0.0);
    rotated.rotated = true;

    let error = PackingGeometryGuard::require_sheet_geometry_with_rule(
        &sheet,
        &[rotated],
        "S1",
        "test",
        None,
        &PackingRotationRule::SymmetricOnly,
    )
    .expect_err("must be rejected");
    assert!(error.to_string().contains("旋转") || error.to_string().contains("rotation"));
}

/// 旋转不变的形状允许旋转 / A rotation-invariant shape may rotate
#[test]
fn rotation_invariant_shape_is_allowed() {
    assert!(is_rotation_invariant(&circle(1.0)));
    assert!(is_rotation_invariant(&rect(2.0, 2.0)));
    assert!(!is_rotation_invariant(&rect(2.0, 3.0)));

    let sheet = Sheet2Shape::new(meters(10.0), meters(8.0));
    let mut rotated = placement(rect(2.0, 2.0), 0.0, 0.0);
    rotated.rotated = true;
    assert!(
        PackingGeometryGuard::require_sheet_geometry_with_rule(
            &sheet,
            &[rotated],
            "S1",
            "test",
            None,
            &PackingRotationRule::SymmetricOnly,
        )
        .is_ok()
    );
}

/// 物品目录规则允许缺失物品旋转 / The catalogue rule allows rotation for missing items
#[test]
fn catalogue_rule_allows_missing_items() {
    let mut rotated = placement(rect(2.0, 3.0), 0.0, 0.0);
    rotated.rotated = true;
    rotated.item_id = Some("missing".to_string());

    let rule = PackingRotationRule::ByItemCatalog(std::collections::BTreeMap::new());
    assert!(rule.allow(&rotated));
}

// ============================================================================
// 余料分析 / Remnant analysis
// ============================================================================

/// 空板材整块为一块余料 / An empty sheet is one whole remnant
#[test]
fn empty_sheet_yields_whole_remnant() {
    let analyzer = FreeSpaceAnalyzer::new();
    let remnants = analyzer.analyze_sheet("S1", &sheet_type(), &[]);

    assert_eq!(remnants.len(), 1);
    assert!((remnants[0].area.value - 80.0).abs() < 1e-9);
    assert_eq!(remnants[0].sheet_name, "S1");
}

/// 余料矩形互不重叠 / Remnant rectangles never overlap
#[test]
fn remnants_are_disjoint() {
    let analyzer = FreeSpaceAnalyzer::new();
    let placements = vec![
        placement(rect(2.0, 2.0), 0.0, 0.0),
        placement(rect(2.0, 2.0), 4.0, 0.0),
        placement(rect(2.0, 2.0), 0.0, 4.0),
    ];
    let remnants = analyzer.analyze_sheet("S1", &sheet_type(), &placements);

    assert!(!remnants.is_empty());
    for (index, left) in remnants.iter().enumerate() {
        for right in remnants.iter().skip(index + 1) {
            assert!(
                !left.rectangle.overlapped(&right.rectangle),
                "remnants must not overlap"
            );
        }
    }
}

/// 余料面积与占用面积之和等于板材面积 / Remnant area plus occupied area equals the sheet area
#[test]
fn remnant_area_is_conserved() {
    let analyzer = FreeSpaceAnalyzer::new();
    let placements = vec![
        placement(rect(2.0, 2.0), 0.0, 0.0),
        placement(rect(3.0, 2.0), 4.0, 0.0),
    ];
    let remnants = analyzer.analyze_sheet("S1", &sheet_type(), &placements);

    let remnant_area: f64 = remnants.iter().map(|remnant| remnant.area.value).sum();
    let occupied: f64 = placements
        .iter()
        .map(|placement| placement.shape.area().value)
        .sum();

    assert!(
        (remnant_area + occupied - 80.0).abs() < 1e-6,
        "remnant={remnant_area}, occupied={occupied}"
    );
}

/// 余料按面积降序 / Remnants are ordered by descending area
#[test]
fn remnants_sorted_by_area() {
    let analyzer = FreeSpaceAnalyzer::new();
    let placements = vec![
        placement(rect(2.0, 2.0), 0.0, 0.0),
        placement(rect(2.0, 2.0), 4.0, 0.0),
    ];
    let remnants = analyzer.analyze_sheet("S1", &sheet_type(), &placements);

    let areas: Vec<f64> = remnants.iter().map(|remnant| remnant.area.value).collect();
    let mut sorted = areas.clone();
    sorted.sort_by(|lhs, rhs| rhs.partial_cmp(lhs).unwrap());
    assert_eq!(areas, sorted);
}

/// 余料统计汇总数量与面积 / The summary reports count and total area
#[test]
fn remnant_summary() {
    let analyzer = FreeSpaceAnalyzer::new();
    let remnants = analyzer.analyze_sheet("S1", &sheet_type(), &[]);
    let (count, area) = FreeSpaceAnalyzer::summarize(&remnants);

    assert_eq!(count, 1);
    assert!((area - 80.0).abs() < 1e-9);
}

// ============================================================================
// 装载顺序与装箱 / Loading order and packing
// ============================================================================

/// 装载顺序先下后左 / The loading order is bottom-first then left-to-right
#[test]
fn loading_order_bottom_then_left() {
    let placements = vec![
        placement(rect(2.0, 2.0), 4.0, 0.0),
        placement(rect(2.0, 2.0), 0.0, 0.0),
        placement(rect(2.0, 2.0), 0.0, 2.0),
    ];
    let order = LoadingOrderCalculator::new().order(&placements);

    // (0,0) 第 0；(4,0) 第 1；(0,2) 第 2。
    assert_eq!(order, vec![1, 0, 2]);
}

/// 装载率按占用面积与板材面积计算 / The loading rate uses occupied over sheet area
#[test]
fn loading_rate_uses_areas() {
    assert!((loading_rate_of(40.0, 80.0) - 0.5).abs() < 1e-9);
    assert_eq!(loading_rate_of(40.0, 0.0), 0.0);
}

fn build_sheet() -> Sheet<f64, Meter> {
    let layer = Layer::new(0, "test", Some(sheet_type()), Sheet2Shape::new(meters(10.0), meters(8.0)))
        .with_units(vec![
            placement(rect(2.0, 2.0), 0.0, 0.0),
            placement(rect(2.0, 2.0), 2.0, 0.0),
        ]);
    let sheet_layer = SheetLayer::new(
        sheet_type(),
        Sheet2Shape::new(meters(10.0), meters(8.0)),
        vec![layer],
    );
    Sheet::new(sheet_type())
        .with_layers(vec![sheet_layer])
        .with_batch(BatchNo::new("b1"))
}

/// 装箱结果展开层与物品 / The packing result expands layers and items
#[test]
fn packer_expands_layers_and_items() {
    let sheet = build_sheet();
    let result = Packer::new()
        .invoke(&[sheet], &PackingAggregation::new("test"), None)
        .expect("packing");

    assert_eq!(result.sheet_count(), 1);
    assert_eq!(result.total_amount(), 2);
    assert_eq!(result.sheets[0].layers.len(), 1);
    assert_eq!(result.sheets[0].items.len(), 2);
    assert!((result.sheets[0].loading_rate - 8.0 / 80.0).abs() < 1e-9);
}

/// 装箱结果为每块板材生成余料 / Packing yields remnants per sheet
#[test]
fn packer_collects_remnants() {
    let sheet = build_sheet();
    let result = Packer::new()
        .invoke(&[sheet], &PackingAggregation::new("test"), None)
        .expect("packing");

    assert!(result.remnant_count() > 0);
    // 占用 8，余料应为 72。
    assert!((result.occupied_area() - 8.0).abs() < 1e-9);
    assert!((result.remnant_area() - 72.0).abs() < 1e-6);
}

/// 装箱器拒绝几何非法的板材 / The packer rejects a geometrically illegal sheet
#[test]
fn packer_rejects_illegal_geometry() {
    let layer = Layer::new(
        0,
        "test",
        Some(sheet_type()),
        Sheet2Shape::new(meters(10.0), meters(8.0)),
    )
    .with_units(vec![
        // 越界：x=9 加宽 2 超过板宽 10。
        placement(rect(2.0, 2.0), 9.0, 0.0),
    ]);
    let sheet_layer = SheetLayer::new(
        sheet_type(),
        Sheet2Shape::new(meters(10.0), meters(8.0)),
        vec![layer],
    );
    let sheet = Sheet::new(sheet_type()).with_layers(vec![sheet_layer]);

    assert!(
        Packer::new()
            .invoke(&[sheet], &PackingAggregation::new("test"), None)
            .is_err()
    );
}

/// 物品级件数映射统计全部物品 / The item amount map counts every item
#[test]
fn item_amounts_are_counted() {
    let mut first = placement(rect(2.0, 2.0), 0.0, 0.0);
    first.item_id = Some("a".to_string());
    let mut second = placement(rect(2.0, 2.0), 2.0, 0.0);
    second.item_id = Some("a".to_string());
    let layer = Layer::new(
        0,
        "test",
        Some(sheet_type()),
        Sheet2Shape::new(meters(10.0), meters(8.0)),
    )
    .with_units(vec![first, second]);
    let sheet_layer = SheetLayer::new(
        sheet_type(),
        Sheet2Shape::new(meters(10.0), meters(8.0)),
        vec![layer],
    );
    let sheet = Sheet::new(sheet_type()).with_layers(vec![sheet_layer]);

    let result = Packer::new()
        .invoke(&[sheet], &PackingAggregation::new("test"), None)
        .expect("packing");
    let amounts = result.item_amounts();

    assert_eq!(amounts.get(&ItemId::new("a")), Some(&2));
}

// ============================================================================
// 渲染适配 / Render adaptation
// ============================================================================

/// 渲染 DTO 输出板材、层与放置 / The render DTO carries sheets, layers and placements
#[test]
fn renderer_maps_packing_result() {
    let sheet = build_sheet();
    let result = Packer::new()
        .invoke(&[sheet], &PackingAggregation::new("test"), None)
        .expect("packing");
    let dto = PackingRendererAdapter::new().to_render(&result);

    assert_eq!(dto.sheets.len(), 1);
    assert_eq!(dto.total_amount, 2);
    assert_eq!(dto.sheets[0].layers.len(), 1);
    assert_eq!(dto.sheets[0].layers[0].placements.len(), 2);
    assert_eq!(dto.sheets[0].sheet_type_code, "S1");
}

/// 圆形件输出半径，矩形件不输出 / Circle items emit a radius; rectangles do not
#[test]
fn renderer_emits_circle_radius() {
    let layer = Layer::new(
        0,
        "test",
        Some(sheet_type()),
        Sheet2Shape::new(meters(10.0), meters(8.0)),
    )
    .with_units(vec![
        placement(circle(1.0), 0.0, 0.0),
        placement(rect(2.0, 2.0), 4.0, 0.0),
    ]);
    let sheet_layer = SheetLayer::new(
        sheet_type(),
        Sheet2Shape::new(meters(10.0), meters(8.0)),
        vec![layer],
    );
    let sheet = Sheet::new(sheet_type()).with_layers(vec![sheet_layer]);
    let result = Packer::new()
        .invoke(&[sheet], &PackingAggregation::new("test"), None)
        .expect("packing");
    let dto = PackingRendererAdapter::new().to_render(&result);
    let placements = &dto.sheets[0].layers[0].placements;

    let circle_dto = placements
        .iter()
        .find(|placement| placement.radius.is_some())
        .expect("circle placement");
    assert!((circle_dto.radius.unwrap() - 1.0).abs() < 1e-9);

    let rect_dto = placements
        .iter()
        .find(|placement| placement.radius.is_none())
        .expect("rectangle placement");
    assert!((rect_dto.width - 2.0).abs() < 1e-9);
}

/// 装箱上下文与聚合可构造 / The packing context and aggregation are constructible
#[test]
fn packing_context_is_constructible() {
    let context = PackingContext::new().with_name("c");
    assert_eq!(context.name.as_deref(), Some("c"));
    let aggregation = PackingAggregation::new("a");
    assert_eq!(aggregation.name, "a");
}
