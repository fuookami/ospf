//! 快递效能聚合 / Express effectiveness aggregation
use crate::framework::demo2::domain::express_effectiveness::context::ExpressEffectivenessContext;

/// 快递效能聚合数据 / Express effectiveness aggregation data
pub struct ExpressEffectivenessAggregation {
    /// 必须装载的货物索引列表 / Indices of must-ship cargos
    pub must_ship_indices: Vec<usize>,
}

impl ExpressEffectivenessAggregation {
    /// 从上下文构建聚合数据 / Build aggregation data from context
    pub fn from_context(context: &ExpressEffectivenessContext<'_>) -> Self {
        let must_ship_indices = (0..context.request.cargos.len())
            .filter(|c| context.request.cargos[*c].priority >= 8)
            .collect();
        Self { must_ship_indices }
    }
}
