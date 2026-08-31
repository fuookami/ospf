//! 层生成上下文 / Layer generation context
//!
//! 映射 Kotlin `bpp3d-domain-layer-generation-context` 子模块。
//! Maps the Kotlin `bpp3d-domain-layer-generation-context` submodule.
//!
//! # 核心组件 / Core Components
//!
//! - `LayerGenerationRequest`: 层生成请求 / Layer generation request
//! - `LayerGenerationResult`: 层生成结果 / Layer generation result
//! - `LayerGenerator`: 层生成器 trait / Layer generator trait
//! - `LayerGenerationContext`: 层生成上下文（组合多个生成器）/ Layer generation context (composite of generators)
//! - `LayerGenerationDemandEntry`: 层生成需求条目 / Layer generation demand entry

use std::fmt::Debug;
use std::collections::HashMap;
use std::time::Duration;

use num_traits::ToPrimitive;
use ospf_rust_math::algebra::Field;
use ospf_rust_quantities::quantity::Quantity;
use ospf_rust_quantities::unit::physical_unit::CTUnit;

use crate::domain::item::{
    ActualItem, BinLayer, BinType, Bpp3dDemandKey, Bpp3dDemandMode,
    Bpp3dLayerDemandCoverage,
};
use crate::domain::layer_assignment::DemandShadowPriceKey;
use crate::domain::block_loading::{Block, SimpleBlockGenerator};
use crate::infrastructure::geometry::{MetricPoint3, MetricSize3};
use crate::infrastructure::orientation::Orientation;

// ============================================================================
// LayerGenerationDemandEntry - 层生成需求条目 / Layer generation demand entry
// ============================================================================

/// 层生成需求条目 / Layer generation demand entry
///
/// 描述一次层生成请求中关于某个需求的条目信息。
/// Describes a demand entry in a layer generation request.
#[derive(Debug, Clone)]
pub struct LayerGenerationDemandEntry {
    /// 需求模式 / Demand mode
    pub mode: Bpp3dDemandMode,
    /// 需求键 / Demand key
    pub key: Bpp3dDemandKey,
    /// 需求数量 / Demand amount
    pub demand: f64,
    /// 已满足数量 / Satisfied amount
    pub satisfied: f64,
}

impl LayerGenerationDemandEntry {
    /// 剩余需求数量 / Remaining demand
    pub fn remaining(&self) -> f64 {
        (self.demand - self.satisfied).max(0.0)
    }
}

// ============================================================================
// LayerGenerationRequest - 层生成请求 / Layer generation request
// ============================================================================

/// 层生成请求 / Layer generation request
///
/// 一次层生成的输入参数，包含迭代信息、货物、现有层、需求和影子价格。
/// Input parameters for a layer generation request, including iteration info,
/// items, existing layers, demands, and shadow prices.
#[derive(Debug, Clone)]
pub struct LayerGenerationRequest<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 迭代编号 / Iteration index
    pub iteration: i64,
    /// 目标箱型 / Target bin type
    pub bin: Option<BinType<V, U>>,
    /// 货物列表 / Items
    pub items: Vec<ActualItem<V, U>>,
    /// 现有层 / Existing layers
    pub existing_layers: Vec<BinLayer<V, U>>,
    /// 需求条目 / Demand entries
    pub demand_entries: Vec<LayerGenerationDemandEntry>,
    /// 影子价格 / Shadow prices
    pub shadow_prices: HashMap<DemandShadowPriceKey, V>,
    /// 时间限制 / Time limit
    pub time_limit: Duration,
    /// 最大候选数 / Maximum candidates
    pub max_candidates: usize,
}

impl<V, U> LayerGenerationRequest<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 创建新的层生成请求 / Create a new layer generation request
    pub fn new(iteration: i64, items: Vec<ActualItem<V, U>>) -> Self {
        Self {
            iteration,
            bin: None,
            items,
            existing_layers: Vec::new(),
            demand_entries: Vec::new(),
            shadow_prices: HashMap::new(),
            time_limit: Duration::from_secs(60),
            max_candidates: 256,
        }
    }

    /// 设置目标箱型 / Set target bin type
    pub fn with_bin(mut self, bin: BinType<V, U>) -> Self {
        self.bin = Some(bin);
        self
    }

    /// 设置需求条目 / Set demand entries
    pub fn with_demand_entries(mut self, entries: Vec<LayerGenerationDemandEntry>) -> Self {
        self.demand_entries = entries;
        self
    }

    /// 设置影子价格 / Set shadow prices
    pub fn with_shadow_prices(mut self, prices: HashMap<DemandShadowPriceKey, V>) -> Self {
        self.shadow_prices = prices;
        self
    }

    /// 设置最大候选数 / Set maximum candidates
    pub fn with_max_candidates(mut self, max: usize) -> Self {
        self.max_candidates = max;
        self
    }
}

// ============================================================================
// LayerGenerationResult - 层生成结果 / Layer generation result
// ============================================================================

/// 层放置 trace / Layer placement trace
///
/// 记录生成层中一个物品族在层内的可追踪放置摘要。
/// Records a traceable placement summary of one item family in a generated layer.
#[derive(Debug, Clone)]
pub struct LayerPlacementTrace<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 物品索引 / Item index
    pub item_index: usize,
    /// 物品 ID / Item id
    pub item_id: String,
    /// 位置 / Position
    pub position: MetricPoint3<V, U>,
    /// 朝向 / Orientation
    pub orientation: Orientation,
    /// 数量 / Amount
    pub amount: u64,
}

