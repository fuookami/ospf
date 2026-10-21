//! 层分配需求与负载 / Layer assignment demand and load
//!
//! `demand` 是必须被**覆盖**的下界（装箱语义：需求必须装完），`upper_bound` 为可选的额外上界
//! （业务上限）。若把需求只当作上界，最小化目标会退化为「一块板材都不用」。
//! `demand` is a lower bound that must be **covered** (packing semantics: the demand has to be shipped) and
//! `upper_bound` is an optional extra upper bound (a business cap). Treating the demand as an upper bound
//! only would let the minimisation degenerate to "use no sheet at all".

use std::collections::BTreeMap;

use crate::domain::item::model::{ItemId, Layer};

// ============================================================================
// Bpp2dAssignmentDemandMode - 需求种类 / Demand kind
// ============================================================================

/// 层分配需求种类 / Layer assignment demand kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Bpp2dAssignmentDemandMode {
    /// 物品件数 / Item amount
    ItemAmount,
    /// 物品面积 / Item area
    ItemArea,
}

impl Bpp2dAssignmentDemandMode {
    /// 约束命名短标签 / Short tag used in constraint names
    pub fn tag(&self) -> &'static str {
        match self {
            Self::ItemAmount => "item_amount",
            Self::ItemArea => "item_area",
        }
    }
}

// ============================================================================
// Bpp2dAssignmentDemandValue - 需求量值 / Demand value
// ============================================================================

/// 层分配需求量值 / Layer assignment demand value
#[derive(Debug, Clone, PartialEq)]
pub enum Bpp2dAssignmentDemandValue {
    /// 件数 / Piece count
    Amount(u64),
    /// 面积 / Area
    Area(f64),
}

impl Bpp2dAssignmentDemandValue {
    /// 求解器数值 / The solver number
    ///
    /// 件数直接转为 `f64`，面积已经是求解器数值。
    /// A piece count converts directly to `f64`; an area already is a solver number.
    pub fn to_solver(&self) -> f64 {
        match self {
            Self::Amount(value) => *value as f64,
            Self::Area(value) => *value,
        }
    }

    /// 是否为正 / Whether strictly positive
    pub fn is_positive(&self) -> bool {
        self.to_solver() > 0.0
    }
}

// ============================================================================
// Bpp2dAssignmentDemandEntry - 层分配需求项 / Layer assignment demand entry
// ============================================================================

/// 层分配需求项 / Layer assignment demand entry
#[derive(Debug, Clone, PartialEq)]
pub struct Bpp2dAssignmentDemandEntry {
    /// 需求种类 / Demand kind
    pub mode: Bpp2dAssignmentDemandMode,
    /// 物品编号 / Item identifier
    pub item_id: ItemId,
    /// 必须覆盖的需求下界 / Demand lower bound to cover
    pub demand: Bpp2dAssignmentDemandValue,
    /// 可选的需求上界 / Optional demand upper bound
    pub upper_bound: Option<Bpp2dAssignmentDemandValue>,
}

impl Bpp2dAssignmentDemandEntry {
    /// 创建需求项 / Create a demand entry
    pub fn new(
        mode: Bpp2dAssignmentDemandMode,
        item_id: impl Into<String>,
        demand: Bpp2dAssignmentDemandValue,
    ) -> Self {
        Self {
            mode,
            item_id: ItemId::new(item_id),
            demand,
            upper_bound: None,
        }
    }

    /// 件数需求 / An amount demand
    pub fn item_amount(item_id: impl Into<String>, amount: u64) -> Self {
        Self::new(
            Bpp2dAssignmentDemandMode::ItemAmount,
            item_id,
            Bpp2dAssignmentDemandValue::Amount(amount),
        )
    }

    /// 面积需求 / An area demand
    pub fn item_area(item_id: impl Into<String>, area: f64) -> Self {
        Self::new(
            Bpp2dAssignmentDemandMode::ItemArea,
            item_id,
            Bpp2dAssignmentDemandValue::Area(area),
        )
    }

    /// 设置业务上界 / Set the business upper bound
    pub fn with_upper_bound(mut self, upper_bound: Bpp2dAssignmentDemandValue) -> Self {
        self.upper_bound = Some(upper_bound);
        self
    }

    /// 约束名称 / Constraint name
    ///
    /// 形如 `demand_item_amount_<item>`，供影子价格键与诊断共用。
    /// Shaped like `demand_item_amount_<item>`, shared by the shadow-price key and diagnostics.
    pub fn constraint_name(&self) -> String {
        format!("demand_{}_{}", self.mode.tag(), self.item_id)
    }
}

