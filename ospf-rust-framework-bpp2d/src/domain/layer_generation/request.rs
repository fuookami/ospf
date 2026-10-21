//! 层生成请求与结果 / Layer generation request and result
//!
//! 对应 BPP3D 的 `Bpp3dLayerGenerationRequest` / `Bpp3dLayerGenerationResult`：候选列生成的输入与输出。
//! 二维侧把「箱」换为「板材」，把「层」保持为层。
//! Mirrors BPP3D `Bpp3dLayerGenerationRequest` / `Bpp3dLayerGenerationResult`: the input and output of
//! candidate column generation. 2D replaces bins with sheets while keeping layers.

use std::collections::BTreeMap;

use ospf_rust_framework_geometry2d::{quantity_is_positive, quantity_zero_of};
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::concept::UnitTrait;

use crate::domain::item::model::{ActualItem, DemandModeKey, Layer, SheetType};

pub use crate::domain::item::model::Bpp2dDemandEntry as LayerGenerationDemandEntry;

// ============================================================================
// Bpp2dLayerGenerationRequest - 层生成请求 / Layer generation request
// ============================================================================

/// 层生成请求 / Layer generation request
#[derive(Debug, Clone)]
pub struct Bpp2dLayerGenerationRequest<V, U: UnitTrait> {
    /// 当前迭代 / Current iteration
    pub iteration: i64,
    /// 主板材类型 / Primary sheet type
    pub sheet: Option<SheetType<V, U>>,
    /// 参与候选生成的全部板材类型；为空时退化为仅 `sheet` / All sheet types taking part in candidate
    /// generation; empty means only `sheet`
    pub sheets: Vec<SheetType<V, U>>,
    /// 参与排样的物品与件数 / Items and amounts taking part in the layout
    pub items: Vec<(ActualItem<V, U>, usize)>,
    /// 已有层，供历史生成器回放 / Existing layers replayed by the historical generator
    pub existing_layers: Vec<Layer<V, U>>,
    /// 需求项 / Demand entries
    pub demand_entries: Vec<LayerGenerationDemandEntry>,
    /// 影子价格 / Shadow prices
    pub shadow_prices: BTreeMap<DemandModeKey, V>,
    /// 刀缝间距 / Kerf spacing
    pub kerf: Option<Quantity<V, U>>,
    /// 最大候选数 / Maximum candidate amount
    pub max_candidates: usize,
}

impl<V, U> Bpp2dLayerGenerationRequest<V, U>
where
    V: Clone,
    U: UnitTrait + Clone,
{
    /// 创建层生成请求 / Create a layer generation request
    pub fn new(iteration: i64, items: Vec<(ActualItem<V, U>, usize)>) -> Self {
        Self {
            iteration,
            sheet: None,
            sheets: Vec::new(),
            items,
            existing_layers: Vec::new(),
            demand_entries: Vec::new(),
            shadow_prices: BTreeMap::new(),
            kerf: None,
            max_candidates: 256,
        }
    }

    /// 设置主板材类型 / Set the primary sheet type
    pub fn with_sheet(mut self, sheet: SheetType<V, U>) -> Self {
        self.sheet = Some(sheet);
        self
    }

    /// 设置全部参与生成的板材类型 / Set every sheet type taking part in generation
    ///
    /// 主板材排在最前，其余按传入顺序去重。与 `with_sheet` 同时使用时 `sheets` 优先。
    /// The main sheet type comes first and the rest keep their input order, deduplicated. When combined with
    /// `with_sheet`, `sheets` takes precedence.
    pub fn with_sheets(mut self, sheets: Vec<SheetType<V, U>>) -> Self {
        self.sheets = order_candidate_sheets(sheets);
        self
    }

    /// 设置已有层 / Set existing layers
    pub fn with_existing_layers(mut self, layers: Vec<Layer<V, U>>) -> Self {
        self.existing_layers = layers;
        self
    }

    /// 设置需求项 / Set demand entries
    pub fn with_demand_entries(mut self, entries: Vec<LayerGenerationDemandEntry>) -> Self {
        self.demand_entries = entries;
        self
    }

    /// 设置刀缝间距 / Set kerf spacing
    pub fn with_kerf(mut self, kerf: Quantity<V, U>) -> Self {
        self.kerf = Some(kerf);
        self
    }

    /// 设置最大候选数 / Set the maximum candidate amount
    pub fn with_max_candidates(mut self, max_candidates: usize) -> Self {
        self.max_candidates = max_candidates;
        self
    }

    /// 实际参与生成的板材序列 / Sheet sequence actually used by generation
    ///
    /// 未指定任何板材时返回空序列，调用方应保持单次直通，使不依赖板材的生成器（历史层）仍可工作。
    /// Returns an empty sequence when no sheet is specified; callers must keep a single pass-through so that
    /// generators that do not depend on a sheet (the historical one) keep working.
    pub fn candidate_sheets(&self) -> Vec<SheetType<V, U>> {
        if !self.sheets.is_empty() {
            return self.sheets.clone();
        }
        self.sheet.clone().into_iter().collect()
    }
}

impl<V, U> Bpp2dLayerGenerationRequest<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: UnitTrait + ospf_rust_quantities::unit::physical_unit::CTUnit + Clone,
{
    /// 是否含圆形件 / Whether any circle item is present
    pub fn has_circle_items(&self) -> bool {
        self.items.iter().any(|(item, _)| item.shape.is_circle())
    }