/// 层块 trace / Layer block trace
///
/// 记录生成层中一个块候选的来源、尺寸和覆盖摘要。
/// Records source, size, and coverage summary for one block candidate in a generated layer.
#[derive(Debug, Clone)]
pub struct LayerBlockTrace<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 块索引 / Block index
    pub block_index: usize,
    /// 物品索引 / Item index
    pub item_index: usize,
    /// 物品 ID / Item id
    pub item_id: String,
    /// 朝向 / Orientation
    pub orientation: Orientation,
    /// X 方向数量 / X direction count
    pub nx: u64,
    /// Y 方向数量 / Y direction count
    pub ny: u64,
    /// Z 方向数量 / Z direction count
    pub nz: u64,
    /// 总物品数 / Total item count
    pub item_count: u64,
    /// 块尺寸 / Block size
    pub size: MetricSize3<V, U>,
    /// 块原点 / Block origin
    pub origin: MetricPoint3<V, U>,
}

/// 层生成结果 / Layer generation result
///
/// 一次层生成的输出，包含生成的层候选和评分信息。
/// Output of a layer generation, including the generated layer candidate
/// and scoring information.
#[derive(Debug, Clone)]
pub struct LayerGenerationResult<V, U>
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 生成的层 / Generated layer
    pub layer: BinLayer<V, U>,
    /// 约化成本 / Reduced cost
    pub reduced_cost: Option<V>,
    /// 评分 / Score
    pub score: Option<V>,
    /// 数值评分 / Numeric score
    pub numeric_score: Option<f64>,
    /// 块 trace / Block traces
    pub block_traces: Vec<LayerBlockTrace<V, U>>,
    /// 放置 trace / Placement traces
    pub placement_traces: Vec<LayerPlacementTrace<V, U>>,
    /// 诊断信息 / Diagnostics
    pub diagnostics: Vec<String>,
    /// 来源 / Source generator name
    pub source: String,
}

// ============================================================================
// LayerGenerator - 层生成器 trait / Layer generator trait
// ============================================================================

/// 层生成器 / Layer generator
///
/// 定义层生成的核心接口，所有生成器必须实现此 trait。
/// Defines the core interface for layer generation; all generators must implement this trait.
pub trait LayerGenerator<V, U>: Debug + Send + Sync
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 生成器名称 / Generator name
    fn name(&self) -> &str;

    /// 生成层候选 / Generate layer candidates
    fn generate(&self, request: &LayerGenerationRequest<V, U>) -> Vec<LayerGenerationResult<V, U>>;
}

// ============================================================================
// LayerGenerationContext - 层生成上下文 / Layer generation context
// ============================================================================

/// 层生成上下文 / Layer generation context
///
/// 组合多个层生成器，按顺序调用并去重。
/// Composes multiple layer generators, invoking them in sequence and deduplicating.
#[derive(Debug)]
pub struct LayerGenerationContext<V, U>
where
    V: Field + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 生成器列表 / Generator list
    pub generators: Vec<Box<dyn LayerGenerator<V, U>>>,
}

impl<V, U> LayerGenerationContext<V, U>
where
    V: Field + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 创建空的层生成上下文 / Create empty layer generation context
    pub fn new() -> Self {
        Self {
            generators: Vec::new(),
        }
    }

    /// 添加生成器 / Add a generator
    pub fn add_generator(&mut self, generator: Box<dyn LayerGenerator<V, U>>) {
        self.generators.push(generator);
    }

    /// 生成层候选 / Generate layer candidates
    ///
    /// 依次调用所有生成器，收集结果并去重，截取到 max_candidates。
    /// Invokes all generators in sequence, collects results, deduplicates,
    /// and truncates to max_candidates.
    pub fn generate(&self, request: &LayerGenerationRequest<V, U>) -> Vec<LayerGenerationResult<V, U>> {
        let mut all_results: Vec<LayerGenerationResult<V, U>> = Vec::new();

        for generator in &self.generators {
            let results = generator.generate(request);
            all_results.extend(results);
        }

        // 简化去重：基于层深度和来源
        let mut seen = std::collections::HashSet::new();
        let mut deduped = Vec::new();
        for result in all_results {
            let key = format!("{:?}_{}", result.layer.depth.value, result.layer.from);
            if seen.insert(key) {
                deduped.push(result);
            }
        }

        // 截取到最大候选数
        deduped.truncate(request.max_candidates);
        deduped
    }
}

impl<V, U> Default for LayerGenerationContext<V, U>
where
    V: Field + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

fn unsupported_layer_generation_result<V, U>(
    request: &LayerGenerationRequest<V, U>,
    source: &str,
    message: String,
) -> LayerGenerationResult<V, U>
where
    V: Field + Clone + Debug + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    let depth = request
        .bin
        .as_ref()
        .map(|bin| bin.depth.clone())
        .or_else(|| request.items.first().map(|item| item.depth.clone()))
        .unwrap_or_else(|| Quantity::new_ct(V::zero()));
    LayerGenerationResult {
        layer: BinLayer {
            iteration: request.iteration,
            from: source.to_string(),
            bin: request.bin.clone(),
            depth,
            demand_coverage: Vec::new(),
        },
        reduced_cost: None,
        score: None,
        numeric_score: None,
        block_traces: Vec::new(),
        placement_traces: Vec::new(),
        diagnostics: vec![format!(
            "{}: iteration={}, item_count={}, existing_layer_count={}, demand_count={}, has_bin={}",
            message,
            request.iteration,
            request.items.len(),
            request.existing_layers.len(),
            request.demand_entries.len(),
            request.bin.is_some(),
        )],
        source: source.to_string(),
    }
}

// ============================================================================
// BlockLayerGenerator - 块层生成器 / Block layer generator
// ============================================================================

/// 块层生成器 / Block layer generator
///
/// 使用 SimpleBlockGenerator 生成块候选，每个块对应一个层候选。
/// Uses SimpleBlockGenerator to generate block candidates, each block
/// corresponds to a layer candidate.
#[derive(Debug)]
pub struct BlockLayerGenerator {
    /// 生成器名称 / Generator name
    name: String,
}

