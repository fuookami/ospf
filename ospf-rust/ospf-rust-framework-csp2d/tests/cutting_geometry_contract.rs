//! 裁切几何契约测试 / Cutting geometry contract tests
//!
//! 对应 Kotlin `CuttingGeometryContractTest`：区域切分扣刀缝、退化拒绝、断头台切分树校验
//! （区域一致性、级数上限、子刀线层号）、叶子面积守恒与下刀顺序。
//! Mirrors Kotlin `CuttingGeometryContractTest`: kerf-aware region cutting, degenerate rejection, guillotine
//! tree validation (region consistency, stage limit, child cut stages), leaf area conservation and cut order.

use ospf_rust_framework_csp2d::infrastructure::{
    CutLine2, CutOrientation2, CutTree2, CuttingProcess2, Region2, region2_of,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;

fn meters(value: f64) -> Quantity<f64, Meter> {
    Quantity::new_ct(value)
}

fn region(width: f64, height: f64) -> Region2<f64, Meter> {
    region2_of(meters(0.0), meters(0.0), meters(width), meters(height))
}

// ============================================================================
// 区域切分 / Region cutting
// ============================================================================

/// 竖刀按刀缝扣减右块起点 / A vertical cut offsets the right piece by the kerf
#[test]
fn vertical_cut_removes_kerf() {
    let source = region(10.0, 6.0);
    let (left, right) = source
        .cut(CutOrientation2::Vertical, &meters(4.0), &meters(0.5))
        .expect("cut");

    assert_eq!(left.width.value, 4.0);
    assert_eq!(right.x.value, 4.5, "右块起点 = position + kerf");
    assert_eq!(right.width.value, 5.5);
    // 刀缝带被切除：两块宽度之和 + 刀缝 = 原宽。
    assert!((left.width.value + right.width.value + 0.5 - 10.0).abs() < 1e-9);
}

/// 横刀按刀缝扣减上块起点 / A horizontal cut offsets the upper piece by the kerf
#[test]
fn horizontal_cut_removes_kerf() {
    let source = region(10.0, 6.0);
    let (bottom, top) = source
        .cut(CutOrientation2::Horizontal, &meters(2.0), &meters(0.5))
        .expect("cut");

    assert_eq!(bottom.height.value, 2.0);
    assert_eq!(top.y.value, 2.5);
    assert_eq!(top.height.value, 3.5);
}

/// 零刀缝时退化为普通切分 / A zero kerf degrades to a plain split
#[test]
fn zero_kerf_cuts_without_gap() {
    let source = region(10.0, 6.0);
    let (left, right) = source
        .cut(CutOrientation2::Vertical, &meters(4.0), &meters(0.0))
        .expect("cut");

    assert_eq!(left.width.value, 4.0);
    assert_eq!(right.x.value, 4.0);
    assert_eq!(right.width.value, 6.0);
}

/// 非法刀位被拒绝 / An illegal cut position is rejected
#[test]
fn illegal_cut_position_is_rejected() {
    let source = region(10.0, 6.0);

    // 位置在左边界上，左块退化。
    assert!(
        source
            .cut(CutOrientation2::Vertical, &meters(0.0), &meters(0.0))
            .is_err()
    );
    // 位置在右边界上，右块退化。
    assert!(
        source
            .cut(CutOrientation2::Vertical, &meters(10.0), &meters(0.0))
            .is_err()
    );
    // 刀缝过宽，右块被挤没。
    assert!(
        source
            .cut(CutOrientation2::Vertical, &meters(9.8), &meters(0.5))
            .is_err()
    );
}

/// 退化区域被识别 / Degenerate regions are detected
#[test]
fn degenerate_region_is_detected() {
    assert!(region(0.0, 6.0).is_degenerate());
    assert!(region(10.0, -1.0).is_degenerate());
    assert!(!region(10.0, 6.0).is_degenerate());
}

/// 区域面积与包含关系 / Region area and containment
#[test]
fn region_area_and_containment() {
    let outer = region(10.0, 6.0);
    let inner = region2_of(meters(1.0), meters(1.0), meters(2.0), meters(2.0));

    assert!((outer.area().value - 60.0).abs() < 1e-9);
    assert!(outer.contains(&inner));
    assert!(!inner.contains(&outer));
}

// ============================================================================
// 断头台切分树 / Guillotine cut tree
// ============================================================================

/// 两级切分树合法 / A two-stage cut tree is valid
#[test]
fn two_stage_tree_is_valid() {
    let root = region(10.0, 6.0);
    // 第 1 级：竖刀切出左条。
    let (left, right) = root
        .cut(CutOrientation2::Vertical, &meters(4.0), &meters(0.0))
        .expect("cut");
    let left_tree = CutTree2::leaf(left, Some("p1".to_string()));
    // 第 2 级：横刀把右条切成两件。
    let (bottom, top) = right
        .cut(CutOrientation2::Horizontal, &meters(3.0), &meters(0.0))
        .expect("cut");
    let right_tree = CutTree2::cut(
        CutLine2 {
            orientation: CutOrientation2::Horizontal,
            position: meters(3.0),
            stage: 2,
        },
        right,
        meters(0.0),
        CutTree2::leaf(bottom, Some("p2".to_string())),
        CutTree2::leaf(top, Some("p3".to_string())),
    )
    .expect("cut node");

    let tree = CutTree2::cut(
        CutLine2 {
            orientation: CutOrientation2::Vertical,
            position: meters(4.0),
            stage: 1,
        },
        root,
        meters(0.0),
        left_tree,
        right_tree,
    )
    .expect("root");

    assert!(tree.validate(CuttingProcess2::GuillotineTwoStage).is_ok());
    assert_eq!(tree.leaf_count(), 3);
    assert_eq!(tree.depth(), 2);
    assert!((tree.leaf_area() - 60.0).abs() < 1e-9);
}

/// 子刀线层号不正确被拒绝 / A child cut with the wrong stage is rejected
#[test]
fn wrong_child_stage_is_rejected() {
    let root = region(10.0, 4.0);
    let (left, right) = root
        .cut(CutOrientation2::Vertical, &meters(5.0), &meters(0.0))
        .expect("cut");
    let (bottom, top) = right
        .cut(CutOrientation2::Horizontal, &meters(2.0), &meters(0.0))
        .expect("cut");
    // 子刀线写成第 1 级，应为第 2 级。
    let right_tree = CutTree2::cut(
        CutLine2 {
            orientation: CutOrientation2::Horizontal,
            position: meters(2.0),
            stage: 1,
        },
        right,
        meters(0.0),
        CutTree2::leaf(bottom, None),
        CutTree2::leaf(top, None),
    )
    .expect("cut node");
    let tree = CutTree2::cut(
        CutLine2 {
            orientation: CutOrientation2::Vertical,
            position: meters(5.0),
            stage: 1,
        },
        root,
        meters(0.0),
        CutTree2::leaf(left, None),
        right_tree,
    )
    .expect("root");

    let error = tree
        .validate(CuttingProcess2::GuillotineTwoStage)
        .expect_err("must be rejected");
    assert!(
        error.to_string().contains("层号") || error.to_string().contains("stage"),
        "{error}"
    );
}

/// 超出工艺级数上限被拒绝 / Exceeding the process stage limit is rejected
#[test]
fn stage_beyond_limit_is_rejected() {
    let root = region(10.0, 8.0);
    let (left, right) = root
        .cut(CutOrientation2::Vertical, &meters(5.0), &meters(0.0))
        .expect("cut");
    let (bottom, top) = right
        .cut(CutOrientation2::Horizontal, &meters(4.0), &meters(0.0))
        .expect("cut");
    // bottom 的 x 起点是 5.0（竖刀切出的右块），因此刀位必须落在这个区间内。
    // `bottom` starts at x = 5.0 (the right piece of the vertical cut), so the cut must fall inside it.
    let (low, high) = bottom
        .cut(CutOrientation2::Vertical, &meters(7.5), &meters(0.0))
        .expect("split");
    // 第 3 级刀线在两级工艺下非法。
    let bottom_tree = CutTree2::cut(
        CutLine2 {
            orientation: CutOrientation2::Vertical,
            position: meters(7.5),
            stage: 3,
        },
        bottom,
        meters(0.0),
        CutTree2::leaf(low, None),
        CutTree2::leaf(high, None),
    )
    .expect("cut node");
    let right_tree = CutTree2::cut(
        CutLine2 {
            orientation: CutOrientation2::Horizontal,
            position: meters(4.0),
            stage: 2,
        },
        right,
        meters(0.0),
        bottom_tree,
        CutTree2::leaf(top, None),
    )
    .expect("right");
    let tree = CutTree2::cut(
        CutLine2 {
            orientation: CutOrientation2::Vertical,
            position: meters(5.0),
            stage: 1,
        },
        root,
        meters(0.0),
        CutTree2::leaf(left, None),
        right_tree,
    )
    .expect("root");

    assert!(
        tree.validate(CuttingProcess2::GuillotineTwoStage).is_err(),
        "stage 3 must be rejected by a two-stage process"
    );
    assert!(tree.validate(CuttingProcess2::GuillotineThreeStage).is_ok());
}

/// 下刀顺序为深度优先 / The cut order is depth-first
#[test]
fn cut_lines_follow_depth_first_order() {
    let root = region(10.0, 6.0);
    let (left, right) = root
        .cut(CutOrientation2::Vertical, &meters(4.0), &meters(0.0))
        .expect("cut");
    let (bottom, top) = right
        .cut(CutOrientation2::Horizontal, &meters(3.0), &meters(0.0))
        .expect("cut");
    let right_tree = CutTree2::cut(
        CutLine2 {
            orientation: CutOrientation2::Horizontal,
            position: meters(3.0),
            stage: 2,
        },
        right,
        meters(0.0),
        CutTree2::leaf(bottom, None),
        CutTree2::leaf(top, None),
    )
    .expect("cut node");
    let tree = CutTree2::cut(
        CutLine2 {
            orientation: CutOrientation2::Vertical,
            position: meters(4.0),
            stage: 1,
        },
        root,
        meters(0.0),
        CutTree2::leaf(left, None),
        right_tree,
    )
    .expect("root");

    let lines = tree.cut_lines();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].stage, 1);
    assert_eq!(lines[0].orientation, CutOrientation2::Vertical);
    assert_eq!(lines[1].stage, 2);
    assert_eq!(lines[1].orientation, CutOrientation2::Horizontal);
}

/// 工艺能力标记 / Process capability flags
#[test]
fn process_capabilities() {
    assert_eq!(CuttingProcess2::Freeform.max_stage(), usize::MAX);
    assert!(!CuttingProcess2::Freeform.requires_cut_tree());
    assert!(CuttingProcess2::GuillotineTwoStage.requires_cut_tree());
    assert!(CuttingProcess2::GuillotineTwoStage.allows_stage(1));
    assert!(CuttingProcess2::GuillotineTwoStage.allows_stage(2));
    assert!(!CuttingProcess2::GuillotineTwoStage.allows_stage(3));
    assert!(!CuttingProcess2::GuillotineThreeStage.allows_stage(0));
    assert_eq!(CuttingProcess2::GuillotineThreeStage.stage_label(2), "切块 / block");
}

/// 叶子总面积等于根区域面积（无刀缝）/ Leaf area equals the root area when there is no kerf
#[test]
fn leaf_area_is_conserved() {
    let root = region(10.0, 6.0);
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
        CutTree2::leaf(left, None),
        CutTree2::leaf(right, None),
    )
    .expect("root");

    assert!((tree.leaf_area() - 60.0).abs() < 1e-9);
}
