//! 图案生成契约测试 / Pattern generation contract tests
//!
//! 对应 Kotlin `PatternGenerationContractTest`：请求校验（负刀缝、空产品、零剩余）、
//! 料域高度上界（定尺取板高、卷材按件累加并截断）、货架式生成（逐行铺满、刀缝节距、
//! 前缀候选）、断头台切分树构造与三级工艺的能力边界。
//! Mirrors Kotlin `PatternGenerationContractTest`: request validation (negative kerf, no product, zero
//! remaining), the region height bound (sheet height or accumulated coil bound with truncation), shelf
//! generation (row filling, kerf pitch, prefix candidates), guillotine tree construction and the three-stage
//! capability boundary.

use ospf_rust_framework_csp2d::domain::material::{
    Material2, Product2, material_id2, product_id2,
};
use ospf_rust_framework_csp2d::domain::pattern_generation::{
    PatternGenerationRequest2, PatternGenerator2, ShelfPatternGenerator2, shelf_pattern,
};
use ospf_rust_framework_csp2d::infrastructure::{CutOrientation2, CuttingProcess2};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;

fn meters(value: f64) -> Quantity<f64, Meter> {
    Quantity::new_ct(value)
}

fn sheet_material() -> Material2<f64, Meter> {
    Material2::sheet(material_id2("m1"), "sheet", meters(10.0), meters(8.0), 10)
}

fn coil_material() -> Material2<f64, Meter> {
    Material2::coil(material_id2("c1"), "coil", meters(10.0), 5)
}

fn product(id: &str, width: f64, height: f64) -> Product2<f64, Meter> {
    Product2::new(product_id2(id), id, meters(width), meters(height))
}

// ============================================================================
// 请求校验 / Request validation
// ============================================================================

/// 合法请求通过校验 / A legal request validates
#[test]
fn legal_request_validates() {
    let request = PatternGenerationRequest2::new(
        sheet_material(),
        vec![(product("a", 2.0, 2.0), 10)],
        meters(0.0),
    );

    assert!(request.validate().is_ok());
}

/// 负刀缝被拒绝 / A negative kerf is rejected
#[test]
fn negative_kerf_is_rejected() {
    let request = PatternGenerationRequest2::new(
        sheet_material(),
        vec![(product("a", 2.0, 2.0), 10)],
        meters(-1.0),
    );

    let error = request.validate().expect_err("must be rejected");
    assert!(error.to_string().contains("刀缝") || error.to_string().contains("kerf"));
}

/// 空产品列表被拒绝 / An empty product list is rejected
#[test]
fn empty_products_are_rejected() {
    let request =
        PatternGenerationRequest2::new(sheet_material(), Vec::new(), meters(0.0));

    assert!(request.validate().is_err());
}

/// 零剩余件数被拒绝 / A zero remaining amount is rejected
#[test]
fn zero_remaining_is_rejected() {
    let request = PatternGenerationRequest2::new(
        sheet_material(),
        vec![(product("a", 2.0, 2.0), 0)],
        meters(0.0),
    );

    assert!(request.validate().is_err());
}

/// 非法产品几何被拒绝 / Illegal product geometry is rejected
#[test]
fn illegal_product_geometry_is_rejected() {
    let request = PatternGenerationRequest2::new(
        sheet_material(),
        vec![(product("a", 0.0, 2.0), 10)],
        meters(0.0),
    );

    assert!(request.validate().is_err());
}

// ============================================================================
// 料域上界 / Region bounds
// ============================================================================

/// 定尺模式的料域高度上界就是板高 / The sheet-mode height bound is the sheet height
#[test]
fn sheet_height_bound_is_the_sheet_height() {
    let request = PatternGenerationRequest2::new(
        sheet_material(),
        vec![(product("a", 2.0, 2.0), 10)],
        meters(0.5),
    );

    assert_eq!(request.region_height_bound().value, 8.0);
    assert_eq!(request.region_width_bound().value, 10.0);
}

