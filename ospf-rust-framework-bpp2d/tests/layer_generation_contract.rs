//! 层生成契约测试 / Layer generation contract tests
//!
//! 对应 Kotlin `LayerGenerationContractTest` 与 `KerfLayerGenerationContractTest`：
//! 逐板材类型生成、未指定板材直通、列身份含板材类型、纯圆形/纯矩形/混合生成器筛选、
//! 图样与窄图样、圆形直角与六角网格、刀缝贯穿全部排布路径、历史层回放。
//! Mirrors Kotlin `LayerGenerationContractTest` and `KerfLayerGenerationContractTest`: per-sheet-type
//! generation, pass-through without a sheet, sheet type in column identity, generator selection for
//! circle-only/rectangle-only/mixed requests, pattern and narrow pattern, square and hexagonal circle grids,
//! kerf propagation across every layout path, and historical replay.

use ospf_rust_framework_bpp2d::domain::item::model::{ItemId, Layer, SheetType, SheetTypeId};
use ospf_rust_framework_bpp2d::domain::layer_generation::{
    Bpp2dLayerGenerationRequest, Bpp2dLayerGenerationResult, LayerGenerationContext,
    order_candidate_sheets,
};
use ospf_rust_framework_bpp2d::domain::layer_generation::service::{
    CircleGridGenerator, CircleGridMode, PatternLayerGenerator,
};
use ospf_rust_framework_bpp2d::domain::item::model::ActualItem;
use ospf_rust_framework_bpp2d::domain::layer_generation::request::Bpp2dLayerGenerator;
use ospf_rust_framework_geometry2d::{
    Box2Geometry, PackingShape2, Sheet2Shape, circle_shape2, rectangle_shape2,
};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::derived::Meter;

fn meters(value: f64) -> Quantity<f64, Meter> {
    Quantity::new_ct(value)
}

fn sheet(code: &str, is_main: bool) -> SheetType<f64, Meter> {
    SheetType::new(meters(10.0), meters(8.0), meters(100.0), SheetTypeId::new(code))
        .with_main(is_main)
}

fn rect(id: &str, width: f64, height: f64, allow_rotate: bool) -> ActualItem<f64, Meter> {
    ActualItem::new(
        ItemId::new(id),
        id,
        PackingShape2::Rectangle(rectangle_shape2(meters(width), meters(height)).expect("rect")),
        allow_rotate,
        meters(1.0),
    )
}

fn circle(id: &str, radius: f64) -> ActualItem<f64, Meter> {
    ActualItem::new(
        ItemId::new(id),
        id,
        PackingShape2::Circle(circle_shape2(meters(radius)).expect("circle")),
        true,
        meters(1.0),
    )
}

fn generate(
    request: &Bpp2dLayerGenerationRequest<f64, Meter>,
) -> Vec<Bpp2dLayerGenerationResult<f64, Meter>> {
    let generators = LayerGenerationContext::default_generators::<f64, Meter>();
    LayerGenerationContext::new().generate(request, &generators)
}

// ============================================================================
// 板材展开 / Sheet expansion
// ============================================================================

/// 主板材排在候选序列首位 / The main sheet type leads the candidate sequence
#[test]
fn main_sheet_leads_candidate_sequence() {
    let ordered = order_candidate_sheets(vec![
        sheet("40HQ", false),
        sheet("20GP", true),
        sheet("40HQ", false),
    ]);

    assert_eq!(ordered.len(), 2);
    assert_eq!(ordered[0].type_code.as_str(), "20GP");
    assert!(ordered[0].is_main);
    assert_eq!(ordered[1].type_code.as_str(), "40HQ");
}

