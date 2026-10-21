//! 块装载与 BLA 契约测试 / Block loading and BLA contract tests
//!
//! 对应 Kotlin `BlockLoadingContractTest` 与 `BottomUpLeftJustifiedAlgorithmContractTest`：
//! 简单块规则网格、刀缝节距、复杂块异构拼接、DFS 与多行切分、旋转守卫、BLA 跳过圆形件与
//! 无法放置的物品、BLA 刀缝双向膨胀。
//! Mirrors Kotlin `BlockLoadingContractTest` and `BottomUpLeftJustifiedAlgorithmContractTest`: regular grids
//! from simple blocks, kerf pitch, heterogeneous complex blocks, DFS and multi-row splitting, the rotation
//! guard, BLA skipping circle and unplaceable items, and BLA's double-sided kerf inflation.

use ospf_rust_framework_bpp2d::domain::bla::{
    BlaConfig, BottomUpLeftJustifiedAlgorithm, compare_placement_position,
};
use ospf_rust_framework_bpp2d::domain::block_loading::model::{Space2, placement_inside_space};
use ospf_rust_framework_bpp2d::domain::block_loading::service::{
    ComplexBlockGenerator, DepthFirstSearchAlgorithm2D, MultiRowHeuristicAlgorithm2D,
    SimpleBlockConfig, SimpleBlockGenerator, require_rotation_allowed,
};
use ospf_rust_framework_bpp2d::domain::item::model::{ActualItem, ItemId};
use ospf_rust_framework_geometry2d::{
    PackingShape2, QuantityPoint2, Sheet2Shape, circle_shape2, rectangle_shape2,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;

fn meters(value: f64) -> Quantity<f64, Meter> {
    Quantity::new_ct(value)
}

fn space(width: f64, height: f64) -> Space2<f64, Meter> {
    Space2::new(meters(0.0), meters(0.0), meters(width), meters(height))
}

fn rect_item(id: &str, width: f64, height: f64, allow_rotate: bool) -> ActualItem<f64, Meter> {
    ActualItem::new(
        ItemId::new(id),
        id,
        PackingShape2::Rectangle(rectangle_shape2(meters(width), meters(height)).expect("rect")),
        allow_rotate,
        meters(1.0),
    )
}

fn circle_item(id: &str, radius: f64) -> ActualItem<f64, Meter> {
    ActualItem::new(
        ItemId::new(id),
        id,
        PackingShape2::Circle(circle_shape2(meters(radius)).expect("circle")),
        true,
        meters(1.0),
    )
}

// ============================================================================
// Space2 - 空间区域 / Space region
// ============================================================================

/// 空间区域按尺寸容纳判定 / A space region accepts fit by size
#[test]
fn space_enabled_by_size() {
    let region = space(10.0, 6.0);

    assert!(region.enabled_by_size(&meters(10.0), &meters(6.0)));
    assert!(region.enabled_by_size(&meters(4.0), &meters(3.0)));
    assert!(!region.enabled_by_size(&meters(11.0), &meters(6.0)));
    assert!(!region.enabled_by_size(&meters(10.0), &meters(7.0)));
}

/// 沿 X 与 Y 切分产生正确尺寸 / Splitting along X and Y yields the right sizes
#[test]
fn space_splits_produce_correct_remainders() {
    let region = space(10.0, 6.0);

    let (left, right) = region.split_x(&meters(4.0)).expect("split x");
    assert_eq!(left.width.value, 4.0);
    assert_eq!(right.width.value, 6.0);
    assert_eq!(right.x.value, 4.0);

    let (bottom, top) = region.split_y(&meters(2.0)).expect("split y");
    assert_eq!(bottom.height.value, 2.0);
    assert_eq!(top.height.value, 4.0);
    assert_eq!(top.y.value, 2.0);
}

/// 退化区域被识别 / Degenerate regions are detected
#[test]
fn space_degeneracy_is_detected() {
    assert!(space(0.0, 6.0).is_degenerate());
    assert!(space(10.0, -1.0).is_degenerate());
    assert!(!space(10.0, 6.0).is_degenerate());
}

// ============================================================================
// 简单块 / Simple block
// ============================================================================

/// 简单块按整行重复铺满区域 / A simple block fills the region with whole-row repetition
#[test]
fn simple_block_fills_regular_grid() {
    let generator = SimpleBlockGenerator::new();
    let item = rect_item("a", 2.0, 2.0, false);
    let blocks = generator
        .invoke(&item, &space(10.0, 6.0), &SimpleBlockConfig::default(), None)
        .expect("blocks");

    assert_eq!(blocks.len(), 1);
    // 10/2 = 5 列，6/2 = 3 行。
    assert_eq!(blocks[0].placements.len(), 15);
    assert_eq!(blocks[0].source, "simple-block");
    assert!((blocks[0].space.width.value - 10.0).abs() < 1e-9);
    assert!((blocks[0].space.height.value - 6.0).abs() < 1e-9);
}

/// 刀缝通过节距减少可放数量 / The kerf reduces the fitting count through the pitch
#[test]
fn simple_block_honours_kerf_pitch() {
    let generator = SimpleBlockGenerator::new();
    let item = rect_item("a", 2.0, 2.0, false);
    let blocks = generator
        .invoke(
            &item,
            &space(10.0, 6.0),
            &SimpleBlockConfig::default(),
            Some(&meters(1.0)),
        )
        .expect("blocks");

    // 节距 3：10/3 = 3 列，6/3 = 2 行。
    assert_eq!(blocks[0].placements.len(), 6);
    // 相邻件间距恰为 1。
    let xs: Vec<f64> = blocks[0]
        .placements
        .iter()
        .map(|placement| placement.x().value)
        .collect();
    assert!(xs.contains(&3.0));
}

/// 圆形件不参与规则块 / Circle items do not take part in regular blocks
#[test]
fn simple_block_skips_circle_items() {
    let generator = SimpleBlockGenerator::new();
    let item = circle_item("c", 1.0);
    let blocks = generator
        .invoke(&item, &space(10.0, 6.0), &SimpleBlockConfig::default(), None)
        .expect("blocks");

    assert!(blocks.is_empty());
}

/// 允许旋转时给出直立与旋转两种块 / Allowing rotation yields both upright and rotated blocks
#[test]
fn simple_block_rotates_when_allowed() {
    let generator = SimpleBlockGenerator::new();
    let item = rect_item("a", 3.0, 1.0, true);
    let blocks = generator
        .invoke(
            &item,
            &space(6.0, 3.0),
            &SimpleBlockConfig {
                with_rotation: true,
                with_remainder: false,
            },
            None,
        )
        .expect("blocks");

    assert_eq!(blocks.len(), 2);
    assert!(blocks.iter().any(|block| block.source == "simple-block"));
    assert!(blocks.iter().any(|block| block.source == "simple-block-rotated"));
}

/// 禁止旋转时只给出直立块 / Disallowing rotation yields the upright block only
#[test]
fn simple_block_respects_disallowed_rotation() {
    let generator = SimpleBlockGenerator::new();
    let item = rect_item("a", 3.0, 1.0, false);
    let blocks = generator
        .invoke(
            &item,
            &space(6.0, 3.0),
            &SimpleBlockConfig::default(),
            None,
        )
        .expect("blocks");

    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].source, "simple-block");
}