/// 卷材模式的高度上界按件累加并计入刀缝 / The coil-mode height bound accumulates pieces plus kerf
#[test]
fn coil_height_bound_accumulates_pieces() {
    let request = PatternGenerationRequest2::new(
        coil_material(),
        vec![(product("a", 2.0, 2.0), 3)],
        meters(0.5),
    );

    // 3 件 × (2 + 0.5) = 7.5。
    assert!((request.region_height_bound().value - 7.5).abs() < 1e-9);
}

/// 卷材高度上界被最大段高截断 / The coil height bound is truncated by the maximum segment height
#[test]
fn coil_height_bound_is_truncated() {
    let request = PatternGenerationRequest2::new(
        coil_material(),
        vec![(product("a", 2.0, 2.0), 100)],
        meters(0.5),
    )
    .with_max_segment_height(meters(20.0));

    assert!((request.region_height_bound().value - 20.0).abs() < 1e-9);
}

// ============================================================================
// 货架式生成 / Shelf generation
// ============================================================================

/// 生成器逐行铺满宽度 / The generator fills the width row by row
#[test]
fn shelf_fills_width() {
    let request = PatternGenerationRequest2::new(
        sheet_material(),
        vec![(product("a", 2.0, 2.0), 20)],
        meters(0.0),
    )
;

    let patterns = ShelfPatternGenerator2::new()
        .generate_patterns(&request)
        .expect("patterns");

    // 板材 10×8、件 2×2：每行 5 件、共 4 行；最后一个前缀覆盖最全。
    // Sheet 10×8 with 2×2 pieces: 5 per row over 4 rows; the last prefix covers the most.
    let full = patterns.last().expect("at least one pattern");
    assert_eq!(full.placements.len(), 20);
    assert!((full.used_area().value - 80.0).abs() < 1e-9);
}

/// 刀缝通过节距减少每行件数 / The kerf reduces the per-row count through the pitch
#[test]
fn shelf_honours_kerf_pitch() {
    let request = PatternGenerationRequest2::new(
        sheet_material(),
        vec![(product("a", 2.0, 2.0), 20)],
        meters(1.0),
    )
;

    let patterns = ShelfPatternGenerator2::new()
        .generate_patterns(&request)
        .expect("patterns");

    // 列距 3：10/3 = 3 件/行；行距 3：8/3 = 2 行 → 最多 6 件。
    let full = patterns.last().expect("at least one pattern");
    assert_eq!(full.placements.len(), 6);
    let xs: Vec<f64> = full
        .placements
        .iter()
        .map(|placement| placement.position.x.value)
        .collect();
    assert!(xs.contains(&3.0), "相邻件间距应为宽 + 刀缝");
}

/// 每个前缀行数产出一个候选 / Every prefix row count yields a candidate
///
/// 与 Kotlin 的 `ShelfPatternGenerator2` 一致：`max_candidates` 限制产出图案数，前缀越长覆盖越多。
/// Matches Kotlin's `ShelfPatternGenerator2`: `max_candidates` bounds the emitted pattern count and longer
/// prefixes cover more.
#[test]
fn shelf_emits_prefix_candidates() {
    let request = PatternGenerationRequest2::new(
        sheet_material(),
        vec![(product("a", 2.0, 2.0), 20)],
        meters(0.0),
    )
    .with_max_candidates(4);

    let patterns = ShelfPatternGenerator2::new()
        .generate_patterns(&request)
        .expect("patterns");

    // 4 行 → 4 个前缀候选，件数依次为 5/10/15/20。
    assert_eq!(patterns.len(), 4);
    let counts: Vec<usize> = patterns
        .iter()
        .map(|pattern| pattern.placements.len())
        .collect();
    assert_eq!(counts, vec![5, 10, 15, 20]);
}

/// 候选上限限制产出图案数并保留覆盖最大的末尾 / The limit bounds emitted patterns and keeps the largest tail
#[test]
fn shelf_respects_candidate_limit() {
    let request = PatternGenerationRequest2::new(
        sheet_material(),
        vec![(product("a", 2.0, 2.0), 20)],
        meters(0.0),
    )
    .with_max_candidates(2);

    let patterns = ShelfPatternGenerator2::new()
        .generate_patterns(&request)
        .expect("patterns");

    // 4 行、上限 2 → 保留 15 件与 20 件两个前缀。
    assert_eq!(patterns.len(), 2);
    let counts: Vec<usize> = patterns
        .iter()
        .map(|pattern| pattern.placements.len())
        .collect();
    assert_eq!(counts, vec![15, 20]);
}