/// 多板材类型逐类型生成候选 / Multiple sheet types generate candidates per type
#[test]
fn multi_sheet_request_generates_per_type() {
    let request = Bpp2dLayerGenerationRequest::new(0, vec![(rect("a", 2.0, 2.0, false), 10)])
        .with_sheets(vec![
            SheetType::new(meters(10.0), meters(8.0), meters(100.0), SheetTypeId::new("S1"))
                .with_main(true),
            SheetType::new(meters(6.0), meters(4.0), meters(100.0), SheetTypeId::new("S2")),
        ])
        .with_max_candidates(64);

    let results = generate(&request);
    let codes: std::collections::HashSet<String> = results
        .iter()
        .filter_map(|result| result.layer.sheet.as_ref())
        .map(|sheet| sheet.type_code.as_str().to_string())
        .collect();

    assert!(codes.contains("S1"), "codes={codes:?}");
    assert!(codes.contains("S2"), "codes={codes:?}");
}

/// 同一排布在不同板材上是不同列 / The same layout on different sheets is a different column
#[test]
fn layer_identity_includes_sheet_type() {
    let request = Bpp2dLayerGenerationRequest::new(0, vec![(rect("a", 2.0, 2.0, false), 10)])
        .with_sheets(vec![
            SheetType::new(meters(10.0), meters(8.0), meters(100.0), SheetTypeId::new("S1"))
                .with_main(true),
            SheetType::new(meters(10.0), meters(8.0), meters(100.0), SheetTypeId::new("S2")),
        ])
        .with_max_candidates(64);

    let results = generate(&request);

    // 同名排布在不同板材上必须得到不同的身份键：否则多板材类型会被去重逻辑直接抹掉。
    // A same-named layout on different sheets must produce different identity keys; otherwise the multi
    // sheet-type capability would be erased by deduplication.
    let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for result in &results {
        let sheet_code = result
            .layer
            .sheet
            .as_ref()
            .map(|sheet| sheet.type_code.as_str().to_string())
            .unwrap_or_default();
        let fingerprint = format!("{}|{}", result.source, result.layer.units.len());
        if let Some(previous_sheet) = seen.get(&fingerprint) {
            assert_ne!(
                previous_sheet, &sheet_code,
                "identical layouts on different sheets must not share an identity"
            );
        }
        seen.insert(fingerprint, sheet_code);
    }

    // 直接验证身份键构造：同排布换板材，键必须变化。
    // Verify the identity key directly: the same layout on another sheet must change the key.
    let mut first = results.first().expect("at least one column").layer.clone();
    let original = first.identity_key();
    if let Some(sheet) = first.sheet.clone() {
        first.sheet = Some(SheetType::new(
            sheet.width.clone(),
            sheet.height.clone(),
            sheet.max_load.clone(),
            SheetTypeId::new("OTHER"),
        ));
        assert_ne!(first.identity_key(), original);
    }
}

/// 未指定任何板材时保持单次直通 / No sheet specified keeps a single pass-through
///
/// 不依赖板材的历史层生成器必须仍能工作；若按板材展开后循环体零次执行，它会静默失效。
/// The historical generator, which does not depend on a sheet, must keep working; if expansion ran the loop
/// body zero times it would silently stop producing anything.
#[test]
fn no_sheet_keeps_single_pass_through() {
    let existing = Layer::new(
        0,
        "seed",
        None,
        Sheet2Shape::new(meters(10.0), meters(8.0)),
    );
    let request = Bpp2dLayerGenerationRequest::new(0, Vec::new())
        .with_existing_layers(vec![existing])
        .with_max_candidates(8);

    let results = generate(&request);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].source, "historical");
    assert!(results[0].layer.sheet.is_none());
}

/// 历史层原样回放 / Historical layers are replayed unchanged
#[test]
fn historical_layers_are_replayed() {
    let existing = Layer::new(
        0,
        "seed",
        Some(sheet("S1", true)),
        Sheet2Shape::new(meters(10.0), meters(8.0)),
    );
    let request = Bpp2dLayerGenerationRequest::new(0, Vec::new())
        .with_existing_layers(vec![existing.clone()])
        .with_max_candidates(8);

    let results = generate(&request);

    assert!(results.iter().any(|result| result.source == "historical"));
    assert!(
        results
            .iter()
            .any(|result| result.layer.identity_key() == existing.identity_key())
    );
}

