//! 最小费用流目标管道 / Min-cost-flow objective pipeline.

use ospf_rust_core::model::{MetaModel, ObjectiveCategory};
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_quantities::unit::UnitConversionValue;

use super::super::{FlowAggregation, FlowProblem};
use crate::error::Result;

/// 注册 `sum(flow * arc_cost)` 最小化目标 / Register `sum(flow * arc_cost)` minimization.
#[derive(Debug, Clone)]
pub struct MinCostFlowObjectivePipeline<V: SolveValue + UnitConversionValue> {
    /// flow 图 / Flow graph.
    pub graph: FlowProblem<V>,
    /// 目标名称 / Objective name.
    pub name: String,
}

impl<V> MinCostFlowObjectivePipeline<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 创建目标管道 / Create an objective pipeline.
    pub fn new(graph: FlowProblem<V>) -> Self {
        Self {
            graph,
            name: "min_cost_flow".to_owned(),
        }
    }

    /// 应用管道 / Apply the pipeline.
    pub fn apply(
        &self,
        model: &mut MetaModel<f64>,
        aggregation: &FlowAggregation<V>,
    ) -> Result<()> {
        let coefficients = aggregation.cost_expression()?;
        model.set_objective_category(ObjectiveCategory::Minimum);
        model.add_linear_objective(&coefficients, &self.name);
        Ok(())
    }
}
