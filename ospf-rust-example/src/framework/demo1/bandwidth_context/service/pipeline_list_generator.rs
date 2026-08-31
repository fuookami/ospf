//! 带宽管道列表生成器模块 / Bandwidth pipeline list generator module

use super::super::aggregation::Aggregation;
use super::limits::{
    apply_bandwidth_cost_objective, apply_demand_constraints, apply_edge_bandwidth_constraints,
    apply_service_capacity_constraints, apply_transfer_node_bandwidth_constraints,
};
use crate::framework::demo1::route_context::model::{Edge, Node, Service};
use ospf_rust_core::model::MetaModel;
use ospf_rust_multiarray::{MultiArray, Shape};
use std::error::Error;

/// 带宽管道列表生成器 / Bandwidth pipeline list generator
///
/// 对齐 Kotlin bandwidth_context/service/PipelineListGenerator
///
/// 从符号组合取多项式构建约束，y_idx 仅用于 var_index 到 service 的反查。
/// Builds constraints from symbol combination polynomials; y_idx is only used for var_index to service reverse lookup.
pub fn generate_pipelines(
    aggregation: &Aggregation,
    model: &mut MetaModel<f64>,
    edges: &[Edge],
    services: &[Service],
    nodes: &[Node],
    normal_node_indices: &[usize],
    node_assignment: &ospf_rust_core::symbol::SymbolCombination<
        f64,
        ospf_rust_core::symbol::LinearExpressionSymbol<f64>,
        Shape<1>,
    >,
    service_assignment: &ospf_rust_core::symbol::SymbolCombination<
        f64,
        ospf_rust_core::symbol::LinearExpressionSymbol<f64>,
        Shape<1>,
    >,
    x_idx: &MultiArray<usize, Shape<2>>,
) -> Result<(), Box<dyn Error>> {
    let bandwidth = &aggregation.edge_bandwidth.bandwidth;
    let y_idx = &aggregation.edge_bandwidth.y_idx;

    apply_edge_bandwidth_constraints(
        model,
        edges,
        services,
        nodes,
        bandwidth,
        service_assignment,
        y_idx,
    )?;
    apply_demand_constraints(model, nodes, edges, services, bandwidth)?;
    apply_service_capacity_constraints(
        model,
        nodes,
        edges,
        services,
        bandwidth,
        node_assignment,
        y_idx,
        normal_node_indices,
    )?;
    apply_transfer_node_bandwidth_constraints(
        model,
        nodes,
        edges,
        services,
        bandwidth,
        node_assignment,
        y_idx,
        normal_node_indices,
    )?;
    apply_bandwidth_cost_objective(model, edges, nodes, services, bandwidth)?;

    Ok(())
}
