//! 硬守恒与费用的单一职责管道。 / Single-purpose hard-conservation and cost pipelines.

use ospf_rust_core::model::{ConstraintRelation, MetaModel, ObjectiveCategory};
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_quantities::unit::UnitConversionValue;
use crate::error::{NetworkSchedulingError, Result};
use super::aggregation::GeneralizedFlowAggregation;

pub(super) fn register_balances<V>(
    model: &mut MetaModel<f64>,
    aggregation: &GeneralizedFlowAggregation<V>,
    name: &str,
) -> Result<()>
where
    V: SolveValue + UnitConversionValue,
{
    for (node, supply) in &aggregation.prepared.supplies {
        model.add_linear_constraint(
            &aggregation.expressions().net_supplies[node],
            ConstraintRelation::Equal,
            *supply,
            &format!("{name}_balance_{node}"),
        ).map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
    }
    Ok(())
}

pub(super) fn register_cost<V>(
    model: &mut MetaModel<f64>,
    aggregation: &GeneralizedFlowAggregation<V>,
    name: &str,
) where
    V: SolveValue + UnitConversionValue,
{
    model.set_objective_category(ObjectiveCategory::Minimum);
    model.add_linear_objective(&aggregation.expressions().cost, &format!("{name}_cost"));
}