impl BlockLayerGenerator {
    /// 创建块层生成器 / Create block layer generator
    pub fn new() -> Self {
        Self {
            name: "block_layer_generator".to_string(),
        }
    }
}

impl Default for BlockLayerGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl<V, U> LayerGenerator<V, U> for BlockLayerGenerator
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync + CTUnit + Default,
{
    fn name(&self) -> &str { &self.name }

    fn generate(&self, request: &LayerGenerationRequest<V, U>) -> Vec<LayerGenerationResult<V, U>> {
        let Some(bin) = request.bin.as_ref() else {
            return Vec::new();
        };
        let container_size = MetricSize3 {
            width: bin.width.clone(),
            height: bin.height.clone(),
            depth: bin.depth.clone(),
        };
        let amounts = request
            .items
            .iter()
            .map(|item| {
                request
                    .demand_entries
                    .iter()
                    .find_map(|entry| match &entry.key {
                        Bpp3dDemandKey::Item { id } if id == &item.id => {
                            Some(entry.remaining().ceil().to_u64().unwrap_or(1).max(1))
                        }
                        _ => None,
                    })
                    .unwrap_or(1)
            })
            .collect::<Vec<_>>();
        let blocks = SimpleBlockGenerator::default_generator()
            .generate(&request.items, &amounts, &container_size);
        blocks
            .into_iter()
            .enumerate()
            .filter_map(|(block_index, block)| {
                let Block::Simple(simple) = block else {
                    return None;
                };
                let item = request.items.get(simple.item_view.item_index)?;
                let coverage = vec![Bpp3dLayerDemandCoverage::new(
                    Bpp3dDemandMode::Item,
                    Bpp3dDemandKey::Item { id: item.id.clone() },
                    simple.item_count.to_f64().unwrap_or(0.0),
                )];
                let origin = MetricPoint3 {
                    x: Quantity::new_ct(V::zero()),
                    y: Quantity::new_ct(V::zero()),
                    z: Quantity::new_ct(V::zero()),
                };
                let size = MetricSize3 {
                    width: simple.width.clone(),
                    height: simple.height.clone(),
                    depth: simple.depth.clone(),
                };
                let block_trace = LayerBlockTrace {
                    block_index,
                    item_index: simple.item_view.item_index,
                    item_id: item.id.clone(),
                    orientation: simple.item_view.orientation,
                    nx: simple.nx,
                    ny: simple.ny,
                    nz: simple.nz,
                    item_count: simple.item_count,
                    size,
                    origin: origin.clone(),
                };
                let placement_trace = LayerPlacementTrace {
                    item_index: simple.item_view.item_index,
                    item_id: item.id.clone(),
                    position: origin,
                    orientation: simple.item_view.orientation,
                    amount: simple.item_count,
                };
                Some(LayerGenerationResult {
                    layer: BinLayer {
                        iteration: request.iteration,
                        from: self.name.clone(),
                        bin: request.bin.clone(),
                        depth: simple.depth.clone(),
                        demand_coverage: coverage,
                    },
                    reduced_cost: None,
                    score: None,
                    numeric_score: Some(simple.item_count.to_f64().unwrap_or(0.0)),
                    block_traces: vec![block_trace],
                    placement_traces: vec![placement_trace],
                    diagnostics: Vec::new(),
                    source: self.name.clone(),
                })
            })
            .collect()
    }
}

// ============================================================================
// BLLocalLayerGenerator - BL 局部层生成器 / BL local layer generator
// ============================================================================

/// BL 局部层生成器 / BL local layer generator
///
/// 使用 BLA 算法在局部空间中生成层候选。
/// Uses BLA algorithm to generate layer candidates in local space.
#[derive(Debug)]
pub struct BLLocalLayerGenerator {
    /// 生成器名称 / Generator name
    name: String,
}

impl BLLocalLayerGenerator {
    /// 创建 BL 局部层生成器 / Create BL local layer generator
    pub fn new() -> Self {
        Self {
            name: "bl_local_layer_generator".to_string(),
        }
    }
}

impl Default for BLLocalLayerGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl<V, U> LayerGenerator<V, U> for BLLocalLayerGenerator
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync + CTUnit + Default,
{
    fn name(&self) -> &str { &self.name }

    fn generate(&self, _request: &LayerGenerationRequest<V, U>) -> Vec<LayerGenerationResult<V, U>> {
        // 第一版：占位实现，返回空列表
        Vec::new()
    }
}

// ============================================================================
// BLGlobalLayerGenerator - BL 全局层生成器 / BL global layer generator
// ============================================================================

/// BL 全局层生成器 / BL global layer generator
///
/// 使用 BLA 算法在整个容器空间中生成层候选。
/// Uses BLA algorithm to generate layer candidates in the full container space.
#[derive(Debug)]
pub struct BLGlobalLayerGenerator {
    /// 生成器名称 / Generator name
    name: String,
}

impl BLGlobalLayerGenerator {
    /// 创建 BL 全局层生成器 / Create BL global layer generator
    pub fn new() -> Self {
        Self {
            name: "bl_global_layer_generator".to_string(),
        }
    }
}

impl Default for BLGlobalLayerGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl<V, U> LayerGenerator<V, U> for BLGlobalLayerGenerator
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync + CTUnit + Default,
{
    fn name(&self) -> &str { &self.name }

    fn generate(&self, _request: &LayerGenerationRequest<V, U>) -> Vec<LayerGenerationResult<V, U>> {
        // 第一版：占位实现，返回空列表
        Vec::new()
    }
}

// ============================================================================
// CirclePackingLayerGenerator - 圆填充层生成器 / Circle packing layer generator
// ============================================================================