/// 块利用率与实际面积一致 / Block utilisation matches the occupied area
#[test]
fn simple_block_utilisation() {
    let generator = SimpleBlockGenerator::new();
    let item = rect_item("a", 2.0, 2.0, false);
    let blocks = generator
        .invoke(&item, &space(10.0, 6.0), &SimpleBlockConfig::default(), None)
        .expect("blocks");

    // 15 件 × 4 = 60，区域 60，利用率 1。
    assert!((blocks[0].area().value - 60.0).abs() < 1e-9);
    assert!((blocks[0].utilization() - 1.0).abs() < 1e-9);
    assert_eq!(blocks[0].amount(), 15);
    assert_eq!(blocks[0].item_amounts().get("a"), Some(&15));
}

// ============================================================================
// 复杂块 / Complex block
// ============================================================================

/// 复杂块按行拼接异构物品 / A complex block stitches heterogeneous items row by row
#[test]
fn complex_block_stitches_rows() {
    let generator = ComplexBlockGenerator::new();
    let items = vec![rect_item("a", 2.0, 1.0, false), rect_item("b", 5.0, 2.0, false)];
    let blocks = generator
        .invoke(&items, &space(10.0, 6.0), false, None)
        .expect("blocks");

    assert_eq!(blocks.len(), 1);
    // a 占一行 5 件，b 占一行 2 件。
    assert_eq!(blocks[0].item_amounts().get("a"), Some(&5));
    assert_eq!(blocks[0].item_amounts().get("b"), Some(&2));
    assert!(blocks[0].source.contains("complex-block"));
}