// ============================================================================
// 需求系数 / Demand coefficients
// ============================================================================

/// 计算候选层对某个需求的贡献系数 / Compute the contribution coefficient of a layer for a demand
///
/// 件数需求取层内件数；面积需求取该物品的单件面积乘以件数。层内不含该物品时系数为 0。
/// An amount demand takes the piece count inside the layer; an area demand takes the item's per-piece area
/// times that count. The coefficient is 0 when the layer holds none of the item.
pub fn layer_demand_coefficient<V, U>(
    layer: &Layer<V, U>,
    entry: &Bpp2dAssignmentDemandEntry,
) -> f64
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: ospf_rust_quantities::unit::concept::UnitTrait
        + ospf_rust_quantities::unit::physical_unit::CTUnit
        + Clone,
{
    let amounts: BTreeMap<ItemId, u64> = layer.item_amounts();
    let Some(amount) = amounts.get(&entry.item_id).copied() else {
        return 0.0;
    };
    match entry.mode {
        Bpp2dAssignmentDemandMode::ItemAmount => amount as f64,
        Bpp2dAssignmentDemandMode::ItemArea => {
            let unit_area = layer
                .units
                .iter()
                .find(|placement| {
                    placement
                        .item_id
                        .as_deref()
                        .map(|id| id == entry.item_id.as_str())
                        .unwrap_or(false)
                })
                .and_then(|placement| placement.shape.area().value.to_f64())
                .unwrap_or(0.0);
            unit_area * amount as f64
        }
    }
}

// ============================================================================
// 负载 / Load
// ============================================================================

/// 负载：需求覆盖的累计量 / Load: the accumulated demand coverage
///
/// `imprecise` 用于 RMP 阶段（列份数连续），`precise` 用于最终 MILP（列份数整数）。
/// `imprecise` serves the RMP phase (continuous column copies) while `precise` serves the final MILP
/// (integer column copies).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadPrecision {
    /// RMP 松弛量 / RMP relaxation
    Imprecise,
    /// 最终整数份数 / Final integer copies
    Precise,
}

/// 负载 / Load
#[derive(Debug, Clone)]
pub struct Load {
    /// 需求项列表 / Demand entry list
    pub demand_entries: Vec<Bpp2dAssignmentDemandEntry>,
    /// 是否启用超载松弛 / Whether the over-load slack is enabled
    pub over_enabled: bool,
    /// 是否启用欠载松弛 / Whether the less-load slack is enabled
    pub less_enabled: bool,
    /// 精度 / Precision
    pub precision: LoadPrecision,
}

impl Load {
    /// 创建 RMP 负载 / Create an RMP load
    pub fn imprecise(demand_entries: Vec<Bpp2dAssignmentDemandEntry>) -> Self {
        Self {
            demand_entries,
            over_enabled: false,
            less_enabled: true,
            precision: LoadPrecision::Imprecise,
        }
    }

    /// 创建最终 MILP 负载 / Create a final MILP load
    pub fn precise(demand_entries: Vec<Bpp2dAssignmentDemandEntry>) -> Self {
        Self {
            demand_entries,
            over_enabled: false,
            less_enabled: true,
            precision: LoadPrecision::Precise,
        }
    }

    /// 启用超载松弛 / Enable the over-load slack
    pub fn with_over(mut self, over_enabled: bool) -> Self {
        self.over_enabled = over_enabled;
        self
    }

    /// 逐层累计各需求的覆盖量 / Accumulate the coverage of every demand across layers
    ///
    /// 返回与 `demand_entries` 同序的覆盖量；`copies[i]` 是第 i 个层的份数。
    /// Returns coverage in the same order as `demand_entries`; `copies[i]` is the copy count of layer i.
    pub fn coverage<V, U>(&self, layers: &[Layer<V, U>], copies: &[f64]) -> Vec<f64>
    where
        V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
        U: ospf_rust_quantities::unit::concept::UnitTrait
            + ospf_rust_quantities::unit::physical_unit::CTUnit
            + Clone,
    {
        self.demand_entries
            .iter()
            .map(|entry| {
                layers
                    .iter()
                    .zip(copies.iter())
                    .map(|(layer, copies)| layer_demand_coefficient(layer, entry) * copies)
                    .sum()
            })
            .collect()
    }
}
