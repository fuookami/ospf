use crate::framework::demo2::domain::express_effectiveness::context::ExpressEffectivenessContext;

pub struct ExpressEffectivenessAggregation {
    pub must_ship_indices: Vec<usize>,
}

impl ExpressEffectivenessAggregation {
    pub fn from_context(context: &ExpressEffectivenessContext<'_>) -> Self {
        let must_ship_indices = (0..context.request.cargos.len())
            .filter(|c| context.request.cargos[*c].priority >= 8)
            .collect();
        Self { must_ship_indices }
    }
}

