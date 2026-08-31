use std::collections::BTreeMap;
use crate::framework_demo::demo2::domain::loading_effectiveness::context::LoadingEffectivenessContext;

pub struct LoadingEffectivenessAggregation {
    pub big_m: f64,
    pub early_end: usize,
    pub cargos_by_source: BTreeMap<String, Vec<usize>>,
}

impl LoadingEffectivenessAggregation {
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
