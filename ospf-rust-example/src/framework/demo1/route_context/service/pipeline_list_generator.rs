//! 管道列表生成器 / Pipeline list generator

use super::super::aggregation::Aggregation;
use super::limits::{
    apply_node_assignment_constraints, apply_service_assignment_constraints,
    apply_service_cost_objective,
};
use ospf_rust_core::model::MetaModel;
use std::error::Error;

/// 路线管道列表生成器 / Route pipeline list generator
/// 对齐 Kotlin route_context/service/PipelineListGenerator
pub fn generate_pipelines(
    aggregation: &Aggregation,
    model: &mut MetaModel<f64>,
) -> Result<(), Box<dyn Error>> {
    apply_node_assignment_constraints(
        model,
        &aggregation.assignment,
        aggregation.graph.nodes.len(),
    )?;
    apply_service_assignment_constraints(
        model,
        &aggregation.assignment,
        aggregation.services.len(),
    )?;
    apply_service_cost_objective(model, &aggregation.assignment, &aggregation.services)?;
    Ok(())
}