/// 复杂块跳过圆形件 / A complex block skips circle items
#[test]
fn complex_block_skips_circles() {
    let generator = ComplexBlockGenerator::new();
    let items = vec![circle_item("c", 1.0), rect_item("a", 2.0, 1.0, false)];
    let blocks = generator
        .invoke(&items, &space(10.0, 6.0), false, None)
        .expect("blocks");

    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].item_amounts().get("c"), None);
    assert_eq!(blocks[0].item_amounts().get("a"), Some(&5));
}

/// 空物品列表返回空结果 / An empty item list yields no block
#[test]
fn complex_block_without_items_returns_empty() {
    let generator = ComplexBlockGenerator::new();
    let blocks = generator
        .invoke(&[], &space(10.0, 6.0), false, None)
        .expect("blocks");
    assert!(blocks.is_empty());
}

// ============================================================================
// 空间切分 / Space splitting
// ============================================================================

/// DFS 放置并在剩余空间继续 / DFS places and continues in the remainder
#[test]
fn depth_first_search_places_items() {
    let algorithm = DepthFirstSearchAlgorithm2D::new();
    let items = vec![(rect_item("a", 2.0, 2.0, false), 4usize)];
    let result = algorithm
        .invoke(&space(10.0, 6.0), &items, 8, None)
        .expect("dfs");

    assert_eq!(result.source, "dfs");
    assert_eq!(result.placed_amounts.get("a"), Some(&4));
    assert_eq!(result.rest_amounts.get("a"), None);
    assert_eq!(result.placements.len(), 4);
}

/// DFS 尊重需求上限，不会超量放置 / DFS respects the demand cap and never over-places
#[test]
fn depth_first_search_respects_demand() {
    let algorithm = DepthFirstSearchAlgorithm2D::new();
    let items = vec![(rect_item("a", 2.0, 2.0, false), 2usize)];
    let result = algorithm
        .invoke(&space(10.0, 6.0), &items, 8, None)
        .expect("dfs");

    assert_eq!(result.placements.len(), 2);
    assert_eq!(result.placed_amounts.get("a"), Some(&2));
}

/// 多行启发式按行推进 / The multi-row heuristic advances row by row
#[test]
fn multi_row_places_rows() {
    let algorithm = MultiRowHeuristicAlgorithm2D::new();
    let items = vec![(rect_item("a", 2.0, 2.0, false), 6usize)];
    let result = algorithm
        .invoke(&space(10.0, 6.0), &items, 8, None)
        .expect("multi-row");

    assert_eq!(result.source, "multi-row");
    assert_eq!(result.placements.len(), 6);
    // 每行 5 件，第二行从 y=2 开始。
    assert!(result.placements.iter().any(|placement| placement.y().value == 2.0));
}

/// 圆形件在这两类搜索中被跳过 / Circle items are skipped in both searches
#[test]
fn space_split_skips_circles() {
    let algorithm = MultiRowHeuristicAlgorithm2D::new();
    let items = vec![(circle_item("c", 1.0), 3usize)];
    let result = algorithm
        .invoke(&space(10.0, 6.0), &items, 8, None)
        .expect("multi-row");

    assert!(result.placements.is_empty());
    assert_eq!(result.rest_amounts.get("c"), Some(&3));
}

// ============================================================================
// 旋转守卫 / Rotation guard
// ============================================================================

/// 未允许旋转的矩形被拒绝 / A rectangle that disallows rotation is rejected
#[test]
fn rotation_guard_rejects_disallowed_rectangles() {
    let item = rect_item("a", 2.0, 3.0, false);
    let error = require_rotation_allowed(&item).expect_err("must be rejected");
    assert!(error.to_string().contains('a'));
}

/// 允许旋转的矩形与圆形件通过守卫 / Allowed rectangles and circle items pass the guard
#[test]
fn rotation_guard_allows_rotatable_and_circles() {
    assert!(require_rotation_allowed(&rect_item("a", 2.0, 3.0, true)).is_ok());
    // 圆形件旋转不变，始终允许。
    assert!(require_rotation_allowed(&circle_item("c", 1.0)).is_ok());
}

// ============================================================================
// BLA - 底部向上左对齐 / Bottom-up left-justified
// ============================================================================

/// BLA 从原点开始放置 / BLA starts placing at the origin
#[test]
fn bla_places_at_origin() {
    let algorithm = BottomUpLeftJustifiedAlgorithm::new(
        Sheet2Shape::new(meters(10.0), meters(6.0)),
        BlaConfig::default(),
        None,
    );
    let result = algorithm
        .invoke(&[(rect_item("a", 2.0, 2.0, false), 1)])
        .expect("bla");

    assert_eq!(result.placements.len(), 1);
    assert_eq!(result.placements[0].x().value, 0.0);
    assert_eq!(result.placements[0].y().value, 0.0);
    assert!(result.fully_placed());
}

