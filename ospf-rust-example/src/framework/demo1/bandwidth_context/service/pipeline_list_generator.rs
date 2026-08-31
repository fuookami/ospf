use std::error::Error;
use ospf_rust_core::model::MetaModel;
use super::super::aggregation::Aggregation;
use crate::framework::demo1::route_context::model::{Edge, Node, Service};
use super::limits::{
    apply_bandwidth_cost_objective, apply_demand_constraints,
    apply_edge_bandwidth_constraints, apply_service_capacity_constraints,
    apply_transfer_node_bandwidth_constraints,
};

/// 带宽管道列表生成器 / Bandwidth pipeline list generator
/// 对齐 Kotlin bandwidth_context/service/PipelineListGenerator
pub fn generate_pipelines(
    aggregation: &Aggregation,
    model: &mut MetaModel<f64>,
    edges: &[Edge],
    services: &[Service],
    nodes: &[Node],
    normal_node_indices: &[usize],
    x_idx: &[Vec<usize>],
) -> Result<(), Box<dyn Error>> {
    let y_idx = &aggregation.edge_bandwidth.y_idx;

    apply_edge_bandwidth_constraints(model, edges, services, nodes, y_idx, x_idx, normal_node_indices)?;
    apply_demand_constraints(model, nodes, edges, services, y_idx)?;
    apply_service_capacity_constraints(model, nodes, edges, services, y_idx, x_idx, normal_node_indices)?;
    apply_transfer_node_bandwidth_constraints(model, nodes, edges, services, y_idx, x_idx, normal_node_indices)?;
    apply_bandwidth_cost_objective(model, edges, nodes, services, y_idx)?;

    Ok(())
}
