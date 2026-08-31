//! 装卸效能聚合 / Loading effectiveness aggregation
use std::collections::BTreeMap;
use crate::framework::demo2::domain::loading_effectiveness::context::LoadingEffectivenessContext;

/// 装载效能聚合数据 / Loading effectiveness aggregation data
pub struct LoadingEffectivenessAggregation {
    /// 大M常数，用于线性化约束 / Big-M constant for linearization
    pub big_m: f64,
    /// 早期结束位置索引 / Early end position index
    pub early_end: usize,
    /// 按来源分组的货物索引 / Cargo indices grouped by source
    pub cargos_by_source: BTreeMap<String, Vec<usize>>,
}

impl LoadingEffectivenessAggregation {
    /// 从上下文构建聚合数据 / Build aggregation data from context
    pub fn from_context(context: &LoadingEffectivenessContext<'_>) -> Self {
        let mut cargos_by_source: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for c in 0..context.request.cargos.len() {
            cargos_by_source
                .entry(context.request.cargos[c].source.clone())
                .or_default()
                .push(c);
        }

        Self {
            big_m: context.request.positions.len() as f64,
            early_end: context.request.positions.len().saturating_sub(1) / 2,
            cargos_by_source,
        }
    }
}
