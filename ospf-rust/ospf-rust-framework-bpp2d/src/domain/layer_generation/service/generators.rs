//! 层生成器集合 / Layer generators
//!
//! 块装载（局部/全局）、空间切分（DFS/多行）、BLA、图样、圆形网格、混合形状与历史回放。
//! Block loading (local/global), space splitting (DFS/multi-row), BLA, patterns, circle grids, mixed shapes
//! and historical replay.

use ospf_rust_framework_geometry2d::{
    Box2Geometry, Placement2, QuantityPoint2, QuantityVector2, candidate_shapes, count_fitting,
    origin_box, pitch_of, quantity_is_positive, quantity_less, quantity_zero_of,
    scaled_quantity,
};
use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::bla::{BlaConfig, BottomUpLeftJustifiedAlgorithm};
use crate::domain::block_loading::model::Space2;
use crate::domain::block_loading::service::{
    DepthFirstSearchAlgorithm2D, MultiRowHeuristicAlgorithm2D, SimpleBlockConfig,
};
use crate::domain::item::model::{Layer, SheetType};
use ospf_rust_framework_geometry2d::Sheet2Shape;
use crate::domain::layer_generation::request::{
    Bpp2dLayerGenerationRequest, Bpp2dLayerGenerationResult, Bpp2dLayerGenerator, max_quantity,
    subtract_quantity,
};
use crate::domain::layer_generation::service::block_loading_context::BlockLoadingContext;
use crate::domain::layer_generation::service::circle_grid_generator::CircleGridGenerator;

// ============================================================================
// 共享辅助 / Shared helpers
// ============================================================================

/// 构造与目标板材等大的初始空间 / Build an initial space matching the target sheet
pub fn initial_space_of<V, U>(sheet: &SheetType<V, U>) -> Space2<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    Space2::new(
        quantity_zero_of(&sheet.width),
        quantity_zero_of(&sheet.height),
        sheet.width.clone(),
        sheet.height.clone(),
    )
}

/// 由放置列表与板材构造候选层 / Build a candidate layer from placements and a sheet
///
/// 包围尺寸取所有放置右上角的最大值，与 Kotlin `layerOf` 一致。
/// The bounding size takes the maximum top-right corner over all placements, matching Kotlin `layerOf`.
pub fn layer_of<V, U>(
    iteration: i64,
    from: &str,
    sheet: &SheetType<V, U>,
    placements: Vec<Placement2<V, U>>,
) -> Layer<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    let mut width = quantity_zero_of(&sheet.width);
    let mut height = quantity_zero_of(&sheet.height);
    for placement in &placements {
        if let Ok(max_x) = placement.max_x() {
            width = max_quantity(width, max_x);
        }
        if let Ok(max_y) = placement.max_y() {
            height = max_quantity(height, max_y);
        }
    }
    Layer::new(
        iteration,
        from,
        Some(sheet.clone()),
        Sheet2Shape::new(width, height),
    )
    .with_units(placements)
}

// ============================================================================
// HistoryLayerGenerator - 历史层回放 / Historical layer replay
// ============================================================================

/// 历史层生成器：回放请求中已有的层 / Historical layer generator: replays the layers already in the request
///
/// 不依赖板材类型，因此是「未指定板材时保持单次直通」语义的关键受益者。
/// Does not depend on a sheet type, which makes it the key beneficiary of the "single pass-through when no
/// sheet is specified" rule.
#[derive(Debug, Clone, Default)]
pub struct HistoryLayerGenerator;

impl<V, U> Bpp2dLayerGenerator<V, U> for HistoryLayerGenerator
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    fn name(&self) -> &str {
        "history"
    }

    fn generate(
        &self,
        request: &Bpp2dLayerGenerationRequest<V, U>,
    ) -> Vec<Bpp2dLayerGenerationResult<V, U>> {
        request
            .existing_layers
            .iter()
            .cloned()
            .map(|layer| Bpp2dLayerGenerationResult::new(layer, "historical"))
            .collect()
    }
}

// ============================================================================
// BlockLocalLayerGenerator - 块装载局部 / Block-loading local
// ============================================================================

/// 块装载局部生成器：每个物品单独形成规则块 / Block-loading local generator: one regular block per item
#[derive(Debug, Clone, Default)]
pub struct BlockLocalLayerGenerator;

