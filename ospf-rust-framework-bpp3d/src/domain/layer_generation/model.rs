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

/// 层生成堆叠单元 / Layer-generation stacking unit
///
/// 暴露给 request-scoped package rule policy 的轻量候选单元，避免
/// 下游依赖 Pattern/Pile 内部实现类型。
/// Lightweight candidate unit exposed to request-scoped package rule policies,
/// avoiding downstream dependency on Pattern/Pile internal implementation types.
#[derive(Debug, Clone)]
pub struct LayerGenerationStackingUnit<V> {
    /// 货物索引 / Item index
    pub item_index: usize,
    /// 货物 ID / Item id
    pub item_id: ItemId,
    /// 朝向 / Orientation
    pub orientation: Orientation,
    /// 朝向是否在普通允许列表中 / Whether orientation is normally enabled
    pub orientation_enabled: bool,
    /// 宽度 / Width
    pub width: V,
    /// 深度 / Depth
    pub depth: V,
    /// 高度 / Height
    pub height: V,
    /// 重量 / Weight
    pub weight: V,
}

// ============================================================================
// LayerGenerationRequest - 层生成请求 / Layer generation request
// ============================================================================

/// 层生成包装规则策略 / Layer-generation package rule policy
///
/// 为 Kotlin `extraOrientationRule` / `extraStackingOnRule` 闭包语义提供
/// request-scoped 扩展点，同时保持 `PackageAttribute` 本体可克隆、可比较和
/// 可序列化友好。
/// Provides a request-scoped extension point for Kotlin
/// `extraOrientationRule` / `extraStackingOnRule` closure semantics while
/// keeping `PackageAttribute` itself cloneable, comparable, and serialization-friendly.
pub trait LayerGenerationPackageRulePolicy<V, U>: Debug + Send + Sync
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
    /// 判断候选朝向是否允许 / Check whether candidate orientation is allowed
    fn allows_orientation(
        &self,
        _item: &ActualItem<V, U>,
        _attribute: Option<&PackageAttribute>,
        _orientation: Orientation,
        _input: &PackageOrientationRuleInput,
    ) -> bool {
        true
    }

    /// 判断放置级堆叠是否允许 / Check whether placement-level stacking is allowed
    fn allows_placement_stacking(
        &self,
        _units: &[LayerGenerationStackingUnit<V>],
        _top_index: usize,
        _input: &PackagePlacementStackingInput<'_>,
    ) -> bool {
        true
    }
}

/// 默认包装规则策略 / Default package rule policy
#[derive(Debug, Clone, Default)]
pub struct DefaultLayerGenerationPackageRulePolicy;

impl<V, U> LayerGenerationPackageRulePolicy<V, U> for DefaultLayerGenerationPackageRulePolicy
where
    V: Debug + Clone + Send + Sync,
    U: ospf_rust_quantities::unit::concept::UnitTrait + Debug + Clone + Send + Sync,
{
}

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
    /// 包装属性 / Package attributes
    pub package_attributes: HashMap<ItemId, PackageAttribute>,
    /// 包装规则策略 / Package rule policy
    pub package_rule_policy: Arc<dyn LayerGenerationPackageRulePolicy<V, U>>,
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
            package_attributes: HashMap::new(),
            package_rule_policy: Arc::new(DefaultLayerGenerationPackageRulePolicy),
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

    /// 设置包装属性 / Set package attributes
    pub fn with_package_attributes<I>(
        mut self,
        attributes: HashMap<I, PackageAttribute>,
    ) -> Self
    where
        I: Into<ItemId>,
    {
        self.package_attributes = attributes
            .into_iter()
            .map(|(id, attribute)| (id.into(), attribute))
            .collect();
        self
    }

    /// 设置包装规则策略 / Set package rule policy
    pub fn with_package_rule_policy(
        mut self,
        policy: Arc<dyn LayerGenerationPackageRulePolicy<V, U>>,
    ) -> Self {
        self.package_rule_policy = policy;
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
    pub item_id: ItemId,
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
    pub item_id: ItemId,
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