    /// 是否含矩形件 / Whether any rectangle item is present
    pub fn has_rectangle_items(&self) -> bool {
        self.items.iter().any(|(item, _)| !item.shape.is_circle())
    }

    /// 读取刀缝间距 / Read the kerf spacing
    ///
    /// 未设置刀缝时返回与样本同单位的零间距；调用方总是持有形状，因此零值一定可构造。
    /// Returns a zero spacing in the sample's unit when the kerf is unset; callers always hold a shape, so a
    /// zero value is always constructible.
    pub fn kerf_spacing(&self, sample: &Quantity<V, U>) -> Quantity<V, U> {
        match &self.kerf {
            Some(kerf) if quantity_is_positive(kerf) => kerf.clone(),
            _ => quantity_zero_of(sample),
        }
    }
}

/// 排序列出参与生成的板材类型 / Order the sheet types taking part in generation
///
/// 主板材排在最前，其余按传入顺序去重。
/// The main sheet type comes first; the rest keep their input order, deduplicated.
pub fn order_candidate_sheets<V: Clone, U: UnitTrait + Clone>(
    sheets: Vec<SheetType<V, U>>,
) -> Vec<SheetType<V, U>> {
    let mut ordered: Vec<SheetType<V, U>> = Vec::new();
    if let Some(main_sheet) = sheets.iter().find(|sheet| sheet.is_main) {
        ordered.push(SheetType::clone(main_sheet));
    }
    for sheet in sheets {
        if !ordered
            .iter()
            .any(|existing| existing.type_code == sheet.type_code)
        {
            ordered.push(sheet);
        }
    }
    ordered
}

// ============================================================================
// Bpp2dLayerGenerationResult - 层生成结果 / Layer generation result
// ============================================================================

/// 层生成结果 / Layer generation result
#[derive(Debug, Clone)]
pub struct Bpp2dLayerGenerationResult<V, U: UnitTrait> {
    /// 候选层 / Candidate layer
    pub layer: Layer<V, U>,
    /// 检验数 / Reduced cost
    pub reduced_cost: Option<V>,
    /// 评分 / Score
    pub score: Option<V>,
    /// 生成来源 / Generation source
    pub source: String,
}

impl<V, U> Bpp2dLayerGenerationResult<V, U>
where
    U: UnitTrait + Clone,
{
    /// 创建层生成结果 / Create a layer generation result
    pub fn new(layer: Layer<V, U>, source: impl Into<String>) -> Self {
        Self {
            layer,
            reduced_cost: None,
            score: None,
            source: source.into(),
        }
    }

    /// 设置评分 / Set the score
    pub fn with_score(mut self, score: V) -> Self {
        self.score = Some(score);
        self
    }

    /// 设置检验数 / Set the reduced cost
    pub fn with_reduced_cost(mut self, reduced_cost: V) -> Self {
        self.reduced_cost = Some(reduced_cost);
        self
    }
}

// ============================================================================
// 层生成器接口 / Layer generator trait
// ============================================================================

/// 层生成器接口 / Layer generator interface
pub trait Bpp2dLayerGenerator<V, U: UnitTrait> {
    /// 生成器名称 / Generator name
    fn name(&self) -> &str;

    /// 生成候选层 / Generate candidate layers
    fn generate(&self, request: &Bpp2dLayerGenerationRequest<V, U>)
    -> Vec<Bpp2dLayerGenerationResult<V, U>>;
}

// ============================================================================
// 同量纲加减 / Same-dimension add and subtract
// ============================================================================

/// 同量纲加法 / Same-dimension addition
///
/// 坐标推进与节距累加统一走这里，量纲不兼容时返回左值本身而不是 panic。
/// Coordinate advancement and pitch accumulation go through here; an incompatible dimension returns the
/// left value instead of panicking.
pub fn add_quantity<V, U>(lhs: &Quantity<V, U>, rhs: &Quantity<V, U>) -> Quantity<V, U>
where
    V: ospf_rust_math::algebra::concept::Field + Clone,
    U: UnitTrait + Clone,
{
    if lhs.unit.dimension_symbol() != rhs.unit.dimension_symbol() {
        return lhs.clone();
    }
    Quantity::new(lhs.value.clone() + rhs.value.clone(), lhs.unit.clone())
}

/// 同量纲减法 / Same-dimension subtraction
///
/// 量纲不兼容时返回 `None`，由调用方决定回退策略。
/// Returns `None` on an incompatible dimension so the caller can choose the fallback.
pub fn subtract_quantity<V, U>(
    lhs: &Quantity<V, U>,
    rhs: &Quantity<V, U>,
) -> Option<Quantity<V, U>>
where
    V: ospf_rust_math::algebra::concept::Field + Clone,
    U: UnitTrait + Clone,
{
    if lhs.unit.dimension_symbol() != rhs.unit.dimension_symbol() {
        return None;
    }
    Some(Quantity::new(
        lhs.value.clone() - rhs.value.clone(),
        lhs.unit.clone(),
    ))
}

/// 同量纲取较大者 / Take the larger of two same-dimension quantities
pub fn max_quantity<V, U>(lhs: Quantity<V, U>, rhs: Quantity<V, U>) -> Quantity<V, U>
where
    V: PartialOrd,
    U: UnitTrait,
{
    if rhs.value > lhs.value {
        rhs
    } else {
        lhs
    }
}
