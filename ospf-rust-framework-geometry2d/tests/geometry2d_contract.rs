//! 二维几何契约测试 / 2D geometry contract tests
//!
//! 对应 Kotlin `Geometry2CoreContractTest`：面积口径、内部重叠语义、刀缝膨胀与节距、板材包含。
//! Mirrors Kotlin `Geometry2CoreContractTest`: area conventions, interior-overlap semantics, kerf
//! inflation and pitch, sheet containment.

use ospf_rust_framework_geometry2d::*;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::derived::{Meter, SquareMeter};
use ospf_rust_quantities::unit::physical_unit::CTUnit;
use std::f64::consts::PI;

fn meters(value: f64) -> Quantity<f64, Meter> {
    Quantity::new_ct(value)
}

// ============================================================================
// 面积口径 / Area convention
// ============================================================================

/// 矩形面积即宽高乘积 / A rectangle area is the product of width and height
#[test]
fn rectangle_area_is_width_times_height() {
    let shape = rectangle_shape2(meters(3.0), meters(4.0)).expect("rectangle");
    let area = area_of(&PackingShape2::Rectangle(shape.clone()));

    assert!((area.value - 12.0).abs() < 1e-9);
    assert_eq!(area.unit.dimension_symbol(), "L^2");
    // 与 SquareMeter 同量纲，说明宽度乘高度确实得到面积而不是长度或体积。
    // Same dimension as SquareMeter, proving width × height yields an area rather than a length or volume.
    let square = SquareMeter::INSTANT.clone();
    assert_eq!(area.unit.dimension_symbol(), square.dimension_symbol());
}

/// 圆形面积是 πr²，外接面积是 4r² / A circle area is πr² and its bounding area is 4r²
#[test]
fn circle_area_and_bounding_area_differ() {
    let circle = circle_shape2(meters(2.0)).expect("circle");
    let shape = PackingShape2::Circle(circle.clone());

    let area = shape.area().value;
    let bounding = shape.bounding_area().value;

    assert!((area - PI * 4.0).abs() < 1e-9, "area={area}");
    assert!((bounding - 16.0).abs() < 1e-9, "bounding={bounding}");
    assert!(area < bounding, "a circle's actual area is below its bounding area");
}

/// 圆形包围尺寸取直径 / A circle's bounding size is its diameter
#[test]
fn circle_bounding_size_is_diameter() {
    let circle = circle_shape2(meters(2.0)).expect("circle");
    let shape = PackingShape2::Circle(circle);

    assert!((shape.bounding_width().value - 4.0).abs() < 1e-9);
    assert!((shape.bounding_height().value - 4.0).abs() < 1e-9);
}

/// 非正尺寸被拒绝 / Non-positive sizes are rejected
#[test]
fn non_positive_shapes_are_rejected() {
    assert!(rectangle_shape2(meters(0.0), meters(4.0)).is_err());
    assert!(rectangle_shape2(meters(3.0), meters(-1.0)).is_err());
    assert!(circle_shape2(meters(0.0)).is_err());
    assert!(sheet2_shape(meters(0.0), meters(6.0)).is_err());
}

// ============================================================================
// 内部重叠语义 / Interior overlap semantics
// ============================================================================

/// 相切不算重叠 / Tangency is not overlap
///
/// 这是网格排样的前提：若把相切判为重叠，任何规则网格都无法产出。
/// This is the precondition for grid layouts: if tangency counted as overlap, no regular grid could exist.
#[test]
fn touching_rectangles_do_not_overlap() {
    let left = Box2Geometry::of_rectangle(meters(0.0), meters(0.0), meters(2.0), meters(2.0));
    let right = Box2Geometry::of_rectangle(meters(2.0), meters(0.0), meters(2.0), meters(2.0));
    let above = Box2Geometry::of_rectangle(meters(0.0), meters(2.0), meters(2.0), meters(2.0));

    assert!(!left.overlapped(&right), "edge-sharing rectangles must not overlap");
    assert!(!left.overlapped(&above), "edge-sharing rectangles must not overlap");
}

