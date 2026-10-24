//! 需求统计 / Demand statistics
//!
//! 二维需求以模式与键描述：按件数、按面积或按重量。模式与键共同构成需求身份，
//! 层生成与影子价格都按同一口径索引。
//! 2D demands are described by a mode and a key: by piece count, by area or by weight. The mode and key
//! together form the demand identity, and both layer generation and shadow prices index by that convention.

use super::id::{ItemId, MaterialNo};

// ============================================================================
// Bpp2dDemandDomain - 需求域 / Demand domain
// ============================================================================

/// 需求域：区分按物品与按物料统计 / Demand domain distinguishing item-based from material-based demand
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bpp2dDemandDomain {
    /// 物品域 / Item domain
    Item,
    /// 物料域 / Material domain
    Material,
}

// ============================================================================
// DemandModeKey - 需求模式键 / Demand mode key
// ============================================================================

/// 需求模式键：影子价格的索引口径 / Demand mode key: the shadow-price index convention
///
/// 模式、键与量纲单位共同构成索引入口，层生成评分与对偶提取共用它。
/// The mode, key and quantity unit together form the index entry; layer-generation scoring and dual
/// extraction share it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DemandModeKey {
    /// 需求模式 / Demand mode
    pub mode: Bpp2dDemandMode,
    /// 需求键文本 / Demand key text
    pub key: String,
    /// 量纲单位符号 / Quantity unit symbol
    pub quantity_unit: Option<String>,
}

impl DemandModeKey {
    /// 创建需求模式键 / Create a demand mode key
    pub fn new(mode: Bpp2dDemandMode, key: impl Into<String>) -> Self {
        Self {
            mode,
            key: key.into(),
            quantity_unit: None,
        }
    }

    /// 设置量纲单位符号 / Set the quantity unit symbol
    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.quantity_unit = Some(unit.into());
        self
    }
}

// ============================================================================
// Bpp2dDemandMode - 需求模式 / Demand mode
// ============================================================================

/// 需求模式 / Demand mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Bpp2dDemandMode {
    /// 物品件数 / Item amount
    ItemAmount,
    /// 物品面积 / Item area
    ItemArea,
    /// 物品重量 / Item weight,
    ItemWeight,
    /// 物料面积 / Material area
    MaterialArea,
    /// 物料重量 / Material weight
    MaterialWeight,
}

impl Bpp2dDemandMode {
    /// 需求域 / Demand domain
    pub fn domain(&self) -> Bpp2dDemandDomain {
        match self {
            Self::ItemAmount | Self::ItemArea | Self::ItemWeight => Bpp2dDemandDomain::Item,
            Self::MaterialArea | Self::MaterialWeight => Bpp2dDemandDomain::Material,
        }
    }
}

// ============================================================================
// Bpp2dDemandKey - 需求键 / Demand key
// ============================================================================

/// 需求键 / Demand key
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Bpp2dDemandKey {
    /// 物品键 / Item key
    Item {
        /// 物品编号 / Item identifier
        id: ItemId,
    },
    /// 物料键 / Material key
    Material {
        /// 物料编号 / Material number
        no: MaterialNo,
    },
}

impl Bpp2dDemandKey {
    /// 物品需求键 / An item demand key
    pub fn item(id: impl Into<String>) -> Self {
        Self::Item {
            id: ItemId::new(id),
        }
    }

    /// 物料需求键 / A material demand key
    pub fn material(no: impl Into<String>) -> Self {
        Self::Material {
            no: MaterialNo::new(no),
        }
    }
}

// ============================================================================
// Bpp2dDemandValue - 需求量值 / Demand value
// ============================================================================

/// 需求量值 / Demand value
#[derive(Debug, Clone, PartialEq)]
pub enum Bpp2dDemandValue {
    /// 件数 / Piece count
    Amount(u64),
    /// 无量纲量值（面积或重量）/ Dimensionless value (area or weight)
    Quantity(f64),
}

// ============================================================================
// Bpp2dDemandEntry - 需求条目 / Demand entry
// ============================================================================

/// 需求条目 / Demand entry
#[derive(Debug, Clone, PartialEq)]
pub struct Bpp2dDemandEntry {
    /// 需求模式 / Demand mode
    pub mode: Bpp2dDemandMode,
    /// 需求键 / Demand key
    pub key: Bpp2dDemandKey,
    /// 需求量值 / Demand value
    pub value: Bpp2dDemandValue,
}

impl Bpp2dDemandEntry {
    /// 创建需求条目 / Create a demand entry
    pub fn new(mode: Bpp2dDemandMode, key: Bpp2dDemandKey, value: Bpp2dDemandValue) -> Self {
        Self { mode, key, value }
    }

    /// 物品件数需求 / An item amount demand
    pub fn item_amount(id: impl Into<String>, amount: u64) -> Self {
        Self::new(
            Bpp2dDemandMode::ItemAmount,
            Bpp2dDemandKey::item(id),
            Bpp2dDemandValue::Amount(amount),
        )
    }

    /// 需求域 / Demand domain
    pub fn domain(&self) -> Bpp2dDemandDomain {
        self.mode.domain()
    }

    /// 需求身份的稳定字符串键 / Stable string key of the demand identity
    ///
    /// 供影子价格索引与 KPI 输出共用，避免各处自行拼接。
    /// Shared by shadow-price indexing and KPI output so no caller assembles it independently.
    pub fn stable_key(&self) -> String {
        match &self.key {
            Bpp2dDemandKey::Item { id } => format!("item:{:?}:{}", self.mode, id),
            Bpp2dDemandKey::Material { no } => format!("material:{:?}:{}", self.mode, no),
        }
    }
}

// ============================================================================
// 需求聚合 / Demand aggregation
// ============================================================================

/// 按需求键聚合件数 / Aggregate piece counts by demand key
pub fn aggregate_item_amounts(
    entries: impl IntoIterator<Item = Bpp2dDemandEntry>,
) -> std::collections::BTreeMap<Bpp2dDemandKey, u64> {
    let mut totals: std::collections::BTreeMap<Bpp2dDemandKey, u64> =
        std::collections::BTreeMap::new();
    for entry in entries {
        if let Bpp2dDemandValue::Amount(amount) = entry.value {
            *totals.entry(entry.key).or_insert(0) += amount;
        }
    }
    totals
}
