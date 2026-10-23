//! 容量边界管道 / Capacity-bounds pipeline.

use ospf_rust_core::model::{ConstraintRelation, MetaModel};
use ospf_rust_core::solver::value::{SolveValue, SolveValueConversionPolicy};
use ospf_rust_quantities::unit::UnitConversionValue;

use super::super::{FlowAggregation, FlowProblem};
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::NetworkSchedulingSolverValueAdapter;

/// 注册单弧跨商品共享容量上下界 / Register shared cross-commodity arc capacity bounds.
#[derive(Debug, Clone)]
pub struct CapacityBoundsPipeline<V: SolveValue + UnitConversionValue> {
    /// flow 图 / Flow graph.
    pub graph: FlowProblem<V>,
    /// 管道名称 / Pipeline name.
    pub name: String,
}

impl<V> CapacityBoundsPipeline<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建容量管道 / Create a capacity pipeline.
    pub fn new(graph: FlowProblem<V>) -> Self {
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
        let adapter = NetworkSchedulingSolverValueAdapter::<V>::default();
        for arc in &self.graph.arcs {
            let coefficients = aggregation.capacity_expression(&arc.id)?;
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
        for resource in self.graph.resources() {
            let coefficients = aggregation.resource_expression(&resource.id)?;
            let lower = adapter.to_solver(&resource.capacity.lower.value)?;
            let upper = adapter.to_solver(&resource.capacity.upper.value)?;
            for (relation, bound, suffix) in [
                (ConstraintRelation::GreaterEqual, lower, "lower"),
                (ConstraintRelation::LessEqual, upper, "upper"),
            ] {
                model.add_linear_constraint(
                    &coefficients, relation, bound, &format!("{}_resource_{}_{}", self.name, resource.id, suffix),
                ).map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
            }
        }
        Ok(())
    }
}