/// 内部相交才算重叠 / Only interior intersection counts as overlap
#[test]
fn intersecting_rectangles_overlap() {
    let left = Box2Geometry::of_rectangle(meters(0.0), meters(0.0), meters(2.0), meters(2.0));
    let shifted = Box2Geometry::of_rectangle(meters(1.5), meters(0.5), meters(2.0), meters(2.0));

    assert!(left.overlapped(&shifted));
    assert!(shifted.overlapped(&left), "overlap must be symmetric");
}

/// 相切圆不算重叠 / Tangent circles do not overlap
#[test]
fn tangent_circles_do_not_overlap() {
    let circle = || {
        PackingShape2::Circle(circle_shape2(meters(1.0)).expect("circle"))
    };
    let left = Box2Geometry::new(meters(0.0), meters(0.0), circle());
    // 圆心距离恰好等于直径 2，两个外接正方形贴边。
    // The centre distance equals the diameter 2, so the enclosing squares share an edge.
    let right = Box2Geometry::new(meters(2.0), meters(0.0), circle());

    assert!(!left.overlapped(&right));
}

/// 圆与矩形边界相切不算重叠 / A circle tangent to a rectangle boundary does not overlap
#[test]
fn circle_tangent_to_rectangle_does_not_overlap() {
    let rectangle = Box2Geometry::of_rectangle(meters(0.0), meters(0.0), meters(2.0), meters(2.0));
    let circle = Box2Geometry::new(
        meters(2.0),
        meters(0.0),
        PackingShape2::Circle(circle_shape2(meters(1.0)).expect("circle")),
    );

    assert!(!rectangle.overlapped(&circle));
}

// ============================================================================
// 刀缝 / Kerf
// ============================================================================

/// 矩形按刀缝向四周膨胀 / A rectangle inflates by the kerf on every side
#[test]
fn rectangle_inflates_by_kerf() {
    let shape = PackingShape2::Rectangle(rectangle_shape2(meters(2.0), meters(3.0)).expect("rect"));
    let inflated = inflated_by_shape(&shape, &meters(0.5));

    assert!((inflated.bounding_width().value - 2.5).abs() < 1e-9);
    assert!((inflated.bounding_height().value - 3.5).abs() < 1e-9);
}

/// 圆形按刀缝膨胀直径 / A circle inflates its diameter by the kerf
#[test]
fn circle_inflates_by_kerf() {
    let shape = PackingShape2::Circle(circle_shape2(meters(1.0)).expect("circle"));
    let inflated = inflated_by_shape(&shape, &meters(0.4));

    // 直径 2 + 0.4 = 2.4，半径 1.2。
    // Diameter 2 + 0.4 = 2.4, radius 1.2.
    assert!((inflated.bounding_width().value - 2.4).abs() < 1e-9);
}

/// 节距等于包围尺寸加间距 / The pitch is the bounding size plus the spacing
#[test]
fn pitch_adds_spacing_to_bounding_size() {
    let shape = PackingShape2::Rectangle(rectangle_shape2(meters(2.0), meters(3.0)).expect("rect"));
    let (pitch_width, pitch_height) = pitch_of(&shape, &meters(0.5));

    assert!((pitch_width.value - 2.5).abs() < 1e-9);
    assert!((pitch_height.value - 3.5).abs() < 1e-9);
}

/// 间距为零时节距退化为包围尺寸 / A zero spacing leaves the pitch equal to the bounding size
#[test]
fn zero_spacing_keeps_the_bounding_size() {
    let shape = PackingShape2::Rectangle(rectangle_shape2(meters(2.0), meters(3.0)).expect("rect"));
    let zero = zero_spacing_of(&meters(1.0));
    let (pitch_width, pitch_height) = pitch_of(&shape, &zero);

    assert!((pitch_width.value - 2.0).abs() < 1e-9);
    assert!((pitch_height.value - 3.0).abs() < 1e-9);
}