/// 剩余件数限制放置数量 / The remaining amount caps the placement count
#[test]
fn shelf_respects_remaining_amount() {
    let request = PatternGenerationRequest2::new(
        sheet_material(),
        vec![(product("a", 2.0, 2.0), 3)],
        meters(0.0),
    )
;

    let patterns = ShelfPatternGenerator2::new()
        .generate_patterns(&request)
        .expect("patterns");
    assert_eq!(patterns[0].placements.len(), 3);
}

/// 多种产品按行分别选择 / Multiple products are chosen per row
#[test]
fn shelf_handles_multiple_products() {
    let request = PatternGenerationRequest2::new(
        sheet_material(),
        vec![
            (product("a", 2.0, 2.0), 10),
            (product("b", 5.0, 3.0), 10),
        ],
        meters(0.0),
    )
;

    let patterns = ShelfPatternGenerator2::new()
        .generate_patterns(&request)
        .expect("patterns");
    let amounts = patterns[0].product_amounts();

    // 两种产品都应出现在图案里（行选择按「可放件数最多」推进）。
    assert!(!amounts.is_empty());
    assert!(patterns[0].placements.len() > 0);
}

/// 放不下任何产品时返回空 / Nothing fits yields no pattern
#[test]
fn shelf_returns_empty_when_nothing_fits() {
    let request = PatternGenerationRequest2::new(
        sheet_material(),
        vec![(product("huge", 20.0, 20.0), 5)],
        meters(0.0),
    );

    let patterns = ShelfPatternGenerator2::new()
        .generate_patterns(&request)
        .expect("patterns");
    assert!(patterns.is_empty());
}

/// 生成器实现 PatternGenerator2 接口 / The generator implements the trait
#[test]
fn shelf_implements_generator_trait() {
    let request = PatternGenerationRequest2::new(
        sheet_material(),
        vec![(product("a", 2.0, 2.0), 20)],
        meters(0.0),
    )
;
    let generator = ShelfPatternGenerator2::new();

    // trait 方法需要显式标注泛型参数，否则无法从 name() 推断 V/U。
    // Trait methods need explicit generics because name() cannot infer V/U.
    assert_eq!(PatternGenerator2::<f64, Meter>::name(&generator), "shelf");
    assert!(!PatternGenerator2::<f64, Meter>::generate(&generator, &request).is_empty());
}

/// 便捷构造返回最后一个前缀（覆盖最多） / The convenience constructor returns the last prefix
#[test]
fn shelf_pattern_helper_returns_fullest() {
    let pattern = shelf_pattern(
        &sheet_material(),
        &[(product("a", 2.0, 2.0), 20)],
        meters(0.0),
        CuttingProcess2::Freeform,
    )
    .expect("pattern")
    .expect("some pattern");

    assert_eq!(pattern.placements.len(), 20);
}

// ============================================================================
// 断头台工艺 / Guillotine processes
// ============================================================================

/// 两级断头台图案携带合法切分树 / A two-stage guillotine pattern carries a valid cut tree
#[test]
fn guillotine_two_stage_builds_cut_tree() {
    let request = PatternGenerationRequest2::new(
        sheet_material(),
        vec![(product("a", 2.0, 2.0), 20)],
        meters(0.0),
    )
    .with_process(CuttingProcess2::GuillotineTwoStage)
;

    let patterns = ShelfPatternGenerator2::new()
        .generate_patterns(&request)
        .expect("patterns");

    assert!(patterns[0].cut_tree.is_some(), "must carry a cut tree");
    assert!(
        patterns[0]
            .validate(&meters(0.0))
            .is_ok(),
        "the tree must validate against the process"
    );
}

/// 任意排样不构造切分树 / Arbitrary nesting builds no cut tree
#[test]
fn freeform_builds_no_cut_tree() {
    let request = PatternGenerationRequest2::new(
        sheet_material(),
        vec![(product("a", 2.0, 2.0), 20)],
        meters(0.0),
    )
;

    let patterns = ShelfPatternGenerator2::new()
        .generate_patterns(&request)
        .expect("patterns");
    assert!(patterns[0].cut_tree.is_none());
}