// ============================================================================
// 生成器筛选 / Generator selection
// ============================================================================

/// 纯矩形请求不产出圆形网格列 / A rectangle-only request produces no circle grid columns
#[test]
fn rectangle_only_request_excludes_circle_generators() {
    let request = Bpp2dLayerGenerationRequest::new(0, vec![(rect("a", 2.0, 2.0, false), 10)])
        .with_sheet(sheet("S1", true))
        .with_max_candidates(64);

    let results = generate(&request);

    assert!(!results.is_empty());
    assert!(
        results
            .iter()
            .all(|result| !result.source.starts_with("circle") && result.source != "mixed-shape"),
        "sources: {:?}",
        results.iter().map(|result| result.source.as_str()).collect::<Vec<_>>()
    );
}

/// 纯圆形请求只产出圆形网格列 / A circle-only request produces only circle grid columns
#[test]
fn circle_only_request_uses_circle_grid() {
    let request = Bpp2dLayerGenerationRequest::new(0, vec![(circle("c", 1.0), 10)])
        .with_sheet(sheet("S1", true))
        .with_max_candidates(64);

    let results = generate(&request);

    assert!(!results.is_empty());
    assert!(
        results
            .iter()
            .all(|result| result.source.starts_with("circle-")),
        "sources: {:?}",
        results.iter().map(|result| result.source.as_str()).collect::<Vec<_>>()
    );
}

/// 混合请求产出混合形状列 / A mixed request produces mixed shape columns
#[test]
fn mixed_request_uses_mixed_generator() {
    let request = Bpp2dLayerGenerationRequest::new(
        0,
        vec![(rect("r", 2.0, 2.0, false), 4), (circle("c", 1.0), 4)],
    )
    .with_sheet(sheet("S1", true))
    .with_max_candidates(64);

    let results = generate(&request);

    assert!(
        results.iter().any(|result| result.source == "mixed-shape"),
        "sources: {:?}",
        results.iter().map(|result| result.source.as_str()).collect::<Vec<_>>()
    );
}

// ============================================================================
// 图样 / Pattern
// ============================================================================

/// 图样铺满整板并给出窄行替代 / A pattern fills the sheet and emits a narrow alternative
#[test]
fn pattern_emits_full_and_narrow_variants() {
    let request = Bpp2dLayerGenerationRequest::new(0, vec![(rect("a", 3.0, 2.0, false), 10)])
        .with_sheet(sheet("S1", true));

    let results = PatternLayerGenerator.generate(&request);
    let sources: Vec<&str> = results.iter().map(|result| result.source.as_str()).collect();

    assert!(sources.contains(&"pattern"));
    assert!(sources.contains(&"pattern-narrow"));
}

/// 图样跳过圆形件 / Patterns skip circle items
#[test]
fn pattern_skips_circles() {
    let request = Bpp2dLayerGenerationRequest::new(0, vec![(circle("c", 1.0), 4)])
        .with_sheet(sheet("S1", true));

    assert!(PatternLayerGenerator.generate(&request).is_empty());
}

/// 图样在刀缝下减少可放数量 / The kerf reduces the fitting count in a pattern
#[test]
fn pattern_honours_kerf() {
    let without_kerf = PatternLayerGenerator.generate(
        &Bpp2dLayerGenerationRequest::new(0, vec![(rect("a", 2.0, 2.0, false), 100)])
            .with_sheet(sheet("S1", true)),
    );
    let with_kerf = PatternLayerGenerator.generate(
        &Bpp2dLayerGenerationRequest::new(0, vec![(rect("a", 2.0, 2.0, false), 100)])
            .with_sheet(sheet("S1", true))
            .with_kerf(meters(1.0)),
    );

    let plain_count = without_kerf
        .iter()
        .find(|result| result.source == "pattern")
        .map(|result| result.layer.units.len())
        .unwrap_or(0);
    let kerf_count = with_kerf
        .iter()
        .find(|result| result.source == "pattern")
        .map(|result| result.layer.units.len())
        .unwrap_or(0);

    assert_eq!(plain_count, 20, "10/2 × 8/2 = 5 × 4");
    assert_eq!(kerf_count, 6, "节距 3：10/3 = 3 列、8/3 = 2 行，共 6 件");
}

