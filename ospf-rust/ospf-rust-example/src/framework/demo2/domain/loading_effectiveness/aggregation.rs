//! 装卸效能聚合 / Loading effectiveness aggregation
use crate::framework::demo2::domain::loading_effectiveness::context::LoadingEffectivenessContext;
use std::collections::BTreeMap;

/// 装载效能聚合数据 / Loading effectiveness aggregation data
pub struct LoadingEffectivenessAggregation {
    /// 大M常数，用于线性化约束 / Big-M constant for linearization
    pub big_m: f64,
    /// 早期结束位置索引 / Early end position index
    pub early_end: usize,
    /// 每个显式顺序作用域的早期位置，包含同序位置。 / Early positions within each explicit ordering scope, including equal-order ties.
    pub early_positions: Vec<usize>,
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

        let mut scopes = BTreeMap::<&str, Vec<(i32, usize)>>::new();
        for (p, position) in context.request.positions.iter().enumerate() {
            let input = &context.request.loading.positions[&position.name];
            scopes.entry(&input.loading_order_scope).or_default().push((input.loading_order, p));
        }
        let mut early_positions = Vec::new();
        for positions in scopes.values_mut() {
            positions.sort_unstable();
            let threshold = positions[(positions.len() - 1) / 2].0;
            early_positions.extend(positions.iter().filter(|(order, _)| *order <= threshold).map(|(_, p)| *p));
        }
        Self {
            big_m: context.request.positions.len() as f64,
            early_end: context.request.positions.len().saturating_sub(1) / 2,
            early_positions,
            cargos_by_source,
        }
    }
}
