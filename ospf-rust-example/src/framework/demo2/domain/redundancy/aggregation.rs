//! 冗余聚合 / Redundancy aggregation
use std::collections::BTreeMap;
use crate::framework::demo2::domain::redundancy::context::RedundancyContext;

/// 冗余聚合 / Redundancy aggregation
///
/// 按目的地分组货物索引，用于目的地分散约束。
/// Groups cargo indices by destination for destination spread constraints.
pub struct RedundancyAggregation {
    /// 按目的地分组的货物索引 / Cargo indices grouped by destination
    pub cargos_by_destination: BTreeMap<String, Vec<usize>>,
}

impl RedundancyAggregation {
    /// 从冗余上下文创建聚合 / Create aggregation from redundancy context
    pub fn from_context(context: &RedundancyContext<'_>) -> Self {
        let mut cargos_by_destination: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for c in 0..context.request.cargos.len() {
            cargos_by_destination
                .entry(context.request.cargos[c].destination.clone())
                .or_default()
                .push(c);
        }
        Self {
            cargos_by_destination,
        }
    }
}
