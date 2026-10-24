//! 板料分配契约测试 / Sheet assignment contract tests
//!
//! 覆盖两种供给模式的分配口径：定尺按张数、卷材按段高；以及可用量校验、余料与利用率、
//! 以及全定尺时不得把张数误报为段高。
//! Covers the assignment conventions of both supply modes (sheet counts, coil segment height), availability
//! validation, remnant and utilisation, and the guarantee that an all-sheet assignment never reports its count
//! as a height.

use ospf_rust_framework_csp2d::domain::material::{
    CuttingPlan2, Material2, Pattern2, material_id2, pattern_id2,
};
use ospf_rust_framework_csp2d::domain::sheet_assignment::{SheetAssignment, assign_sheets};
use ospf_rust_framework_csp2d::infrastructure::{CuttingProcess2, region2_of};
use ospf_rust_framework_geometry2d::{
    PackingShape2, Placement2, QuantityPoint2, rectangle_shape2,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;

fn meters(value: f64) -> Quantity<f64, Meter> {
    Quantity::new_ct(value)
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

fn sheet_plan(id: &str, available: u64) -> CuttingPlan2<f64, Meter> {
    let material = Material2::sheet(material_id2("m1"), "sheet", meters(10.0), meters(8.0), available);
    let pattern = Pattern2::new(
        pattern_id2(id),
        material,
        region2_of(meters(0.0), meters(0.0), meters(10.0), meters(8.0)),
        vec![placement(2.0, 2.0, 0.0, 0.0, "a")],
        CuttingProcess2::Freeform,
    );
    CuttingPlan2::from_pattern(pattern)
}

fn coil_plan(id: &str, segment: f64) -> CuttingPlan2<f64, Meter> {
    let material = Material2::coil(material_id2("c1"), "coil", meters(10.0), 100);
    let region = material.region(Some(meters(segment))).expect("region");
    let pattern = Pattern2::new(
        pattern_id2(id),
        material,
        region,
        vec![placement(2.0, 2.0, 0.0, 0.0, "a")],
        CuttingProcess2::Freeform,
    );
    CuttingPlan2::from_pattern(pattern)
}

// ============================================================================
// 定尺分配 / Sheet assignment
// ============================================================================

/// 定尺按张数计量 / A sheet assignment counts pieces
#[test]
fn sheet_counts_pieces() {
    let assignment =
        SheetAssignment::from_usage(&[(sheet_plan("p1", 10), 3)]).expect("assignment");

    assert_eq!(assignment.len(), 1);
    assert_eq!(assignment.total_sheet_count(), 3);
    assert_eq!(assignment.total_coil_segment_count(), 0);
    // 全定尺时不得把张数当成段高。
    assert!(assignment.total_consumed_height().is_none());
}

/// 消耗面积与余料按次数累加 / Consumption and remnant accumulate by usage
#[test]
fn sheet_consumption_and_waste() {
    let assignment =
        SheetAssignment::from_usage(&[(sheet_plan("p1", 10), 2)]).expect("assignment");
    let entry = &assignment.entries[0];

    // 每张 80，用 2 张 → 160；每张余料 80 − 4 = 76 → 152。
    assert!((entry.consumed_area - 160.0).abs() < 1e-9);
    assert!((entry.waste_area - 152.0).abs() < 1e-9);
    assert!((assignment.total_consumed_area() - 160.0).abs() < 1e-9);
    assert!((assignment.total_waste_area() - 152.0).abs() < 1e-9);
}

/// 利用率与余料率互补 / Utilisation and remnant ratio are complementary
#[test]
fn utilization_and_waste_ratio() {
    let assignment =
        SheetAssignment::from_usage(&[(sheet_plan("p1", 10), 1)]).expect("assignment");
    let entry = &assignment.entries[0];

    // 成品 4 / 80 = 0.05，余料 76 / 80 = 0.95。
    assert!((entry.utilization() - 0.05).abs() < 1e-9);
    assert!((entry.waste_ratio() - 0.95).abs() < 1e-9);
    assert!((entry.utilization() + entry.waste_ratio() - 1.0).abs() < 1e-9);
}

/// 超出可用量被拒绝 / Exceeding availability is rejected
#[test]
fn exceeding_availability_is_rejected() {
    // 可用 2 张，用 5 张。
    let plans = vec![(sheet_plan("p1", 2), 5)];
    assert!(assign_sheets(&plans).is_err());

    // 未校验的构造仍能看到超限标记。
    let assignment = SheetAssignment::from_usage(&plans).expect("assignment");
    assert!(assignment.entries[0].exceeds_availability());
}

/// 零使用量不产生条目 / Zero usage produces no entry
#[test]
fn zero_usage_produces_no_entry() {
    let assignment =
        SheetAssignment::from_usage(&[(sheet_plan("p1", 10), 0)]).expect("assignment");
    assert!(assignment.is_empty());
}

/// 空使用量得到空分配 / Empty usage yields an empty assignment
#[test]
fn empty_usage_is_empty_assignment() {
    let assignment = SheetAssignment::from_usage(&[]).expect("assignment");
    assert!(assignment.is_empty());
    assert_eq!(assignment.total_sheet_count(), 0);
    assert_eq!(assignment.total_consumed_height(), None);
}

// ============================================================================
// 卷材分配 / Coil assignment
// ============================================================================

/// 卷材按段高计量 / A coil assignment counts segment height
#[test]
fn coil_accumulates_segment_height() {
    let assignment =
        SheetAssignment::from_usage(&[(coil_plan("c", 4.0), 3)]).expect("assignment");

    assert_eq!(assignment.total_coil_segment_count(), 3);
    assert_eq!(assignment.total_sheet_count(), 0);
    // 段高 4 × 3 = 12。
    assert!((assignment.total_consumed_height().expect("height") - 12.0).abs() < 1e-9);
}

/// 混用时只累加卷材段高 / With mixed materials only coil heights accumulate
#[test]
fn mixed_assignment_accumulates_only_coil_height() {
    let assignment = SheetAssignment::from_usage(&[
        (sheet_plan("p1", 10), 2),
        (coil_plan("c", 5.0), 1),
    ])
    .expect("assignment");

    assert_eq!(assignment.len(), 2);
    assert_eq!(assignment.total_sheet_count(), 2);
    assert_eq!(assignment.total_coil_segment_count(), 1);
    assert!((assignment.total_consumed_height().expect("height") - 5.0).abs() < 1e-9);
}

/// 按编号查询条目 / Look up an entry by identifier
#[test]
fn lookup_entry_by_material_id() {
    let assignment =
        SheetAssignment::from_usage(&[(sheet_plan("p1", 10), 1)]).expect("assignment");

    assert!(assignment.entry_of(&material_id2("m1")).is_some());
    assert!(assignment.entry_of(&material_id2("absent")).is_none());
}
