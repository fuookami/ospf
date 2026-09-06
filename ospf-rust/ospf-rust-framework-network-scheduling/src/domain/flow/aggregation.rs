//! flow 聚合 / Flow aggregation.

use std::collections::BTreeMap;

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use ospf_rust_quantities::unit::UnitConversionValue;

use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{FlowIndex, NetworkArcId};

use super::model::FlowGraph;

/// flow 领域聚合，持有 `(commodity, arc)` 到模型变量的稳定索引 / Flow aggregation holding stable `(commodity, arc)` model-variable indices.
#[derive(Debug, Clone)]
pub struct FlowAggregation<V: SolveValue + UnitConversionValue> {
    /// 不可变 flow 图 / Immutable flow graph.
    pub graph: FlowGraph<V>,
    variable_indices: BTreeMap<(String, NetworkArcId), usize>,
}

impl<V> FlowAggregation<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建聚合 / Create an aggregation.
    pub fn new(graph: FlowGraph<V>) -> Self {
        Self {
            graph,
            variable_indices: BTreeMap::new(),
        }
    }

    /// 注册非负流变量 / Register non-negative flow variables.
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> Result<()> {
        for commodity in &self.graph.commodities {
            for arc in &self.graph.arcs {
                let upper = arc
                    .capacity
                    .upper
                    .value
                    .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?;
                let index = model
                    .register_auto_variable_with_range::<ospf_rust_core::variable::Continuous>(
                        &format!("flow_{}_{}", commodity.commodity, arc.id),
                        ospf_rust_core::variable::VariableRange::bounded(0.0, upper),
                    )
                    .map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
                self.variable_indices.insert(
                    (commodity.commodity.as_str().to_owned(), arc.id.clone()),
                    index,
                );
            }
        }
        Ok(())
    }

    /// 获取流变量索引 / Get a flow-variable index.
    pub fn variable_index(&self, commodity: &str, arc: &NetworkArcId) -> Option<usize> {
        self.variable_indices
            .get(&(commodity.to_owned(), arc.clone()))
            .copied()
    }

    /// 获取全部变量索引 / Get all flow-variable indices.
    pub fn variable_indices(&self) -> &BTreeMap<(String, NetworkArcId), usize> {
        &self.variable_indices
    }

    /// 创建 flow index / Create a typed flow index.
    pub fn flow_index(
        &self,
        commodity: &str,
        arc: &NetworkArcId,
    ) -> Option<FlowIndex<String, NetworkArcId>> {
        self.variable_index(commodity, arc)
            .map(|_| FlowIndex::new(commodity.to_owned(), arc.clone()))
    }
}
