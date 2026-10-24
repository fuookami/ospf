//! 二维层生成领域 / 2D layer generation domain
//!
//! 映射 Kotlin `bpp2d-domain-layer-generation-context`：候选层生成、组合上下文与刀缝贯穿。
//! Maps the Kotlin `bpp2d-domain-layer-generation-context` submodule: candidate layer generation, the
//! composite context and kerf propagation.

pub mod request;
pub mod service;

pub use request::{
    Bpp2dLayerGenerationRequest, Bpp2dLayerGenerationResult, Bpp2dLayerGenerator,
    LayerGenerationDemandEntry, add_quantity, max_quantity, order_candidate_sheets,
    subtract_quantity,
};
pub use service::*;

use std::cmp::Ordering;

use ospf_rust_quantities::unit::concept::UnitTrait;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::item::model::Layer;

// ============================================================================
// LayerGenerationContext - 组合式层生成上下文 / Composite layer generation context
// ============================================================================

/// 组合式层生成上下文 / Composite layer generation context
///
/// 按板材类型展开生成器链，再按列身份去重并按评分排序。
/// Expands the generator chain per sheet type, deduplicates by column identity and ranks by score.
#[derive(Debug, Clone, Default)]
pub struct LayerGenerationContext;

impl LayerGenerationContext {
    /// 创建上下文 / Create the context
    pub fn new() -> Self {
        Self
    }

    /// 默认生成器集合 / Default generator set
    pub fn default_generators<V, U>() -> Vec<Box<dyn Bpp2dLayerGenerator<V, U>>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        vec![
            Box::new(BlockLocalLayerGenerator),
            Box::new(BlockGlobalLayerGenerator),
            Box::new(SpaceSplitLayerGenerator),
            Box::new(BlaLayerGenerator),
            Box::new(PatternLayerGenerator),
            Box::new(CircleGridLayerGenerator),
            Box::new(MixedShapeLayerGenerator),
            Box::new(HistoryLayerGenerator),
        ]
    }

    /// 生成层候选 / Generate layer candidates
    ///
    /// 多板材类型逐类型生成，列自带 `Layer.sheet`；未指定任何板材时保持单次直通，
    /// 使不依赖板材的历史层生成器仍可工作。
    /// Candidates are generated per sheet type and carry `Layer.sheet`; when no sheet is specified the
    /// request is passed through once so the historical generator keeps working.
    pub fn generate<V, U>(
        &self,
        request: &Bpp2dLayerGenerationRequest<V, U>,
        generators: &[Box<dyn Bpp2dLayerGenerator<V, U>>],
    ) -> Vec<Bpp2dLayerGenerationResult<V, U>>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: UnitTrait + CTUnit + Clone,
    {
        if generators.is_empty() {
            return Vec::new();
        }
        let sheets = request.candidate_sheets();
        let sheet_requests: Vec<Bpp2dLayerGenerationRequest<V, U>> = if sheets.is_empty() {
            vec![request.clone()]
        } else {
            sheets
                .into_iter()
                .map(|sheet| Bpp2dLayerGenerationRequest {
                    sheet: Some(sheet),
                    ..request.clone()
                })
                .collect()
        };

        let selected = select_generators(generators, request);
        let mut all = Vec::new();
        for sheet_request in &sheet_requests {
            if all.len() >= request.max_candidates {
                break;
            }
            for generator in &selected {
                all.extend(generator.generate(sheet_request));
                if all.len() >= request.max_candidates {
                    break;
                }
            }
        }
        all.truncate(request.max_candidates);
        rank_by_area(all)
    }
}

/// 按请求的物品形状挑选生成器 / Pick generators by the requested item shapes
///
/// 纯矩形请求排除圆形专用生成器；纯圆形请求只保留圆网格与历史；混合请求保留全部。
/// Pure rectangle requests exclude the circle-only generators; pure circle requests keep only the circle
/// grid and history; mixed requests keep everything.
fn select_generators<'a, V, U>(
    generators: &'a [Box<dyn Bpp2dLayerGenerator<V, U>>],
    request: &Bpp2dLayerGenerationRequest<V, U>,
) -> Vec<&'a Box<dyn Bpp2dLayerGenerator<V, U>>>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    let has_circle = request.has_circle_items();
    let has_rectangle = request.has_rectangle_items();
    generators
        .iter()
        .filter(|generator| {
            let name = generator.name();
            let is_circle_only = name == "circle-grid" || name == "mixed-shape";
            if !has_circle && has_rectangle {
                // 纯矩形请求排除圆形专用生成器。
                return !is_circle_only;
            }
            if has_circle && !has_rectangle {
                // 纯圆形请求只走圆网格与历史生成器。
                return name == "circle-grid" || name == "history";
            }
            true
        })
        .collect()
}

/// 按面积降序排序候选层 / Rank candidate layers by descending area
///
/// 无影子价格评分时面积是唯一可比较的通用口径。
/// Without a shadow-price score, area is the only universally comparable measure.
fn rank_by_area<V, U>(
    results: Vec<Bpp2dLayerGenerationResult<V, U>>,
) -> Vec<Bpp2dLayerGenerationResult<V, U>>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    let mut results = results;
    results.sort_by(|lhs, rhs| {
        let left = lhs
            .score
            .as_ref()
            .map(|score| score.clone())
            .unwrap_or_else(|| V::zero());
        let right = rhs
            .score
            .as_ref()
            .map(|score| score.clone())
            .unwrap_or_else(|| V::zero());
        let left_value = if lhs.score.is_some() {
            left
        } else {
            lhs.layer.area().value.clone()
        };
        let right_value = if rhs.score.is_some() {
            right
        } else {
            rhs.layer.area().value.clone()
        };
        right_value
            .partial_cmp(&left_value)
            .unwrap_or(Ordering::Equal)
    });
    results
}

/// 去重后的候选层 / Deduplicated candidate layers
///
/// 按列身份键去重：板材类型、来源、包围尺寸与放置集合共同参与。
/// Deduplicates by the column identity key: sheet type, source, bounding size and placement set together.
pub fn deduplicate_layers<V, U>(layers: Vec<Layer<V, U>>) -> Vec<Layer<V, U>>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + CTUnit + Clone,
{
    let mut seen = std::collections::HashSet::new();
    layers
        .into_iter()
        .filter(|layer| seen.insert(layer.identity_key()))
        .collect()
}