/// BLA 跳过圆形件并记录 / BLA skips circle items and records them
#[test]
fn bla_skips_circle_items() {
    let algorithm = BottomUpLeftJustifiedAlgorithm::new(
        Sheet2Shape::new(meters(10.0), meters(6.0)),
        BlaConfig::default(),
        None,
    );
    let result = algorithm
        .invoke(&[(circle_item("c", 1.0), 2), (rect_item("a", 2.0, 2.0, false), 1)])
        .expect("bla");

    assert_eq!(result.skipped_circle_items.len(), 1);
    assert_eq!(result.placements.len(), 1);
    assert!(!result.fully_placed());
}

/// 放不下的物品记入未放置列表 / Items that do not fit are recorded as unplaced
#[test]
fn bla_records_unfitted_items() {
    let algorithm = BottomUpLeftJustifiedAlgorithm::new(
        Sheet2Shape::new(meters(4.0), meters(4.0)),
        BlaConfig::default(),
        None,
    );
    // 每件 3×3，板材 4×4 只能放一件。
    let result = algorithm
        .invoke(&[(rect_item("a", 3.0, 3.0, false), 2)])
        .expect("bla");

    assert_eq!(result.placements.len(), 1);
    assert_eq!(result.skipped_unfitted_items.len(), 1);
    assert!(!result.fully_placed());
}

/// BLA 放置不越界且互不重叠 / BLA placements stay inside the sheet and never overlap
#[test]
fn bla_placements_are_inside_and_disjoint() {
    let algorithm = BottomUpLeftJustifiedAlgorithm::new(
        Sheet2Shape::new(meters(10.0), meters(10.0)),
        BlaConfig::default(),
        None,
    );
    let result = algorithm
        .invoke(&[(rect_item("a", 2.0, 2.0, false), 6)])
        .expect("bla");

    let region = space(10.0, 10.0);
    for placement in &result.placements {
        assert!(placement_inside_space(placement, &region));
    }
    for (index, left) in result.placements.iter().enumerate() {
        for right in result.placements.iter().skip(index + 1) {
            let left_box = ospf_rust_framework_geometry2d::Box2Geometry::new(
                left.x(),
                left.y(),
                left.shape.clone(),
            );
            let right_box = ospf_rust_framework_geometry2d::Box2Geometry::new(
                right.x(),
                right.y(),
                right.shape.clone(),
            );
            assert!(!left_box.overlapped(&right_box), "BLA must not overlap pieces");
        }
    }
}

/// BLA 刀缝拒绝过近的放置 / BLA's kerf rejects placements that are too close
#[test]
fn bla_kerf_rejects_narrow_gaps() {
    // 无刀缝时 2×2 可放两件；给 1.5 的刀缝后只能放一件。
    // Without a kerf two 2×2 pieces fit; with a 1.5 kerf only one fits.
    let without_kerf = BottomUpLeftJustifiedAlgorithm::new(
        Sheet2Shape::new(meters(4.2), meters(2.0)),
        BlaConfig::default(),
        None,
    )
    .invoke(&[(rect_item("a", 2.0, 2.0, false), 2)])
    .expect("bla");
    assert_eq!(without_kerf.placements.len(), 2);

    let with_kerf = BottomUpLeftJustifiedAlgorithm::new(
        Sheet2Shape::new(meters(4.2), meters(2.0)),
        BlaConfig::default(),
        Some(meters(1.5)),
    )
    .invoke(&[(rect_item("a", 2.0, 2.0, false), 2)])
    .expect("bla");
    assert_eq!(with_kerf.placements.len(), 1);
}

/// 落点比较先加权距离、再 y、最后 x / Position comparison uses weighted distance, then y, then x
#[test]
fn placement_position_ordering() {
    let origin = QuantityPoint2 {
        x: meters(0.0),
        y: meters(0.0),
    };
    let right = QuantityPoint2 {
        x: meters(2.0),
        y: meters(0.0),
    };
    let up = QuantityPoint2 {
        x: meters(0.0),
        y: meters(1.0),
    };

    // 加权距离 2 vs 1：上方优先。
    assert_eq!(
        compare_placement_position(&right, &up, true),
        Some(std::cmp::Ordering::Greater)
    );
    assert_eq!(
        compare_placement_position(&origin, &up, true),
        Some(std::cmp::Ordering::Less)
    );
    // 关闭加权距离后先比较 y：右侧 y=0 小于上方 y=1，因此右侧优先。
    assert_eq!(
        compare_placement_position(&right, &up, false),
        Some(std::cmp::Ordering::Less)
    );
}