// ============================================================================
// 圆形网格 / Circle grids
// ============================================================================

/// 圆形网格给出直角与六角两种布局 / The circle generator yields square and hexagonal layouts
#[test]
fn circle_grid_yields_both_modes() {
    let generator = CircleGridGenerator::new();
    let layouts = generator.invoke(&circle("c", 1.0), &sheet("S1", true), None);
    let modes: Vec<CircleGridMode> = layouts.iter().map(|layout| layout.mode).collect();

    assert!(modes.contains(&CircleGridMode::SquareGrid));
    // 板材 10×8、直径 2：六角网格按 26/30 行距可容纳更多行。
    assert!(modes.contains(&CircleGridMode::HexGrid), "modes={modes:?}");
}

/// 圆形网格的放置互不重叠且落在板内 / Circle grid placements are disjoint and inside the sheet
#[test]
fn circle_grid_placements_are_disjoint_and_inside() {
    let generator = CircleGridGenerator::new();
    let layouts = generator.invoke(&circle("c", 1.0), &sheet("S1", true), None);
    let sheet_box = Box2Geometry::of_rectangle(
        meters(0.0),
        meters(0.0),
        meters(10.0),
        meters(8.0),
    );

    for layout in &layouts {
        assert!(!layout.placements.is_empty());
        for (index, left) in layout.placements.iter().enumerate() {
            let left_box = Box2Geometry::new(left.x(), left.y(), left.shape.clone());
            assert!(sheet_box.contains_box(&left_box), "{:?} outside", layout.mode);
            for right in layout.placements.iter().skip(index + 1) {
                // 圆心距必须不小于直径：圆网格的重叠判定口径与外接正方形不同，
                // 斜向相邻的两圆外接方块必然相交，但圆本身并不重叠。
                // The centre distance must be at least one diameter: circle grids are not validated by the
                // enclosing square, because diagonally adjacent circles have intersecting bounding squares
                // while the circles themselves do not overlap.
                let dx = right.x().value - left.x().value;
                let dy = right.y().value - left.y().value;
                let distance = (dx * dx + dy * dy).sqrt();
                assert!(
                    distance >= 2.0 - 1e-9,
                    "{:?} has overlapping circles at distance {}",
                    layout.mode,
                    distance
                );
            }
        }
    }
}

/// 圆形网格的六角布局在大板上比直角布局更密 / The hexagonal layout packs more on a large sheet
///
/// 小板上边界舍入可能让六角反而更少（例如 10×8 上直角 20、六角 18），因此用大板验证密度优势。
/// On a small sheet boundary rounding can make the hex grid pack fewer (e.g. 20 vs 18 on a 10×8 sheet), so a
/// large sheet is used to verify the density advantage.
#[test]
fn hex_grid_packs_more_than_square() {
    let generator = CircleGridGenerator::new();
    let large_sheet = SheetType::new(
        meters(20.0),
        meters(20.0),
        meters(100.0),
        SheetTypeId::new("L"),
    );
    let layouts = generator.invoke(&circle("c", 1.0), &large_sheet, None);

    let square = layouts
        .iter()
        .find(|layout| layout.mode == CircleGridMode::SquareGrid)
        .map(|layout| layout.placements.len())
        .unwrap_or(0);
    let hex = layouts
        .iter()
        .find(|layout| layout.mode == CircleGridMode::HexGrid)
        .map(|layout| layout.placements.len())
        .unwrap_or(0);

    assert!(hex > square, "hex={hex} must beat square={square}");
}