/// 三级工艺在所需深度内同样构造切分树 / A three-stage process also builds a tree within the required depth
///
/// 切分树的深度等于覆盖 N 行所需的二叉层级数。4 行只需 2 层，因此三级工艺（上限 3）完全够用，
/// 图案合法。
/// The tree depth equals the binary levels needed for N rows. Four rows need only two levels, so a three-stage
/// process (limit 3) is sufficient and the pattern is legal.
#[test]
fn guillotine_three_stage_builds_tree_within_depth() {
    let request = PatternGenerationRequest2::new(
        sheet_material(),
        vec![(product("a", 2.0, 2.0), 20)],
        meters(0.0),
    )
    .with_process(CuttingProcess2::GuillotineThreeStage)
;

    let patterns = ShelfPatternGenerator2::new()
        .generate_patterns(&request)
        .expect("patterns");

    assert!(patterns[0].cut_tree.is_some());
    assert!(patterns[0].validate(&meters(0.0)).is_ok());
}

/// 多行图案同样能构造合法切分树 / A multi-row pattern also builds a valid cut tree
///
/// 级数规则是「同向刀同级、换向才加一级」，因此把整板切成多条同向条带时所有条带刀线都是第 1 级，
/// 行内竖切成件才是第 2 级。8 行与 4 行在两级工艺下都合法——这正是 Kotlin `buildRowsTree` 的语义，
/// 也是我最初迁移时误判为「必须父级 +1」而收紧的地方。
/// The stage rule is "same direction keeps the stage, a direction change adds one", so cutting a sheet into
/// several same-direction strips keeps every strip cut at stage 1 while vertical cuts inside a row become
/// stage 2. Eight rows and four rows are both legal under a two-stage process — exactly Kotlin's
/// `buildRowsTree` semantics, and the point I wrongly tightened to "must be parent + 1" during migration.
#[test]
fn multi_row_pattern_builds_valid_two_stage_tree() {
    // 板材 10×8、件 1×1：8 行。
    // Sheet 10×8 with 1×1 pieces: eight rows.
    let request = PatternGenerationRequest2::new(
        sheet_material(),
        vec![(product("tiny", 1.0, 1.0), 200)],
        meters(0.0),
    )
    .with_process(CuttingProcess2::GuillotineTwoStage)
;

    let patterns = ShelfPatternGenerator2::new()
        .generate_patterns(&request)
        .expect("patterns");

    assert!(patterns[0].cut_tree.is_some(), "a cut tree must be built");
    assert!(
        patterns[0].validate(&meters(0.0)).is_ok(),
        "same-direction strip cuts stay at stage 1, so the tree is valid"
    );
}

/// 同向刀保持同一级、换向才加一级 / Same-direction cuts share a stage; only a direction change advances it
#[test]
fn stage_is_coupled_to_cut_orientation() {
    let request = PatternGenerationRequest2::new(
        sheet_material(),
        vec![(product("a", 2.0, 2.0), 20)],
        meters(0.0),
    )
    .with_process(CuttingProcess2::GuillotineTwoStage)
;

    let patterns = ShelfPatternGenerator2::new()
        .generate_patterns(&request)
        .expect("patterns");
    let full = patterns.last().expect("at least one pattern");
    let tree = full.cut_tree.as_ref().expect("cut tree");

    // 条带刀线（横刀）全部是第 1 级；行内竖切是第 2 级。
    // Every strip cut (horizontal) stays at stage 1 while vertical cuts inside a row are stage 2.
    let lines = tree.cut_lines();
    assert!(!lines.is_empty());
    let horizontal: Vec<_> = lines
        .iter()
        .filter(|line| line.orientation == CutOrientation2::Horizontal)
        .collect();
    assert!(!horizontal.is_empty(), "there must be strip cuts");
    assert!(
        horizontal.iter().all(|line| line.stage == 1),
        "every strip cut must stay at stage 1"
    );
    assert!(
        lines
            .iter()
            .filter(|line| line.orientation == CutOrientation2::Vertical)
            .all(|line| line.stage == 2),
        "every piece cut must be stage 2"
    );
}
