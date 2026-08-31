//! 容量边界管道 / Capacity-bounds pipeline.

use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use ospf_rust_quantities::unit::UnitConversionValue;

use super::super::{FlowAggregation, FlowGraph};
use crate::error::{NetworkSchedulingError, Result};

/// 注册单弧跨商品共享容量上下界 / Register shared cross-commodity arc capacity bounds.
#[derive(Debug, Clone)]
pub struct CapacityBoundsPipeline<V: SolveValue + UnitConversionValue> {
    /// flow 图 / Flow graph.
    pub graph: FlowGraph<V>,
    /// 管道名称 / Pipeline name.
    pub name: String,
}

impl<V> CapacityBoundsPipeline<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建容量管道 / Create a capacity pipeline.
    pub fn new(graph: FlowGraph<V>) -> Self {
        Self {
            graph,
            name: "flow_capacity".to_owned(),
        }
    }

    /// 应用管道 / Apply the pipeline.
    pub fn apply(
        &self,
        model: &mut MetaModel<f64>,
        aggregation: &FlowAggregation<V>,
    ) -> Result<()> {
        for arc in &self.graph.arcs {
            let coefficients = self
                .graph
                .commodities
                .iter()
                .map(|commodity| {
                    aggregation
                        .variable_index(commodity.commodity.as_str(), &arc.id)
                        .map(|index| (index, 1.0))
                        .ok_or_else(|| {
                            NetworkSchedulingError::model(format!(
                                "缺少流变量 {} / missing flow variable {}",
                                arc.id, arc.id
                            ))
                        })
                })
                .collect::<Result<Vec<_>>>()?;
            let lower = arc
                .capacity
                .lower
                .value
                .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
            let upper = arc
                .capacity
                .upper
                .value
                .to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                .map_err(|error| NetworkSchedulingError::Conversion {
                    message: error.to_string(),
                })?;
            model
                .add_linear_constraint(
                    &coefficients,
                    ConstraintRelation::GreaterEqual,
                    lower,
                    &format!("{}_{}_lower", self.name, arc.id),
                )
                .map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
            model
                .add_linear_constraint(
                    &coefficients,
                    ConstraintRelation::LessEqual,
                    upper,
                    &format!("{}_{}_upper", self.name, arc.id),
                )
                .map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
        }
        Ok(())
    }
}