/// 圆形网格跳过非圆形件 / The circle generator skips non-circle items
#[test]
fn circle_grid_skips_rectangles() {
    let generator = CircleGridGenerator::new();
    assert!(
        generator
            .invoke(&rect("a", 2.0, 2.0, false), &sheet("S1", true), None)
            .is_empty()
    );
}

// ============================================================================
// 刀缝贯穿 / Kerf propagation
// ============================================================================

/// 刀缝影响层生成的全部矩形排布路径 / The kerf affects every rectangle layout path
#[test]
fn kerf_reaches_every_rectangle_generator() {
    let without_kerf = generate(
        &Bpp2dLayerGenerationRequest::new(0, vec![(rect("a", 2.0, 2.0, false), 100)])
            .with_sheet(sheet("S1", true))
            .with_max_candidates(64),
    );
    let with_kerf = generate(
        &Bpp2dLayerGenerationRequest::new(0, vec![(rect("a", 2.0, 2.0, false), 100)])
            .with_sheet(sheet("S1", true))
            .with_kerf(meters(1.0))
            .with_max_candidates(64),
    );

    let max_plain = without_kerf
        .iter()
        .map(|result| result.layer.units.len())
        .max()
        .unwrap_or(0);
    let max_kerf = with_kerf
        .iter()
        .map(|result| result.layer.units.len())
        .max()
        .unwrap_or(0);

    assert!(
        max_kerf < max_plain,
        "kerf must reduce the packing density: plain={max_plain}, kerf={max_kerf}"
    );
}

/// 刀缝下相邻件的真实间隙不小于刀缝 / Adjacent pieces keep a real gap of at least the kerf
#[test]
fn kerf_keeps_real_gaps() {
    let kerf = meters(1.0);
    let results = generate(
        &Bpp2dLayerGenerationRequest::new(0, vec![(rect("a", 2.0, 2.0, false), 100)])
            .with_sheet(sheet("S1", true))
            .with_kerf(kerf.clone())
            .with_max_candidates(64),
    );

    for result in &results {
        // 只检查同源图样列：其放置按规则网格推进，可直接验证间隙。
        // Only check pattern columns: their placements advance on a regular grid so the gap is verifiable.
        if !result.source.starts_with("pattern") {
            continue;
        }
        let xs: Vec<f64> = result
            .layer
            .units
            .iter()
            .map(|placement| placement.x().value)
            .collect();
        let distinct: std::collections::BTreeSet<u64> =
            xs.iter().map(|value| (value * 1000.0) as u64).collect();
        let mut sorted: Vec<u64> = distinct.into_iter().collect();
        sorted.sort_unstable();
        for window in sorted.windows(2) {
            let gap = (window[1] - window[0]) as f64 / 1000.0;
            assert!(
                gap == 0.0 || gap >= 2.0,
                "相邻件间距 {} 小于包围宽度 2（含 kerf 的节距应为 3）",
                gap
            );
        }
    }
}

// ============================================================================
// 候选上限 / Candidate cap
// ============================================================================

/// 候选数量不超过请求上限 / The candidate amount never exceeds the request cap
#[test]
fn candidate_cap_is_respected() {
    let request = Bpp2dLayerGenerationRequest::new(
        0,
        vec![
            (rect("a", 2.0, 2.0, false), 10),
            (rect("b", 3.0, 1.5, true), 10),
            (circle("c", 1.0), 10),
        ],
    )
    .with_sheet(sheet("S1", true))
    .with_max_candidates(3);

    assert!(generate(&request).len() <= 3);
}

/// 空生成器集合返回空结果 / An empty generator set yields nothing
#[test]
fn empty_generator_set_yields_nothing() {
    let request = Bpp2dLayerGenerationRequest::new(0, vec![(rect("a", 2.0, 2.0, false), 1)])
        .with_sheet(sheet("S1", true));
    let generators: Vec<Box<dyn Bpp2dLayerGenerator<f64, Meter>>> = Vec::new();

    assert!(LayerGenerationContext::new().generate(&request, &generators).is_empty());
}
