// ============================================================================
// DemandStatistics - 需求统计 / Demand statistics
// ============================================================================

/// 需求模式 / Demand mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bpp3dDemandMode {
    /// 货物需求 / Item demand
    Item,
    /// 物料需求 / Material demand
    Material,
    /// 货物数量需求 / Item amount demand
    ItemAmount,
    /// 货物重量需求 / Item weight demand
    ItemWeight,
    /// 货物物料数量需求 / Item material amount demand
    ItemMaterialAmount,
    /// 货物物料重量需求 / Item material weight demand
    ItemMaterialWeight,
}

/// 需求键 / Demand key
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Bpp3dDemandKey {
    /// 货物键 / Item key
    Item { id: String },
    /// 物料键 / Material key
    Material { no: String },
}

/// 层需求覆盖 / Layer demand coverage
///
/// 描述一个层候选对某个需求条目的覆盖系数。
/// Describes the coverage coefficient of a layer candidate for a demand entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Bpp3dLayerDemandCoverage {
    /// 需求模式 / Demand mode
    pub mode: Bpp3dDemandMode,
    /// 需求键 / Demand key
    pub key: Bpp3dDemandKey,
    /// 覆盖系数 / Coverage coefficient
    pub coefficient: f64,
}

impl Bpp3dLayerDemandCoverage {
    /// 创建覆盖条目 / Create coverage entry
    pub fn new(mode: Bpp3dDemandMode, key: Bpp3dDemandKey, coefficient: f64) -> Self {
        Self {
            mode,
            key,
            coefficient,
        }
    }
}

/// 需求值 / Demand value
#[derive(Debug, Clone)]
pub enum Bpp3dDemandValue<V, U: UnitTrait> {
    /// 数量 / Amount
    Amount(u64),
    /// 重量 / Weight
    Weight(Quantity<V, U>),
}

/// 需求统计 / Demand statistics
#[derive(Debug, Clone)]
pub struct DemandStatistics<V, U: UnitTrait> {
    /// 统计条目 / Statistics entries
    pub entries: Vec<(Bpp3dDemandKey, Bpp3dDemandValue<V, U>)>,
}

