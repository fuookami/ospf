//! 检验数定价 / Reduced-cost pricing
//!
//! 列生成的定价步骤：按当前 RMP 的覆盖约束对偶（影子价格）计算候选列的检验数，只把能改进目标的列加入模型。
//! The pricing step of column generation: computes the reduced cost of a candidate column from the current RMP
//! coverage duals (shadow prices) and keeps only columns that can improve the objective.
//!
//! 模型为 `min Σ_j area_j · y_j  s.t.  Σ_j a_ij · y_j ≥ d_i`（`a_ij` 为列 j 对需求 i 的覆盖量，
//! `d_i ≥ 0`，对偶 `π_i ≥ 0`），因此列 j 的检验数为
//! `rc_j = area_j · c_area − Σ_i π_i · a_ij`，`rc_j < 0` 表示该列可改进目标。
//! For `min Σ_j area_j · y_j  s.t.  Σ_j a_ij · y_j ≥ d_i` (`a_ij` is the coverage of demand i by column j,
//! `d_i ≥ 0`, duals `π_i ≥ 0`), the reduced cost of column j is
//! `rc_j = area_j · c_area − Σ_i π_i · a_ij`; `rc_j < 0` means the column can improve the objective.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::domain::item::model::Layer;
use crate::domain::layer_assignment::model::load::{
    Bpp2dAssignmentDemandEntry, layer_demand_coefficient,
};

// ============================================================================
// 影子价格键 / Shadow price key
// ============================================================================

/// 需求影子价格键：需求种类与物品编号 / Demand shadow price key: kind and item
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bpp2dDemandShadowPriceKey {
    /// 需求种类 / Demand kind
    pub mode: crate::domain::layer_assignment::model::load::Bpp2dAssignmentDemandMode,
    /// 物品编号 / Item identifier
    pub item_id: String,
}

impl Bpp2dDemandShadowPriceKey {
    /// 创建影子价格键 / Create a shadow price key
    pub fn new(
        mode: crate::domain::layer_assignment::model::load::Bpp2dAssignmentDemandMode,
        item_id: impl Into<String>,
    ) -> Self {
        Self {
            mode,
            item_id: item_id.into(),
        }
    }
}

/// 需求影子价格映射 / Demand shadow price map
pub type Bpp2dDemandShadowPriceMap = BTreeMap<Bpp2dDemandShadowPriceKey, f64>;

// ============================================================================
// 检验数 / Reduced cost
// ============================================================================

/// 计算候选列的检验数 / Compute the reduced cost of a candidate column
///
/// 面积系数必须与 RMP 目标一致，否则检验数符号会与实际改进方向相反。
/// The area coefficient must match the RMP objective, otherwise the sign of the reduced cost would disagree
/// with the actual improvement direction.
pub fn layer_reduced_cost<V, U>(
    layer: &Layer<V, U>,
    demand_entries: &[Bpp2dAssignmentDemandEntry],
    shadow_prices: &Bpp2dDemandShadowPriceMap,
    area_coefficient: f64,
) -> f64
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: ospf_rust_quantities::unit::concept::UnitTrait
        + ospf_rust_quantities::unit::physical_unit::CTUnit
        + Clone,
{
    let mut dual_value = 0.0;
    for entry in demand_entries {
        let key = Bpp2dDemandShadowPriceKey::new(entry.mode, entry.item_id.as_str());
        let Some(price) = shadow_prices.get(&key).copied() else {
            continue;
        };
        if price == 0.0 {
            continue;
        }
        dual_value += price * layer_demand_coefficient(layer, entry);
    }
    let area = layer.area().value.to_f64().unwrap_or(0.0);
    area * area_coefficient - dual_value
}

/// 判断候选列是否为改进列 / Decide whether a candidate column is an improving column
///
/// 检验数必须小于 `-tolerance` 才算改进；用零容差会把数值噪声当成改进列。
/// The reduced cost must be below `-tolerance` to count as improving; a zero tolerance would treat numerical
/// noise as an improving column.
pub fn is_improving_column<V, U>(
    layer: &Layer<V, U>,
    demand_entries: &[Bpp2dAssignmentDemandEntry],
    shadow_prices: &Bpp2dDemandShadowPriceMap,
    area_coefficient: f64,
    tolerance: f64,
) -> bool
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: ospf_rust_quantities::unit::concept::UnitTrait
        + ospf_rust_quantities::unit::physical_unit::CTUnit
        + Clone,
{
    layer_reduced_cost(layer, demand_entries, shadow_prices, area_coefficient) < -tolerance
}

/// 按检验数升序排序候选列 / Sort candidate columns by ascending reduced cost
///
/// 最可能改进目标的列排在最前，便于按预算截断。
/// The columns most likely to improve the objective come first, so a budget truncation keeps them.
pub fn sort_by_reduced_cost<V, U>(
    layers: &mut [Layer<V, U>],
    demand_entries: &[Bpp2dAssignmentDemandEntry],
    shadow_prices: &Bpp2dDemandShadowPriceMap,
    area_coefficient: f64,
) where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: ospf_rust_quantities::unit::concept::UnitTrait
        + ospf_rust_quantities::unit::physical_unit::CTUnit
        + Clone,
{
    layers.sort_by(|lhs, rhs| {
        let left = layer_reduced_cost(lhs, demand_entries, shadow_prices, area_coefficient);
        let right = layer_reduced_cost(rhs, demand_entries, shadow_prices, area_coefficient);
        left.partial_cmp(&right).unwrap_or(Ordering::Equal)
    });
}

/// 只保留改进列 / Keep only improving columns
///
/// 结果按检验数升序排列。
/// The result is ordered by ascending reduced cost.
pub fn filter_improving_columns<V, U>(
    layers: Vec<Layer<V, U>>,
    demand_entries: &[Bpp2dAssignmentDemandEntry],
    shadow_prices: &Bpp2dDemandShadowPriceMap,
    area_coefficient: f64,
    tolerance: f64,
) -> Vec<Layer<V, U>>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: ospf_rust_quantities::unit::concept::UnitTrait
        + ospf_rust_quantities::unit::physical_unit::CTUnit
        + Clone,
{
    let mut improving: Vec<Layer<V, U>> = layers
        .into_iter()
        .filter(|layer| {
            is_improving_column(layer, demand_entries, shadow_prices, area_coefficient, tolerance)
        })
        .collect();
    sort_by_reduced_cost(
        &mut improving,
        demand_entries,
        shadow_prices,
        area_coefficient,
    );
    improving
}
