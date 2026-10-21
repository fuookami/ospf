//! 副产物填充与三级断头台契约测试 / By-product filling and three-stage guillotine contract tests
//!
//! 覆盖副产物填充的三条约束（不与成品重叠、不互相重叠、落在料域内）与一条语义约束
//! （副产物不参与需求覆盖），以及三级断头台的「条 → 块 → 件」刀线级数。
//! Covers the three constraints of by-product filling (no overlap with finished parts, no overlap with each
//! other, inside the region) plus the semantic constraint that by-products never enter demand coverage, and the
//! "strips, blocks, pieces" stage structure of a three-stage guillotine.

use ospf_rust_framework_csp2d::domain::material::{
    Costar2, Material2, Pattern2, Product2, costar_id2, material_id2, pattern_id2, product_id2,
};
use ospf_rust_framework_csp2d::domain::pattern_generation::{
    CostarFiller, PatternGenerationRequest2, ShelfPatternGenerator2,
};
use ospf_rust_framework_csp2d::infrastructure::{
    CutOrientation2, CuttingProcess2, region2_of,
};
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

fn product(id: &str, width: f64, height: f64) -> Product2<f64, Meter> {
    Product2::new(product_id2(id), id, meters(width), meters(height))
}

fn costar(id: &str, width: f64, height: f64) -> Costar2<f64, Meter> {
    Costar2::new(costar_id2(id), id, meters(width), meters(height))
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

/// 构造一个只在左下角放一件的图案，其余是余料。
fn sparse_pattern(width: f64, height: f64, part: f64) -> Pattern2<f64, Meter> {
    Pattern2::new(
        pattern_id2("p1"),
        sheet_material(width, height),
        region2_of(meters(0.0), meters(0.0), meters(width), meters(height)),
        vec![placement(part, part, 0.0, 0.0, "a")],
        CuttingProcess2::Freeform,
    )
}

// ============================================================================
// 副产物填充 / By-product filling
// ============================================================================

/// 副产物被放进余料 / By-products are packed into the remnant
#[test]
fn costars_are_packed_into_remnant() {
    let pattern = sparse_pattern(10.0, 8.0, 2.0);
    let filled = CostarFiller::new().fill(
        &pattern,
        &[costar("c1", 2.0, 2.0), costar("c2", 2.0, 2.0)],
        &meters(0.0),
    );

    assert_eq!(filled.placed_count(), 2);
    assert!(filled.unplaced.is_empty());
}

/// 副产物不与成品件重叠 / A by-product never overlaps a finished part
#[test]
fn costars_do_not_overlap_parts() {
    let pattern = sparse_pattern(10.0, 8.0, 2.0);
    let filled = CostarFiller::new().fill(&pattern, &[costar("c1", 3.0, 3.0)], &meters(0.0));

    for placement in &filled.placements {
        for part in &pattern.placements {
            let overlap_x = placement.position.x.value < part.position.x.value + part.shape.bounding_width().value
                && placement.position.x.value + placement.shape.bounding_width().value > part.position.x.value;
            let overlap_y = placement.position.y.value < part.position.y.value + part.shape.bounding_height().value
                && placement.position.y.value + placement.shape.bounding_height().value > part.position.y.value;
            assert!(!(overlap_x && overlap_y), "副产物与成品件重叠");
        }
    }
}

/// 副产物之间互不重叠 / By-products never overlap each other
#[test]
fn costars_do_not_overlap_each_other() {
    let pattern = sparse_pattern(10.0, 8.0, 2.0);
    let filled = CostarFiller::new().fill(
        &pattern,
        &[
            costar("c1", 2.0, 2.0),
            costar("c2", 2.0, 2.0),
            costar("c3", 2.0, 2.0),
        ],
        &meters(0.0),
    );

    for (index, left) in filled.placements.iter().enumerate() {
        for right in filled.placements.iter().skip(index + 1) {
            let overlap_x = left.position.x.value < right.position.x.value + right.shape.bounding_width().value
                && left.position.x.value + left.shape.bounding_width().value > right.position.x.value;
            let overlap_y = left.position.y.value < right.position.y.value + right.shape.bounding_height().value
                && left.position.y.value + left.shape.bounding_height().value > right.position.y.value;
            assert!(!(overlap_x && overlap_y), "副产物互相重叠");
        }
    }
}

/// 副产物必须落在料域内 / A by-product must stay inside the region
#[test]
fn costars_stay_inside_region() {
    let pattern = sparse_pattern(10.0, 8.0, 2.0);
    let filled = CostarFiller::new().fill(
        &pattern,
        &[costar("big", 6.0, 6.0), costar("small", 2.0, 2.0)],
        &meters(0.0),
    );

    for placement in &filled.placements {
        assert!(placement.position.x.value >= 0.0);
        assert!(placement.position.y.value >= 0.0);
        assert!(placement.position.x.value + placement.shape.bounding_width().value <= 10.0 + 1e-9);
        assert!(placement.position.y.value + placement.shape.bounding_height().value <= 8.0 + 1e-9);
    }
}

/// 放不下的副产物被记录为未放置 / A by-product that does not fit is recorded as unplaced
#[test]
fn oversized_costar_is_unplaced() {
    let pattern = sparse_pattern(10.0, 8.0, 2.0);
    let filled = CostarFiller::new().fill(&pattern, &[costar("huge", 20.0, 20.0)], &meters(0.0));

    assert_eq!(filled.placed_count(), 0);
    assert_eq!(filled.unplaced.len(), 1);
}

/// 无副产物时返回空结果 / No by-product yields an empty result
#[test]
fn no_costars_yields_empty_result() {
    let pattern = sparse_pattern(10.0, 8.0, 2.0);
    let filled = CostarFiller::new().fill(&pattern, &[], &meters(0.0));

    assert_eq!(filled.placed_count(), 0);
    assert!(filled.unplaced.is_empty());
}

/// 副产物不改变成品覆盖量 / By-products never change finished coverage
///
/// 这是语义约束：副产物不是需求驱动的产出，不能参与需求覆盖。
/// This is the semantic constraint: by-products are not demand-driven output and must not enter demand
/// coverage.
#[test]
fn costars_do_not_change_finished_coverage() {
    let without = sparse_pattern(10.0, 8.0, 2.0);
    let filled = CostarFiller::new().fill(&without, &[costar("c1", 2.0, 2.0)], &meters(0.0));
    let with = without.clone().with_costars(filled.placements);

    assert_eq!(without.product_amounts(), with.product_amounts());
    // 但余料面积会因为副产物而减少。
    // The remnant area does shrink because of the by-products.
    assert!(with.waste_area().value < without.waste_area().value);
}

/// 生成器自动填充副产物 / The generator fills by-products automatically
#[test]
fn generator_fills_costars() {
    let request = PatternGenerationRequest2::new(
        sheet_material(10.0, 8.0),
        vec![(product("a", 2.0, 2.0), 4)],
        meters(0.0),
    )
    .with_costars(vec![costar("c1", 2.0, 2.0)]);

    let patterns = ShelfPatternGenerator2::new()
        .generate_patterns(&request)
        .expect("patterns");

    assert_eq!(patterns.len(), 1);
    assert!(
        !patterns[0].costar_placements.is_empty(),
        "the generator must fill by-products when they are supplied"
    );
    // 填充后图案仍应通过校验。
    assert!(patterns[0].validate(&meters(0.0)).is_ok());
}

// ============================================================================
// 三级断头台 / Three-stage guillotine
// ============================================================================

/// 三级工艺产出「条 → 块 → 件」的切分树 / A three-stage process emits a "strips, blocks, pieces" tree
#[test]
fn three_stage_emits_block_cuts() {
    // 板材 10×8、件 2×2 但每行只放 4 件（8 宽），右侧留余料；行高 2 小于料域高 8，
    // 因此需要条（横切）→ 块（横切）→ 件（竖切）三级。
    // Sheet 10×8 with 2×2 pieces, four per row (8 wide) leaving a right remnant; the row height 2 is below the
    // sheet height 8, so strips (horizontal), blocks (horizontal) and pieces (vertical) are all needed.
    let request = PatternGenerationRequest2::new(
        sheet_material(10.0, 8.0),
        vec![(product("a", 2.0, 2.0), 4)],
        meters(0.0),
    )
    .with_process(CuttingProcess2::GuillotineThreeStage);

    let patterns = ShelfPatternGenerator2::new()
        .generate_patterns(&request)
        .expect("patterns");
    let tree = patterns[0].cut_tree.as_ref().expect("cut tree");

    assert!(
        patterns[0].validate(&meters(0.0)).is_ok(),
        "a three-stage tree must validate against the process"
    );
    let stages: Vec<usize> = tree.cut_lines().iter().map(|line| line.stage).collect();
    assert!(!stages.is_empty());
    assert!(
        stages.iter().all(|stage| *stage <= CuttingProcess2::GuillotineThreeStage.max_stage()),
        "no cut may exceed the three-stage limit: {stages:?}"
    );
    assert!(
        tree.cut_lines()
            .iter()
            .any(|line| line.orientation == CutOrientation2::Vertical),
        "pieces must be cut vertically"
    );
}

/// 两级工艺的切分树不含第 3 级刀线 / A two-stage tree has no stage-3 cut
#[test]
fn two_stage_has_no_third_stage() {
    let request = PatternGenerationRequest2::new(
        sheet_material(10.0, 8.0),
        vec![(product("a", 2.0, 2.0), 20)],
        meters(0.0),
    )
    .with_process(CuttingProcess2::GuillotineTwoStage);

    let patterns = ShelfPatternGenerator2::new()
        .generate_patterns(&request)
        .expect("patterns");
    let tree = patterns[0].cut_tree.as_ref().expect("cut tree");

    assert!(
        tree.cut_lines().iter().all(|line| line.stage <= 2),
        "a two-stage tree must not contain stage-3 cuts"
    );
    assert!(patterns[0].validate(&meters(0.0)).is_ok());
}

/// 任意排样不产出切分树 / Arbitrary nesting emits no cut tree
#[test]
fn freeform_emits_no_tree() {
    let request = PatternGenerationRequest2::new(
        sheet_material(10.0, 8.0),
        vec![(product("a", 2.0, 2.0), 20)],
        meters(0.0),
    );

    let patterns = ShelfPatternGenerator2::new()
        .generate_patterns(&request)
        .expect("patterns");
    assert!(patterns[0].cut_tree.is_none());
}