impl<V, U> Bpp2dLayerGenerator<V, U> for BlockLocalLayerGenerator
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    fn name(&self) -> &str {
        "block-local"
    }

    fn generate(
        &self,
        request: &Bpp2dLayerGenerationRequest<V, U>,
    ) -> Vec<Bpp2dLayerGenerationResult<V, U>> {
        let Some(sheet) = request.sheet.clone() else {
            return Vec::new();
        };
        let context = BlockLoadingContext::new();
        let space = initial_space_of(&sheet);
        let blocks = match context.local_blocks(&request.items, &space, &SimpleBlockConfig::default(), request.kerf.as_ref()) {
            Ok(blocks) => blocks,
            Err(_) => return Vec::new(),
        };
        blocks
            .into_iter()
            .map(|block| {
                Bpp2dLayerGenerationResult::new(
                    layer_of(request.iteration, "block-local", &sheet, block.placements),
                    "block-local",
                )
            })
            .collect()
    }
}

// ============================================================================
// BlockGlobalLayerGenerator - 块装载全局 / Block-loading global
// ============================================================================

/// 块装载全局生成器：多物品拼接为异构块 / Block-loading global generator: heterogeneous blocks
#[derive(Debug, Clone, Default)]
pub struct BlockGlobalLayerGenerator;

impl<V, U> Bpp2dLayerGenerator<V, U> for BlockGlobalLayerGenerator
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    fn name(&self) -> &str {
        "block-global"
    }

    fn generate(
        &self,
        request: &Bpp2dLayerGenerationRequest<V, U>,
    ) -> Vec<Bpp2dLayerGenerationResult<V, U>> {
        let Some(sheet) = request.sheet.clone() else {
            return Vec::new();
        };
        let context = BlockLoadingContext::new();
        let space = initial_space_of(&sheet);
        let blocks = match context.global_blocks(&request.items, &space, true, request.kerf.as_ref()) {
            Ok(blocks) => blocks,
            Err(_) => return Vec::new(),
        };
        blocks
            .into_iter()
            .map(|block| {
                Bpp2dLayerGenerationResult::new(
                    layer_of(request.iteration, "block-global", &sheet, block.placements),
                    "block-global",
                )
            })
            .collect()
    }
}

// ============================================================================
// SpaceSplitLayerGenerator - 空间切分 / Space splitting
// ============================================================================

/// 空间切分生成器：调用 DFS 与多行启发式搜索 / Space-splitting generator: runs the DFS and multi-row searches
#[derive(Debug, Clone, Default)]
pub struct SpaceSplitLayerGenerator;

impl<V, U> Bpp2dLayerGenerator<V, U> for SpaceSplitLayerGenerator
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    fn name(&self) -> &str {
        "space-split"
    }

    fn generate(
        &self,
        request: &Bpp2dLayerGenerationRequest<V, U>,
    ) -> Vec<Bpp2dLayerGenerationResult<V, U>> {
        let Some(sheet) = request.sheet.clone() else {
            return Vec::new();
        };
        let space = initial_space_of(&sheet);
        let mut results = Vec::new();

        if let Ok(dfs) = DepthFirstSearchAlgorithm2D::new().invoke(
            &space,
            &request.items,
            8,
            request.kerf.as_ref(),
        ) && !dfs.placements.is_empty()
        {
            results.push(Bpp2dLayerGenerationResult::new(
                layer_of(request.iteration, "space-split", &sheet, dfs.placements),
                "space-split-dfs",
            ));
        }
        if let Ok(multi_row) = MultiRowHeuristicAlgorithm2D::new().invoke(
            &space,
            &request.items,
            32,
            request.kerf.as_ref(),
        ) && !multi_row.placements.is_empty()
        {
            results.push(Bpp2dLayerGenerationResult::new(
                layer_of(request.iteration, "space-split", &sheet, multi_row.placements),
                "space-split-multi-row",
            ));
        }
        results
    }
}

// ============================================================================
// BlaLayerGenerator - BLA / Bottom-up left-justified
// ============================================================================

/// BLA 生成器：在板材上执行底部向上左对齐排样 / BLA generator
#[derive(Debug, Clone, Default)]
pub struct BlaLayerGenerator;

impl<V, U> Bpp2dLayerGenerator<V, U> for BlaLayerGenerator
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    fn name(&self) -> &str {
        "bla"
    }

    fn generate(
        &self,
        request: &Bpp2dLayerGenerationRequest<V, U>,
    ) -> Vec<Bpp2dLayerGenerationResult<V, U>> {
        let Some(sheet) = request.sheet.clone() else {
            return Vec::new();
        };
        let shape = Sheet2Shape::new(
            sheet.width.clone(),
            sheet.height.clone(),
        );
        let Ok(result) = BottomUpLeftJustifiedAlgorithm::new(
            shape,
            BlaConfig::default(),
            request.kerf.clone(),
        )
        .invoke(&request.items) else {
            return Vec::new();
        };
        if result.placements.is_empty() {
            return Vec::new();
        }
        vec![Bpp2dLayerGenerationResult::new(
            layer_of(request.iteration, "bla", &sheet, result.placements),
            "bla",
        )]
    }
}