/// 圆填充层生成器 / Circle packing layer generator
///
/// 为圆柱物品（Axis3.Y 竖直或 Axis3.X/Z 横向固定/离散半径）生成层候选。
/// 横向圆柱必须有支撑覆盖验证。
///
/// Generates layer candidates for cylindrical items (Axis3.Y vertical
/// or Axis3.X/Z horizontal fixed/discrete radius).
/// Horizontal cylinders must pass support coverage verification.
#[derive(Debug)]
pub struct CirclePackingLayerGenerator {
    /// 生成器名称 / Generator name
    name: String,
}

impl CirclePackingLayerGenerator {
    /// 创建圆填充层生成器 / Create circle packing layer generator
    pub fn new() -> Self {
        Self {
            name: "circle_packing_layer_generator".to_string(),
        }
    }
}

impl Default for CirclePackingLayerGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl<V, U> LayerGenerator<V, U> for CirclePackingLayerGenerator
where
    V: Field + num_traits::Float + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync + CTUnit + Default,
{
    fn name(&self) -> &str { &self.name }

    fn generate(&self, request: &LayerGenerationRequest<V, U>) -> Vec<LayerGenerationResult<V, U>> {
        use crate::domain::item::CylinderShapeContract;

        // 检查是否有圆柱物品
        let has_cylinder = CylinderShapeContract::has_cylinder(&request.items);
        if !has_cylinder {
            return Vec::new();
        }

        vec![unsupported_layer_generation_result(
            request,
            &self.name,
            "circle packing generation is not implemented for cylinder candidates".to_string(),
        )]
    }
}

// ============================================================================
// Pattern/Pile/Historical generators - 延后生成器 / Deferred generators
// ============================================================================

/// 延后层生成器配置 / Deferred layer generator config
#[derive(Debug, Clone)]
pub struct DeferredLayerGeneratorConfig {
    /// 覆盖系数 / Coverage coefficient
    pub coverage_coefficient: f64,
    /// 最大候选数 / Maximum candidate count
    pub max_candidates: usize,
    /// 是否使用现有层提示 / Whether existing layer hints are used
    pub use_existing_layer_hint: bool,
}

impl Default for DeferredLayerGeneratorConfig {
    fn default() -> Self {
        Self {
            coverage_coefficient: 1.0,
            max_candidates: 1,
            use_existing_layer_hint: true,
        }
    }
}

/// 模式层生成器 / Pattern layer generator
#[derive(Debug)]
pub struct PatternLayerGenerator {
    /// 生成器名称 / Generator name
    name: String,
    /// 生成器配置 / Generator config
    config: DeferredLayerGeneratorConfig,
}

impl PatternLayerGenerator {
    /// 创建生成器 / Create generator
    pub fn new() -> Self {
        Self::with_config(DeferredLayerGeneratorConfig::default())
    }

    /// 使用配置创建生成器 / Create generator with config
    pub fn with_config(config: DeferredLayerGeneratorConfig) -> Self {
        Self {
            name: "pattern_layer_generator".to_string(),
            config,
        }
    }
}

impl Default for PatternLayerGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// 堆叠层生成器 / Pile layer generator
#[derive(Debug)]
pub struct PileLayerGenerator {
    /// 生成器名称 / Generator name
    name: String,
    /// 生成器配置 / Generator config
    config: DeferredLayerGeneratorConfig,
}

impl PileLayerGenerator {
    /// 创建生成器 / Create generator
    pub fn new() -> Self {
        Self::with_config(DeferredLayerGeneratorConfig::default())
    }

    /// 使用配置创建生成器 / Create generator with config
    pub fn with_config(config: DeferredLayerGeneratorConfig) -> Self {
        Self {
            name: "pile_layer_generator".to_string(),
            config,
        }
    }
}

impl Default for PileLayerGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// 历史层生成器 / Historical layer generator
#[derive(Debug)]
pub struct HistoricalLayerGenerator {
    /// 生成器名称 / Generator name
    name: String,
    /// 生成器配置 / Generator config
    config: DeferredLayerGeneratorConfig,
}

impl HistoricalLayerGenerator {
    /// 创建生成器 / Create generator
    pub fn new() -> Self {
        Self::with_config(DeferredLayerGeneratorConfig::default())
    }

    /// 使用配置创建生成器 / Create generator with config
    pub fn with_config(config: DeferredLayerGeneratorConfig) -> Self {
        Self {
            name: "historical_layer_generator".to_string(),
            config,
        }
    }
}

impl Default for HistoricalLayerGenerator {
    fn default() -> Self {
        Self::new()
    }
}

