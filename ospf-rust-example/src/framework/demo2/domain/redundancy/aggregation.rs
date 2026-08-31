use std::collections::BTreeMap;
use crate::framework_demo::demo2::domain::redundancy::context::RedundancyContext;

pub struct RedundancyAggregation {
    pub cargos_by_destination: BTreeMap<String, Vec<usize>>,
}

impl RedundancyAggregation {
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
