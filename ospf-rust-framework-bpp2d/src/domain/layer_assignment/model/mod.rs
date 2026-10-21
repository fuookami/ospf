//! 赋值与负载模型声明 / Assignment and load model declarations
//!
//! 层分配是 bpp2d 唯一允许出现求解器数值类型的建模边界：领域其余部分泛型于 `V`，
//! 这里统一收到 `f64` 并集中做适配。
//! Layer assignment is the only modelling boundary in bpp2d where solver value types are allowed: the rest of
//! the domain is generic over `V`, while this layer normalises to `f64` and centralises the adaptation.

pub mod layer_aggregation;
pub mod load;
pub mod objectives;
pub mod reduced_cost;

pub use layer_aggregation::LayerAggregation;
pub use load::{
    Bpp2dAssignmentDemandEntry, Bpp2dAssignmentDemandMode, Bpp2dAssignmentDemandValue, Load,
    LoadPrecision, layer_demand_coefficient,
};
pub use objectives::{
    LayerAssignmentObjectiveConfig, ObjectiveTerm, layer_waste_area, layer_waste_ratio,
    sheet_amount_terms, sheet_area_terms, sheet_type_cost_terms,
};
pub use reduced_cost::{
    Bpp2dDemandShadowPriceKey, Bpp2dDemandShadowPriceMap, filter_improving_columns,
    is_improving_column, layer_reduced_cost, sort_by_reduced_cost,
};

// ============================================================================
// 每列份数上界 / Per-column copy upper bound
// ============================================================================

/// 计算每列份数的上界 / Compute the per-column copy upper bound
///
/// 单列重复次数不会超过「满足全部需求所需的最少重复次数」：取需求上限除以该列覆盖件数后向上取整。
/// 上限等于 0 或列内无件数时取 1，避免出现零上界把列锁死。
/// A single column cannot repeat more often than the repetitions needed to satisfy the whole demand: the
/// demand cap divided by that column's covered pieces, rounded up. The bound falls back to 1 when it is zero
/// or the column holds no piece, so a zero bound never locks a column out.
pub fn max_copies_per_column<V, U>(
    layers: &[crate::domain::item::model::Layer<V, U>],
    demand_cap: u64,
) -> Vec<u64>
where
    V: ospf_rust_math::algebra::concept::Field + Clone + PartialOrd + num_traits::Float,
    U: ospf_rust_quantities::unit::concept::UnitTrait
        + ospf_rust_quantities::unit::physical_unit::CTUnit
        + Clone,
{
    layers
        .iter()
        .map(|layer| {
            // 空列不贡献任何覆盖，份数上界必须为 1：用「需求 / 1」会给出与需求等量的上界，
            // 让一个不装货的列在模型里反复出现。
            // An empty column contributes no coverage, so its copy bound must be 1: dividing by a padded
            // "1 covered piece" would give a demand-sized bound and let a column carrying nothing repeat.
            if layer.units.is_empty() {
                return 1;
            }
            let covered = layer.units.len() as u64;
            let bound = demand_cap.div_ceil(covered);
            bound.max(1)
        })
        .collect()
}

/// 需求上限：所有需求条目中最大的件数或面积 / The demand cap: the largest amount or area over all entries
pub fn demand_cap(entries: &[Bpp2dAssignmentDemandEntry]) -> u64 {
    entries
        .iter()
        .map(|entry| match entry.demand {
            Bpp2dAssignmentDemandValue::Amount(amount) => amount,
            Bpp2dAssignmentDemandValue::Area(area) => area.max(0.0).ceil() as u64,
        })
        .max()
        .unwrap_or(0)
}