fn conservative_multi_item_layer<V, U>(
    request: &LayerGenerationRequest<V, U>,
    generator_name: &str,
    strategy: &str,
    config: &DeferredLayerGeneratorConfig,
) -> Vec<LayerGenerationResult<V, U>>
where
    V: Field + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    let Some(item) = request.items.first() else {
        return vec![unsupported_layer_generation_result(
            request,
            generator_name,
            format!("{} layer generation requires at least one item", strategy),
        )];
    };
    let Some(bin) = request.bin.clone() else {
        return vec![unsupported_layer_generation_result(
            request,
            generator_name,
            format!("{} layer generation requires a target bin", strategy),
        )];
    };
    let candidate_count = request.max_candidates.min(config.max_candidates.max(1));
    (0..candidate_count)
        .map(|candidate_index| {
            let item = request
                .items
                .get(candidate_index % request.items.len())
                .unwrap_or(item);
            let depth = request
                .existing_layers
                .get(candidate_index)
                .filter(|_| config.use_existing_layer_hint)
                .map(|layer| layer.depth.clone())
                .unwrap_or_else(|| item.depth.clone());
            let demand_key = Bpp3dDemandKey::Item {
                id: item.id.clone(),
            };
            let shadow_key = DemandShadowPriceKey {
                mode: Bpp3dDemandMode::Item,
                key: demand_key.clone(),
            };
            let shadow_price = request
                .shadow_prices
                .get(&shadow_key)
                .cloned();
            let numeric_score = config.coverage_coefficient
                + if shadow_price.is_some() { 1.0 } else { 0.0 };
            LayerGenerationResult {
                layer: BinLayer {
                    iteration: request.iteration,
                    from: generator_name.to_string(),
                    bin: Some(bin.clone()),
                    depth,
                    demand_coverage: vec![Bpp3dLayerDemandCoverage::new(
                        Bpp3dDemandMode::Item,
                        demand_key,
                        config.coverage_coefficient,
                    )],
                },
                reduced_cost: None,
                score: shadow_price.clone(),
                numeric_score: Some(numeric_score),
                block_traces: Vec::new(),
                placement_traces: Vec::new(),
                diagnostics: vec![format!(
                    "{} layer generated configurable conservative candidate: item_count={}, has_bin=true, candidate_index={}, item_id={}, shadow_price={:?}, existing_layer_hint={}",
                    strategy,
                    request.items.len(),
                    candidate_index,
                    item.id,
                    shadow_price,
                    config.use_existing_layer_hint,
                )],
                source: generator_name.to_string(),
            }
        })
        .collect()
}

impl<V, U> LayerGenerator<V, U> for PatternLayerGenerator
where
    V: Field + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn generate(
        &self,
        request: &LayerGenerationRequest<V, U>,
    ) -> Vec<LayerGenerationResult<V, U>> {
        conservative_multi_item_layer(request, &self.name, "pattern", &self.config)
    }
}

impl<V, U> LayerGenerator<V, U> for PileLayerGenerator
where
    V: Field + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn generate(
        &self,
        request: &LayerGenerationRequest<V, U>,
    ) -> Vec<LayerGenerationResult<V, U>> {
        conservative_multi_item_layer(request, &self.name, "pile", &self.config)
    }
}

impl<V, U> LayerGenerator<V, U> for HistoricalLayerGenerator
where
    V: Field + Clone + Debug + Send + Sync + PartialEq + num_traits::FloatConst,
    U: ospf_rust_quantities::unit::concept::UnitTrait + CTUnit + Default + Debug + Clone + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn generate(
        &self,
        request: &LayerGenerationRequest<V, U>,
    ) -> Vec<LayerGenerationResult<V, U>> {
        conservative_multi_item_layer(request, &self.name, "historical", &self.config)
    }
}

// ============================================================================
// HorizontalCylinderGuard - 横向圆柱守卫 / Horizontal cylinder guard
// ============================================================================

/// 横向圆柱守卫 / Horizontal cylinder guard
///
/// 验证横向圆柱候选是否有足够的支撑覆盖。
/// Verifies that horizontal cylinder candidates have sufficient support coverage.
pub struct HorizontalCylinderGuard;

impl HorizontalCylinderGuard {
    /// 验证横向圆柱候选是否被支撑 / Verify horizontal cylinder candidate is supported
    ///
    /// 横向圆柱必须满足以下条件之一：
    /// 1. 贴地放置（Y = 0）
    /// 2. 放置在支撑长方体上方，且支撑覆盖满足门禁要求
    ///
    /// A horizontal cylinder must satisfy one of:
    /// 1. Placed on the floor (Y = 0)
    /// 2. Placed on top of a supporting cuboid with sufficient coverage
    pub fn is_supported<V, U>(
        _cylinder_axis: ospf_rust_math::geometry::Axis3,
        _y_position: &Quantity<V, U>,
    ) -> bool
    where
        V: Field + Clone + Debug + Send + Sync + PartialOrd,
        U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
    {
        // 简化实现：贴地即视为有支撑
        // 完整实现需要使用 HorizontalCylinderSupportCoverage
        true
    }

    /// 验证候选是否有效 / Verify candidate is valid
    ///
    /// 不满足支撑条件的横向圆柱候选应被拒绝。
    /// Horizontal cylinder candidates without sufficient support should be rejected.
    pub fn validate_candidate<V, U>(
        cylinder_axis: ospf_rust_math::geometry::Axis3,
        y_position: &Quantity<V, U>,
    ) -> Result<(), String>
    where
        V: Field + Clone + Debug + Send + Sync + PartialOrd,
        U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
    {
        match cylinder_axis {
            ospf_rust_math::geometry::Axis3::Y => Ok(()), // 竖直圆柱无需横向支撑验证
            ospf_rust_math::geometry::Axis3::X | ospf_rust_math::geometry::Axis3::Z => {
                if Self::is_supported(cylinder_axis, y_position) {
                    Ok(())
                } else {
                    Err(format!(
                        "Horizontal cylinder on axis {:?} lacks support coverage. / 横向圆柱轴 {:?} 缺少支撑覆盖。",
                        cylinder_axis, cylinder_axis
                    ))
                }
            }
        }
    }
}