/// 双向膨胀才判定间隙不足 / Only double inflation proves an insufficient gap
///
/// 单向膨胀会把贴边的合法排布误判为间隙不足。
/// Inflating one side only would misjudge a legal edge-sharing layout as an insufficient gap.
#[test]
fn double_inflation_detects_insufficient_gap() {
    let spacing = meters(0.5);
    // 原间隙 0.3 < 刀缝 0.5：任一侧膨胀后仍相交。
    // The original gap 0.3 is below the kerf 0.5, so inflating either side still intersects.
    let left = PackingShape2::Rectangle(rectangle_shape2(meters(2.0), meters(2.0)).expect("rect"));
    let right = PackingShape2::Rectangle(rectangle_shape2(meters(2.0), meters(2.0)).expect("rect"));
    let left_inflated =
        Box2Geometry::new(meters(0.0), meters(0.0), inflated_by_shape(&left, &spacing));
    let right_inflated =
        Box2Geometry::new(meters(2.3), meters(0.0), inflated_by_shape(&right, &spacing));
    let right_raw = Box2Geometry::new(meters(2.3), meters(0.0), right.clone());

    assert!(
        left_inflated.overlapped(&right_inflated),
        "double inflation must detect the narrow gap"
    );
    // 只膨胀一侧时，真实的 0.3 间隙被判为足够，正是需要避免的漏判。
    // With only one side inflated the real 0.3 gap looks sufficient, which is exactly the miss to avoid.
    assert!(
        !Box2Geometry::new(meters(0.0), meters(0.0), left.clone()).overlapped(&right_raw),
        "the raw boxes do not overlap"
    );
}

// ============================================================================
// 板材与包含 / Sheet and containment
// ============================================================================

/// 板材面积与包围盒一致 / A sheet area matches its bounding box
#[test]
fn sheet_area_matches_origin_box() {
    let sheet = sheet2_shape(meters(10.0), meters(6.0)).expect("sheet");
    let origin = sheet.origin_box();

    assert!((sheet.area().value - 60.0).abs() < 1e-9);
    assert!((origin.width().value - 10.0).abs() < 1e-9);
    assert!((origin.height().value - 6.0).abs() < 1e-9);
}

/// 完全在板材内的盒被接受，越界与贴边分别判定 / Boxes inside a sheet are accepted; out-of-bounds and edge cases differ
#[test]
fn sheet_containment_is_closed_at_the_boundary() {
    let sheet = origin_box(meters(10.0), meters(6.0));
    let inside = Box2Geometry::of_rectangle(meters(1.0), meters(1.0), meters(2.0), meters(2.0));
    let touching = Box2Geometry::of_rectangle(meters(8.0), meters(1.0), meters(2.0), meters(2.0));
    let outside = Box2Geometry::of_rectangle(meters(9.0), meters(1.0), meters(2.0), meters(2.0));

    assert!(inside_sheet(&inside, &sheet));
    // 板材边界不是刀缝，贴边属于在内。
    // A sheet boundary is not a kerf, so touching the boundary counts as inside.
    assert!(inside_sheet(&touching, &sheet));
    assert!(!inside_sheet(&outside, &sheet));
}

/// 点包含使用闭区间 / Point containment uses closed intervals
#[test]
fn box_contains_boundary_points() {
    let b = Box2Geometry::of_rectangle(meters(0.0), meters(0.0), meters(2.0), meters(2.0));

    assert!(b.contains(&point2_of(meters(1.0), meters(1.0))));
    assert!(b.contains(&point2_of(meters(0.0), meters(0.0))));
    assert!(b.contains(&point2_of(meters(2.0), meters(2.0))));
    assert!(!b.contains(&point2_of(meters(2.5), meters(1.0))));
}

// ============================================================================
// 候选与旋转 / Candidates and rotation
// ============================================================================