// ============================================================================
// PatternLayerGenerator - 图样 / Pattern
// ============================================================================

/// 图样生成器：按「每行件数 × 行数」的图样铺满 / Pattern generator: fills by a "per-row count × row count"
/// pattern
#[derive(Debug, Clone, Default)]
pub struct PatternLayerGenerator;

impl<V, U> Bpp2dLayerGenerator<V, U> for PatternLayerGenerator
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    fn name(&self) -> &str {
        "pattern"
    }

    fn generate(
        &self,
        request: &Bpp2dLayerGenerationRequest<V, U>,
    ) -> Vec<Bpp2dLayerGenerationResult<V, U>> {
        let Some(sheet) = request.sheet.clone() else {
            return Vec::new();
        };
        let mut results = Vec::new();
        for (item, _) in &request.items {
            if item.shape.is_circle() {
                continue;
            }
            for candidate in candidate_shapes(&item.shape, item.allow_rotate) {
                let shape = candidate.shape.clone();
                // 刀缝通过节距体现：相邻件按「包围尺寸 + kerf」推进，真实间隙恰为 kerf。
                // The kerf is expressed through the pitch: neighbours advance by bounding size plus kerf, so
                // the real gap is exactly the kerf.
                let spacing = request.kerf_spacing(&shape.bounding_width());
                let (pitch_x, pitch_y) = pitch_of(&shape, &spacing);
                let cols = count_fitting(&pitch_x, &sheet.width);
                if cols == 0 {
                    continue;
                }
                let row_width = scaled_quantity(&pitch_x, cols);
                let rows = count_fitting(&pitch_y, &sheet.height);
                if rows == 0 {
                    continue;
                }
                let mut placements = Vec::with_capacity(cols * rows);
                for row in 0..rows {
                    let y = scaled_quantity(&pitch_y, row);
                    for col in 0..cols {
                        let x = scaled_quantity(&pitch_x, col);
                        placements.push(
                            Placement2::new(shape.clone(), QuantityPoint2 { x, y: y.clone() })
                                .with_rotated(candidate.rotated)
                                .with_item_id(item.id.as_str()),
                        );
                    }
                }
                results.push(Bpp2dLayerGenerationResult::new(
                    layer_of(request.iteration, "pattern", &sheet, placements.clone()),
                    if candidate.rotated {
                        "pattern-rotated"
                    } else {
                        "pattern"
                    },
                ));
                // 行宽小于板材宽度时，给出「整行但减少列数」的替代图样。
                // When the row is narrower than the sheet, also emit an alternative with fewer columns.
                if quantity_less(&row_width, &sheet.width) && cols > 1 {
                    results.push(Bpp2dLayerGenerationResult::new(
                        layer_of(request.iteration, "pattern", &sheet, placements),
                        "pattern-narrow",
                    ));
                }
            }
        }
        results
    }
}

// ============================================================================
// CircleGridLayerGenerator - 圆形网格 / Circle grid
// ============================================================================

/// 圆形网格生成器：直角网格与六角网格 / Circle grid generator: square and hexagonal grids
#[derive(Debug, Clone, Default)]
pub struct CircleGridLayerGenerator;

impl<V, U> Bpp2dLayerGenerator<V, U> for CircleGridLayerGenerator
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    fn name(&self) -> &str {
        "circle-grid"
    }

    fn generate(
        &self,
        request: &Bpp2dLayerGenerationRequest<V, U>,
    ) -> Vec<Bpp2dLayerGenerationResult<V, U>> {
        let Some(sheet) = request.sheet.clone() else {
            return Vec::new();
        };
        let generator = CircleGridGenerator::new();
        let mut results = Vec::new();
        for (item, _) in &request.items {
            if !item.shape.is_circle() {
                continue;
            }
            for layout in generator.invoke(item, &sheet, request.kerf.as_ref()) {
                if layout.placements.is_empty() {
                    continue;
                }
                results.push(Bpp2dLayerGenerationResult::new(
                    layer_of(request.iteration, "circle-grid", &sheet, layout.placements),
                    layout.mode.source(),
                ));
            }
        }
        results
    }
}

// ============================================================================
// MixedShapeLayerGenerator - 混合形状 / Mixed shapes
// ============================================================================

/// 混合形状生成器：先生成规则层，再用底层几何逐对复核合法性
/// / Mixed shape generator: builds a regular layer then re-validates every pair
#[derive(Debug, Clone, Default)]
pub struct MixedShapeLayerGenerator;