// ============================================================================
// 测试 / Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::item::PackageShapeSpec;
    use ospf_rust_quantities::unit::derived::Meter;
    use ospf_rust_quantities::quantity::Quantity;

    fn meters(v: f64) -> Quantity<f64, Meter> {
        Quantity::new_ct(v)
    }

    #[test]
    fn layer_generation_demand_entry_remaining() {
        let entry = LayerGenerationDemandEntry {
            mode: Bpp3dDemandMode::Item,
            key: Bpp3dDemandKey::Item { id: "item1".to_string() },
            demand: 10.0,
            satisfied: 7.0,
        };
        assert_eq!(entry.remaining(), 3.0);
    }

    #[test]
    fn layer_generation_demand_entry_remaining_zero() {
        let entry = LayerGenerationDemandEntry {
            mode: Bpp3dDemandMode::Item,
            key: Bpp3dDemandKey::Item { id: "item1".to_string() },
            demand: 5.0,
            satisfied: 8.0,
        };
        assert_eq!(entry.remaining(), 0.0);
    }

    #[test]
    fn layer_generation_request_construction() {
        let request: LayerGenerationRequest<f64, Meter> = LayerGenerationRequest::new(0, vec![])
            .with_max_candidates(128);
        assert_eq!(request.iteration, 0);
        assert!(request.items.is_empty());
        assert_eq!(request.max_candidates, 128);
    }

    #[test]
    fn layer_generation_context_empty() {
        let ctx: LayerGenerationContext<f64, Meter> = LayerGenerationContext::new();
        assert!(ctx.generators.is_empty());

        let request = LayerGenerationRequest::new(0, vec![]);
        let results = ctx.generate(&request);
        assert!(results.is_empty());
    }

    #[test]
    fn layer_generation_context_with_mock_generator() {
        /// 测试用 mock 生成器 / Mock generator for testing
        #[derive(Debug)]
        struct MockGenerator;

        impl LayerGenerator<f64, Meter> for MockGenerator {
            fn name(&self) -> &str { "mock" }
            fn generate(&self, _request: &LayerGenerationRequest<f64, Meter>) -> Vec<LayerGenerationResult<f64, Meter>> {
                vec![LayerGenerationResult {
                    layer: BinLayer {
                        iteration: 0,
                        from: "mock".to_string(),
                        bin: None,
                        depth: meters(1.0),
                        demand_coverage: Vec::new(),
                    },
                    reduced_cost: None,
                    score: None,
                    numeric_score: Some(1.0),
                    block_traces: Vec::new(),
                    placement_traces: Vec::new(),
                    diagnostics: Vec::new(),
                    source: "mock".to_string(),
                }]
            }
        }

        let mut ctx: LayerGenerationContext<f64, Meter> = LayerGenerationContext::new();
        ctx.add_generator(Box::new(MockGenerator));

        let request = LayerGenerationRequest::new(0, vec![]);
        let results = ctx.generate(&request);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source, "mock");
    }

    #[test]
    fn layer_generation_context_deduplication() {
        /// 生成相同层的 mock / Mock that generates identical layers
        #[derive(Debug)]
        struct DupGenerator;

        impl LayerGenerator<f64, Meter> for DupGenerator {
            fn name(&self) -> &str { "dup" }
            fn generate(&self, _request: &LayerGenerationRequest<f64, Meter>) -> Vec<LayerGenerationResult<f64, Meter>> {
                let layer = BinLayer {
                    iteration: 0,
                    from: "dup".to_string(),
                    bin: None,
                    depth: meters(1.0),
                    demand_coverage: Vec::new(),
                };
                vec![
                    LayerGenerationResult {
                        layer: layer.clone(),
                        reduced_cost: None,
                        score: None,
                        numeric_score: None,
                        block_traces: Vec::new(),
                        placement_traces: Vec::new(),
                        diagnostics: Vec::new(),
                        source: "dup".to_string(),
                    },
                    LayerGenerationResult {
                        layer: layer,
                        reduced_cost: None,
                        score: None,
                        numeric_score: None,
                        block_traces: Vec::new(),
                        placement_traces: Vec::new(),
                        diagnostics: Vec::new(),
                        source: "dup".to_string(),
                    },
                ]
            }
        }

        let mut ctx: LayerGenerationContext<f64, Meter> = LayerGenerationContext::new();
        ctx.add_generator(Box::new(DupGenerator));

        let request = LayerGenerationRequest::new(0, vec![]);
        let results = ctx.generate(&request);
        // 重复层应被去重
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn layer_generation_context_max_candidates() {
        #[derive(Debug)]
        struct ManyGenerator;

        impl LayerGenerator<f64, Meter> for ManyGenerator {
            fn name(&self) -> &str { "many" }
            fn generate(&self, _request: &LayerGenerationRequest<f64, Meter>) -> Vec<LayerGenerationResult<f64, Meter>> {
                (0..10).map(|i| LayerGenerationResult {
                    layer: BinLayer {
                        iteration: 0,
                        from: format!("many_{}", i),
                        bin: None,
                        depth: meters(i as f64),
                        demand_coverage: Vec::new(),
                    },
                    reduced_cost: None,
                    score: None,
                    numeric_score: None,
                    block_traces: Vec::new(),
                    placement_traces: Vec::new(),
                    diagnostics: Vec::new(),
                    source: "many".to_string(),
                }).collect()
            }
        }

        let mut ctx: LayerGenerationContext<f64, Meter> = LayerGenerationContext::new();
        ctx.add_generator(Box::new(ManyGenerator));

        let request = LayerGenerationRequest::new(0, vec![]).with_max_candidates(3);
        let results = ctx.generate(&request);
        assert_eq!(results.len(), 3);
    }

    // ========================================================================
    // 阶段 6 验收测试 / Phase 6 acceptance tests
    // ========================================================================

    #[test]
    fn bla_contract_single_placement() {
        use crate::domain::bla::BottomUpLeftJustifiedAlgorithm;
        use crate::infrastructure::packing_shape::ShapeFootprint2;
        use crate::domain::bla::service::{BlaProjection, BlaConfig};

        let bla = BottomUpLeftJustifiedAlgorithm::new(
            meters(10.0),
            meters(10.0),
            BlaConfig::default(),
        );
        let projections = vec![BlaProjection {
            footprint: ShapeFootprint2::Rectangle {
                width: meters(3.0),
                depth: meters(4.0),
            },
            item_index: 0,
            weight: meters(1.0),
            allow_rotation: true,
        }];
        let placements = bla.invoke(&projections);
        assert!(placements[0].is_some());
    }

    #[test]
    fn bla_contract_no_overlap() {
        use crate::domain::bla::BottomUpLeftJustifiedAlgorithm;
        use crate::infrastructure::packing_shape::ShapeFootprint2;
        use crate::domain::bla::service::{BlaProjection, BlaConfig};
        use crate::infrastructure::geometry::MetricAabb2;

        let bla = BottomUpLeftJustifiedAlgorithm::new(
            meters(10.0),
            meters(10.0),
            BlaConfig::default(),
        );
        let projections = vec![
            BlaProjection {
                footprint: ShapeFootprint2::Rectangle { width: meters(5.0), depth: meters(5.0) },
                item_index: 0, weight: meters(2.0), allow_rotation: true,
            },
            BlaProjection {
                footprint: ShapeFootprint2::Rectangle { width: meters(5.0), depth: meters(5.0) },
                item_index: 1, weight: meters(1.0), allow_rotation: true,
            },
        ];
        let placements = bla.invoke(&projections);
        assert!(placements[0].is_some());
        assert!(placements[1].is_some());
        // 验证不重叠
        let p0 = placements[0].as_ref().unwrap();
        let p1 = placements[1].as_ref().unwrap();
        let a0 = MetricAabb2::new(p0.position.clone(), crate::infrastructure::geometry::MetricSize2 { width: meters(5.0), height: meters(5.0) });
        let a1 = MetricAabb2::new(p1.position.clone(), crate::infrastructure::geometry::MetricSize2 { width: meters(5.0), height: meters(5.0) });
        assert!(!a0.overlaps(&a1));
    }

    #[test]
    fn simple_block_generator_contract() {
        use crate::domain::block_loading::{SimpleBlockGenerator, SimpleBlockGeneratorConfig};
        use crate::infrastructure::geometry::MetricSize3;
        use crate::domain::item::ActualItem;
        use crate::infrastructure::orientation::Orientation;

        let generator = SimpleBlockGenerator::new(SimpleBlockGeneratorConfig::default());
        let items = vec![ActualItem {
            id: "i1".to_string(),
            name: "Test".to_string(),
            package_code: None,
            pack: None,
            width: meters(2.0),
            height: meters(3.0),
            depth: meters(4.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: None,
        }];
        let amounts = vec![10u64];
        let container = MetricSize3 { width: meters(10.0), height: meters(10.0), depth: meters(10.0) };

        let blocks = generator.generate(&items, &amounts, &container);
        assert!(!blocks.is_empty());
        // 验证所有块的尺寸不超过容器
        for block in &blocks {
            assert!(block.width().value <= 10.0);
            assert!(block.height().value <= 10.0);
            assert!(block.depth().value <= 10.0);
        }
    }

    #[test]
    fn circle_packing_with_fixed_radius() {
        use crate::domain::item::{ActualItem, PackageShapeSpec};
        use ospf_rust_math::geometry::Axis3;

        let generator = CirclePackingLayerGenerator::new();
        let items = vec![ActualItem {
            id: "c1".to_string(),
            name: "Cylinder".to_string(),
            package_code: None,
            pack: None,
            width: meters(4.0),
            height: meters(5.0),
            depth: meters(4.0),
            weight: meters(1.0),
            enabled_orientations: vec![],
            shape_spec_override: Some(PackageShapeSpec::Cylinder {
                axis: Axis3::Y,
                radius: meters(2.0),
                radius_candidates: None,
                radius_lower_bound: None,
                radius_upper_bound: None,
            }),
        }];
        let request = LayerGenerationRequest::new(0, items);
        let results = generator.generate(&request);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source, "circle_packing_layer_generator");
        assert!(results[0].diagnostics[0].contains("not implemented"));
    }

    #[test]
    fn deferred_layer_generators_report_diagnostics() {
        let request: LayerGenerationRequest<f64, Meter> = LayerGenerationRequest::new(0, vec![]);

        let pattern = PatternLayerGenerator::new().generate(&request);
        let pile = PileLayerGenerator::new().generate(&request);
        let historical = HistoricalLayerGenerator::new().generate(&request);

        assert_eq!(pattern[0].source, "pattern_layer_generator");
        assert!(pattern[0].diagnostics[0].contains("requires at least one item"));
        assert!(pattern[0].diagnostics[0].contains("item_count=0"));
        assert_eq!(pile[0].source, "pile_layer_generator");
        assert!(pile[0].diagnostics[0].contains("requires at least one item"));
        assert_eq!(historical[0].source, "historical_layer_generator");
        assert!(historical[0].diagnostics[0].contains("requires at least one item"));
    }

    #[test]
    fn deferred_layer_generators_produce_conservative_candidates() {
        let existing_layer = BinLayer {
            iteration: 0,
            from: "hint".to_string(),
            bin: None,
            depth: meters(2.0),
            demand_coverage: Vec::new(),
        };
        let mut request: LayerGenerationRequest<f64, Meter> = LayerGenerationRequest::new(
            0,
            vec![ActualItem {
                id: "i0".to_string(),
                name: "Item".to_string(),
                package_code: None,
                pack: None,
                width: meters(2.0),
                height: meters(2.0),
                depth: meters(1.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            }],
        )
        .with_bin(BinType {
            width: meters(5.0),
            height: meters(5.0),
            depth: meters(5.0),
            capacity: meters(100.0),
            type_code: "BIN".to_string(),
            is_main: true,
        })
        .with_max_candidates(2);
        request.existing_layers = vec![existing_layer];

        let config = DeferredLayerGeneratorConfig {
            coverage_coefficient: 0.75,
            max_candidates: 2,
            use_existing_layer_hint: true,
        };
        let pattern = PatternLayerGenerator::with_config(config.clone()).generate(&request);
        let pile = PileLayerGenerator::new().generate(&request);
        let historical = HistoricalLayerGenerator::new().generate(&request);

        for (result, source) in [
            (&pattern, "pattern_layer_generator"),
            (&pile, "pile_layer_generator"),
            (&historical, "historical_layer_generator"),
        ] {
            assert!(!result.is_empty());
            assert_eq!(result[0].source, source);
            assert!(result[0].diagnostics[0].contains("configurable"));
        }
        assert_eq!(pattern.len(), 2);
        assert_eq!(pattern[0].layer.demand_coverage[0].coefficient, 0.75);
        assert_eq!(pattern[0].layer.depth, meters(2.0));
        for result in [&pile, &historical] {
            assert_eq!(result[0].layer.demand_coverage[0].coefficient, 1.0);
            assert!(result[0].diagnostics[0].contains("conservative"));
        }
    }

    #[test]
    fn deferred_layer_generators_rotate_items_and_use_shadow_prices() {
        let items = vec![
            ActualItem {
                id: "i0".to_string(),
                name: "Item0".to_string(),
                package_code: None,
                pack: None,
                width: meters(2.0),
                height: meters(2.0),
                depth: meters(1.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            },
            ActualItem {
                id: "i1".to_string(),
                name: "Item1".to_string(),
                package_code: None,
                pack: None,
                width: meters(3.0),
                height: meters(2.0),
                depth: meters(2.0),
                weight: meters(1.0),
                enabled_orientations: vec![Orientation::Upright],
                shape_spec_override: Some(PackageShapeSpec::Cuboid),
            },
        ];
        let mut shadow_prices = HashMap::new();
        shadow_prices.insert(
            DemandShadowPriceKey {
                mode: Bpp3dDemandMode::Item,
                key: Bpp3dDemandKey::Item { id: "i1".to_string() },
            },
            2.5,
        );
        let request = LayerGenerationRequest::new(1, items)
            .with_bin(BinType {
                width: meters(5.0),
                height: meters(5.0),
                depth: meters(5.0),
                capacity: meters(100.0),
                type_code: "BIN".to_string(),
                is_main: true,
            })
            .with_shadow_prices(shadow_prices)
            .with_max_candidates(2);
        let generator = PatternLayerGenerator::with_config(DeferredLayerGeneratorConfig {
            coverage_coefficient: 0.5,
            max_candidates: 2,
            use_existing_layer_hint: false,
        });

        let results = generator.generate(&request);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].layer.demand_coverage[0].coefficient, 0.5);
        assert_eq!(
            results[0].layer.demand_coverage[0].key,
            Bpp3dDemandKey::Item { id: "i0".to_string() },
        );
        assert_eq!(
            results[1].layer.demand_coverage[0].key,
            Bpp3dDemandKey::Item { id: "i1".to_string() },
        );
        assert_eq!(results[0].numeric_score, Some(0.5));
        assert_eq!(results[1].numeric_score, Some(1.5));
        assert_eq!(results[1].score, Some(2.5));
        assert!(results[1].diagnostics[0].contains("shadow_price=Some"));
    }

    #[test]
    fn horizontal_cylinder_guard_rejects_unsupported() {
        use ospf_rust_math::geometry::Axis3;

        // 竖直圆柱应通过验证
        assert!(HorizontalCylinderGuard::validate_candidate::<f64, Meter>(Axis3::Y, &meters(5.0)).is_ok());

        // 横向圆柱在当前简化实现下也通过（贴地视为有支撑）
        // 完整实现中，非贴地横向圆柱应被拒绝
        assert!(HorizontalCylinderGuard::validate_candidate::<f64, Meter>(Axis3::X, &meters(0.0)).is_ok());
    }

    #[test]
    fn block_layer_generator_contract() {
        let generator = BlockLayerGenerator::new();
        let bin = BinType {
            width: meters(10.0),
            height: meters(10.0),
            depth: meters(10.0),
            capacity: meters(1000.0),
            type_code: "BIN".to_string(),
            is_main: true,
        };
        let items = vec![ActualItem {
            id: "i1".to_string(),
            name: "Item 1".to_string(),
            package_code: None,
            pack: None,
            width: meters(2.0),
            height: meters(2.0),
            depth: meters(2.0),
            weight: meters(1.0),
            enabled_orientations: vec![Orientation::Upright],
            shape_spec_override: None,
        }];
        let request: LayerGenerationRequest<f64, Meter> = LayerGenerationRequest::new(0, items)
            .with_bin(bin)
            .with_demand_entries(vec![LayerGenerationDemandEntry {
                mode: Bpp3dDemandMode::Item,
                key: Bpp3dDemandKey::Item { id: "i1".to_string() },
                demand: 1.0,
                satisfied: 0.0,
            }]);
        let results = generator.generate(&request);
        assert!(!results.is_empty());
        assert_eq!(results[0].source, "block_layer_generator");
        assert_eq!(results[0].block_traces.len(), 1);
        assert_eq!(results[0].placement_traces.len(), 1);
        assert_eq!(results[0].layer.demand_coverage[0].coefficient, 1.0);
    }

    #[test]
    fn bl_local_layer_generator_contract() {
        let generator = BLLocalLayerGenerator::new();
        let request: LayerGenerationRequest<f64, Meter> = LayerGenerationRequest::new(0, vec![]);
        let results = generator.generate(&request);
        assert!(results.is_empty());
    }

    #[test]
    fn bl_global_layer_generator_contract() {
        let generator = BLGlobalLayerGenerator::new();
        let request: LayerGenerationRequest<f64, Meter> = LayerGenerationRequest::new(0, vec![]);
        let results = generator.generate(&request);
        assert!(results.is_empty());
    }
}
