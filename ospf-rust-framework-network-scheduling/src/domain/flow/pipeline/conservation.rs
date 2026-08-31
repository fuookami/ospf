//! 流守恒管道 / Flow conservation pipeline.

use std::collections::BTreeMap;

use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use ospf_rust_quantities::unit::UnitConversionValue;

use super::super::{FlowAggregation, FlowGraph};
use crate::error::{NetworkSchedulingError, Result};

/// 在每个 `(commodity, node)` 上注册流守恒等式 / Register flow-conservation equalities for each `(commodity, node)`.
#[derive(Debug, Clone)]
pub struct FlowConservationPipeline<V: SolveValue + UnitConversionValue> {
    /// flow 图 / Flow graph.
    pub graph: FlowGraph<V>,
    /// 管道名称 / Pipeline name.
    pub name: String,
}

impl<V> FlowConservationPipeline<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建守恒管道 / Create a conservation pipeline.
    pub fn new(graph: FlowGraph<V>) -> Self {
        Self {
            graph,
            name: "flow_conservation".to_owned(),
        }
    }

    /// 应用管道 / Apply the pipeline.
    pub fn apply(
        &self,
        model: &mut MetaModel<f64>,
        aggregation: &FlowAggregation<V>,
    ) -> Result<()> {
        for commodity in &self.graph.commodities {
            for node in &self.graph.nodes {
                let mut coefficients = BTreeMap::<usize, f64>::new();
                for arc in &self.graph.arcs {
                    let index = aggregation
                        .variable_index(commodity.commodity.as_str(), &arc.id)
                        .ok_or_else(|| {
                            NetworkSchedulingError::model(format!(
                                "缺少流变量 {} / missing flow variable {}",
                                arc.id, arc.id
                            ))
                        })?;
                    if arc.from == node.id {
                        *coefficients.entry(index).or_default() += 1.0;
                    }
                    if arc.to == node.id {
                        *coefficients.entry(index).or_default() -= 1.0;
                    }
                }
                let balance = commodity
                    .balances
                    .get(&node.id)
                    .map(|value| {
                        value
                            .value
                            .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                    })
                    .transpose()
                    .map_err(|error| NetworkSchedulingError::Conversion {
                        message: error.to_string(),
                    })?
                    .unwrap_or(0.0);
                let coefficients = coefficients
                    .into_iter()
                    .filter(|(_, coefficient)| coefficient.abs() > f64::EPSILON)
                    .collect::<Vec<_>>();
                model
                    .add_linear_constraint(
                        &coefficients,
                        ConstraintRelation::Equal,
                        balance,
                        &format!("{}_{}_{}", self.name, commodity.commodity, node.id),
                    )
                    .map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
            }
        }
        Ok(())
    }
}