/// 可旋转矩形给出两个候选 / A rotatable rectangle yields two candidates
#[test]
fn rotatable_rectangle_yields_two_candidates() {
    let shape = PackingShape2::Rectangle(rectangle_shape2(meters(2.0), meters(3.0)).expect("rect"));
    let candidates = candidate_shapes(&shape, true);

    assert_eq!(candidates.len(), 2);
    assert!(!candidates[0].rotated);
    assert!(candidates[1].rotated);
    assert!((candidates[1].shape.bounding_width().value - 3.0).abs() < 1e-9);
}

/// 正方形与圆形只给一个候选 / Squares and circles yield a single candidate
#[test]
fn square_and_circle_yield_one_candidate() {
    let square = PackingShape2::Rectangle(rectangle_shape2(meters(2.0), meters(2.0)).expect("rect"));
    assert_eq!(candidate_shapes(&square, true).len(), 1);

    let circle = PackingShape2::Circle(circle_shape2(meters(1.0)).expect("circle"));
    assert_eq!(candidate_shapes(&circle, true).len(), 1);
}

/// 禁止旋转时只给直立候选 / Disallowing rotation yields only the upright candidate
#[test]
fn disallowed_rotation_yields_one_candidate() {
    let shape = PackingShape2::Rectangle(rectangle_shape2(meters(2.0), meters(3.0)).expect("rect"));
    assert_eq!(candidate_shapes(&shape, false).len(), 1);
}

// ============================================================================
// 量值运算 / Quantity operations
// ============================================================================

/// 量纲不兼容的加减以错误暴露 / Incompatible dimensions surface as errors
#[test]
fn dimension_mismatch_is_an_error_not_a_panic() {
    let length = meters(2.0);
    let area = Quantity::new(4.0, SquareMeter::INSTANT.clone());
    let length_runtime = length.to_runtime();

    assert!(quantity_plus_safe(&length_runtime, &area).is_err());
    assert!(quantity_minus_safe(&length_runtime, &area).is_err());
    assert!(quantity_ord_or_null(&length_runtime, &area).is_none());
    assert!(!quantity_greater(&length_runtime, &area));
}

/// 同量纲加减与比较正常 / Same-dimension arithmetic and comparison work
#[test]
fn same_dimension_arithmetic_works() {
    let lhs = meters(2.0).to_runtime();
    let rhs = meters(3.0).to_runtime();

    let sum = quantity_plus_safe(&lhs, &rhs).expect("sum");
    assert!((sum.value - 5.0).abs() < 1e-9);
    let difference = quantity_minus_safe(&rhs, &lhs).expect("difference");
    assert!((difference.value - 1.0).abs() < 1e-9);
    assert!(quantity_less(&lhs, &rhs));
    assert!(quantity_greater_or_equal(&rhs, &lhs));
}

/// 计数与整数缩放 / Fitting counts and integer scaling
#[test]
fn count_fitting_and_scaling() {
    let cell = meters(2.0).to_runtime();
    let container = meters(9.0).to_runtime();

    assert_eq!(count_fitting(&cell, &container), 4);
    assert_eq!(count_fitting(&meters(0.0).to_runtime(), &container), 0);
    assert_eq!(scaled_quantity(&cell, 3).value, 6.0);
    assert_eq!(scaled_quantity(&cell, 0).value, 0.0);
}

/// 排序按量值大小 / Sorting follows the quantity ordering
#[test]
fn sorting_follows_quantities() {
    let values = vec![meters(3.0), meters(1.0), meters(2.0)];
    let ascending = sorted_by_quantity(values.clone(), |value| value.clone());
    assert_eq!(
        ascending.iter().map(|value| value.value).collect::<Vec<_>>(),
        vec![1.0, 2.0, 3.0]
    );
    let descending = sorted_by_descending_quantity(values, |value| value.clone());
    assert_eq!(
        descending.iter().map(|value| value.value).collect::<Vec<_>>(),
        vec![3.0, 2.0, 1.0]
    );
}