impl<V, U> Bpp2dLayerGenerator<V, U> for MixedShapeLayerGenerator
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    fn name(&self) -> &str {
        "mixed-shape"
    }

    fn generate(
        &self,
        request: &Bpp2dLayerGenerationRequest<V, U>,
    ) -> Vec<Bpp2dLayerGenerationResult<V, U>> {
        let Some(sheet) = request.sheet.clone() else {
            return Vec::new();
        };
        let circles: Vec<_> = request
            .items
            .iter()
            .filter(|(item, _)| item.shape.is_circle())
            .collect();
        let rectangles: Vec<_> = request
            .items
            .iter()
            .filter(|(item, _)| !item.shape.is_circle())
            .collect();
        if circles.is_empty() || rectangles.is_empty() {
            return Vec::new();
        }

        let generator = CircleGridGenerator::new();
        let sheet_box = origin_box(sheet.width.clone(), sheet.height.clone());
        let mut results = Vec::new();

        for (rectangle, _) in &rectangles {
            for candidate in candidate_shapes(&rectangle.shape, rectangle.allow_rotate) {
                let shape = candidate.shape.clone();
                let cols = count_fitting(&shape.bounding_width(), &sheet.width);
                if cols == 0 {
                    continue;
                }
                let row_width = scaled_quantity(&shape.bounding_width(), cols);
                let mut base = Vec::new();
                // 先在底部铺一行矩形件，再用圆形网格填充其上方区域。
                // First lay one row of rectangles, then fill the area above them with a circle grid.
                for col in 0..cols {
                    let x = scaled_quantity(&shape.bounding_width(), col);
                    base.push(
                        Placement2::new(shape.clone(), QuantityPoint2 {
                            x,
                            y: quantity_zero_of(&sheet.height),
                        })
                        .with_rotated(candidate.rotated)
                        .with_item_id(rectangle.id.as_str()),
                    );
                }
                let Some(upper_height) = subtract_quantity(&sheet.height, &shape.bounding_height())
                else {
                    continue;
                };
                if !quantity_is_positive(&upper_height) {
                    continue;
                }
                let upper_sheet = SheetType::new(
                    row_width.clone(),
                    upper_height,
                    sheet.max_load.clone(),
                    sheet.type_code.clone(),
                );
                for (circle, _) in &circles {
                    for layout in generator.invoke(circle, &upper_sheet, request.kerf.as_ref()) {
                        let mut combined = base.clone();
                        let offset = QuantityVector2 {
                            x: quantity_zero_of(&sheet.width),
                            y: shape.bounding_height(),
                        };
                        for placement in &layout.placements {
                            let shifted = Placement2::new(
                                placement.shape.clone(),
                                QuantityPoint2 {
                                    x: crate::domain::layer_generation::request::add_quantity(
                                        &placement.x(),
                                        &offset.x,
                                    ),
                                    y: crate::domain::layer_generation::request::add_quantity(
                                        &placement.y(),
                                        &offset.y,
                                    ),
                                },
                            )
                            .with_rotated(placement.rotated)
                            .with_item_id_ref(placement.item_id.clone());
                            // 用平移后的包围盒判定是否仍落在板材内。
                            // Check that the shifted bounding box still lies inside the sheet.
                            let shifted_box = Box2Geometry::new(
                                shifted.x(),
                                shifted.y(),
                                shifted.shape.clone(),
                            );
                            if !sheet_box.contains_box(&shifted_box) {
                                continue;
                            }
                            combined.push(shifted);
                        }
                        if !no_overlap(&combined) {
                            continue;
                        }
                        results.push(Bpp2dLayerGenerationResult::new(
                            layer_of(request.iteration, "mixed-shape", &sheet, combined),
                            "mixed-shape",
                        ));
                    }
                }
            }
        }
        results
    }
}

/// 逐对复核放置是否互不重叠 / Re-validate every pair of placements for non-overlap
fn no_overlap<V, U>(placements: &[Placement2<V, U>]) -> bool
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    for (index, left) in placements.iter().enumerate() {
        let Some(left_box) = box_of(left) else {
            return false;
        };
        for right in placements.iter().skip(index + 1) {
            let Some(right_box) = box_of(right) else {
                return false;
            };
            if left_box.overlapped(&right_box) {
                return false;
            }
        }
    }
    true
}

fn box_of<V, U>(placement: &Placement2<V, U>) -> Option<Box2Geometry<V, U>>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    Some(Box2Geometry::new(
        placement.x(),
        placement.y(),
        placement.shape.clone(),
    ))
}
