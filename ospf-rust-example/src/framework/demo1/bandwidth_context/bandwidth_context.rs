use std::error::Error;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::symbol::{SymbolCombination, LinearExpressionSymbol};
use ospf_rust_core::symbol::flatten::{Linear as ModelLinear, LinearMonomial as ModelLinearMonomial};
use ospf_rust_core::symbol::next_auto_intermediate_symbol_id;
use ospf_rust_multiarray::Shape;
use crate::framework::demo1::route_context::RouteContext;
use super::aggregation::Aggregation;
use super::model::{EdgeBandwidth, NodeBandwidth, ServiceBandwidth};

pub struct BandwidthContext {
    pub aggregation: Option<Aggregation>,
}

impl BandwidthContext {
    pub fn new() -> Self {
        Self { aggregation: None }
    }

    pub fn register(
        &mut self,
        model: &mut MetaModel<f64>,
        route_context: &RouteContext,
    ) -> Result<(), Box<dyn Error>> {
        let route_agg = route_context
            .aggregation
            .as_ref()
            .ok_or("route context not initialized")?;

        let edges = &route_agg.graph.edges;
        let nodes = &route_agg.graph.nodes;
        let services = &route_agg.services;
        let normal_node_indices = &route_agg.assignment.normal_node_indices;
        let y_idx = &route_agg.assignment.x_idx; // placeholder, will use edge_bandwidth.y_idx

        // 1. 注册 EdgeBandwidth (y 变量 + bandwidth 符号)
        let mut edge_bandwidth = EdgeBandwidth::new();
        edge_bandwidth.register(model, edges, services, nodes)?;
        let y_idx = &edge_bandwidth.y_idx;

        // 2. 构建 ServiceBandwidth (入度/出度/出流 per node per service)
        let service_bandwidth = build_service_bandwidth(
            model, edges, nodes, services, normal_node_indices,
            &edge_bandwidth.bandwidth, y_idx,
        )?;

        // 3. 构建 NodeBandwidth (入度/出度/出流 per node)
        let node_bandwidth = build_node_bandwidth(
            model, services, normal_node_indices, &service_bandwidth,
        )?;

        self.aggregation = Some(Aggregation::new(
            edge_bandwidth,
            service_bandwidth,
            node_bandwidth,
        ));
        Ok(())
    }

    pub fn construct(
        &self,
        model: &mut MetaModel<f64>,
        route_context: &RouteContext,
    ) -> Result<(), Box<dyn Error>> {
        let agg = self
            .aggregation
            .as_ref()
            .ok_or("bandwidth context not registered")?;
        let route_agg = route_context
            .aggregation
            .as_ref()
            .ok_or("route context not initialized")?;

        super::service::generate_pipelines(
            agg,
            model,
            &route_agg.graph.edges,
            &route_agg.services,
            &route_agg.graph.nodes,
            &route_agg.assignment.normal_node_indices,
            &route_agg.assignment.node_assignment,
            &route_agg.assignment.service_assignment,
            &route_agg.assignment.x_idx,
        )
    }

    pub fn analyze(
        &self,
        route_context: &RouteContext,
        solution: &[f64],
    ) -> Result<Vec<Vec<u64>>, Box<dyn Error>> {
        let agg = self
            .aggregation
            .as_ref()
            .ok_or("bandwidth context not registered")?;
        let route_agg = route_context
            .aggregation
            .as_ref()
            .ok_or("route context not initialized")?;

        let analyzer = super::service::solution_analyzer::SolutionAnalyzer::new(
            &route_agg.graph,
            &route_agg.services,
            &route_agg.assignment,
            agg,
        );
        Ok(analyzer.analyze(solution))
    }
}

/// 构建 ServiceBandwidth 符号组合
///
/// 对每个 (node, service) 对，构建入度/出度/出流多项式：
/// - in_degree[node][s] = sum(y[e][s] for e where edge.to == node)
/// - out_degree[node][s] = sum(y[e][s] for e where edge.from == node)
/// - out_flow[node][s] = out_degree - in_degree
fn build_service_bandwidth(
    model: &mut MetaModel<f64>,
    edges: &[crate::framework::demo1::route_context::model::Edge],
    nodes: &[crate::framework::demo1::route_context::model::Node],
    services: &[crate::framework::demo1::route_context::model::Service],
    normal_node_indices: &[usize],
    bandwidth: &SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    y_idx: &ospf_rust_multiarray::MultiArray<usize, Shape<2>>,
) -> Result<ServiceBandwidth, Box<dyn Error>> {
    let node_count = normal_node_indices.len();
    let service_count = services.len();
    let shape = Shape::new([node_count, service_count]);

    // 构建 in_degree 多项式
    let in_degree = SymbolCombination::new(shape.clone(), "svc_bw_in", |index, vec| {
        let row = vec[0];
        let s = vec[1];
        let node_idx = normal_node_indices[row];
        let mut monomials = Vec::new();
        for (e, edge) in edges.iter().enumerate() {
            if edge.to == node_idx && !nodes[edge.from].is_client() {
                if let Some(coeff) = extract_service_coeff_from_bw(bandwidth, e, s, y_idx) {
                    monomials.push(ModelLinearMonomial::new(coeff, y_idx[&[e, s]]));
                }
            }
        }
        let poly = ModelLinear::new(monomials, 0.0);
        let id = next_auto_intermediate_symbol_id();
        LinearExpressionSymbol::new(
            id,
            &format!("svc_bw_in_{}_{}", node_idx, s),
            poly.monomials().to_vec(),
            *poly.constant_term(),
        )
    });
    model.add_symbol_combination(&in_degree)?;

    // 构建 out_degree 多项式
    let out_degree = SymbolCombination::new(shape.clone(), "svc_bw_out", |index, vec| {
        let row = vec[0];
        let s = vec[1];
        let node_idx = normal_node_indices[row];
        let mut monomials = Vec::new();
        for (e, edge) in edges.iter().enumerate() {
            if edge.from == node_idx && !nodes[edge.to].is_client() {
                if let Some(coeff) = extract_service_coeff_from_bw(bandwidth, e, s, y_idx) {
                    monomials.push(ModelLinearMonomial::new(coeff, y_idx[&[e, s]]));
                }
            }
        }
        let poly = ModelLinear::new(monomials, 0.0);
        let id = next_auto_intermediate_symbol_id();
        LinearExpressionSymbol::new(
            id,
            &format!("svc_bw_out_{}_{}", node_idx, s),
            poly.monomials().to_vec(),
            *poly.constant_term(),
        )
    });
    model.add_symbol_combination(&out_degree)?;

    // 构建 out_flow = out_degree - in_degree 多项式
    let out_flow = SymbolCombination::new(shape, "svc_bw_flow", |index, vec| {
        let row = vec[0];
        let s = vec[1];
        let node_idx = normal_node_indices[row];
        let out_poly = out_degree.symbol_polynomial(index);
        let in_poly = in_degree.symbol_polynomial(index);
        let mut monomials: Vec<ModelLinearMonomial<f64>> = out_poly.monomials().to_vec();
        // 减去 in_degree 的单项式（系数取反）
        for m in in_poly.monomials() {
            monomials.push(ModelLinearMonomial::new(-*m.coefficient(), m.var_index()));
        }
        let id = next_auto_intermediate_symbol_id();
        LinearExpressionSymbol::new(
            id,
            &format!("svc_bw_flow_{}_{}", node_idx, s),
            monomials,
            0.0,
        )
    });
    model.add_symbol_combination(&out_flow)?;

    Ok(ServiceBandwidth { in_degree, out_degree, out_flow })
}

/// 构建 NodeBandwidth 符号组合
///
/// 对每个 node，将 ServiceBandwidth 的 [node, *] 维度求和：
/// - in_degree[node] = sum(svc_bw.in_degree[node][s] for all s)
/// - out_degree[node] = sum(svc_bw.out_degree[node][s] for all s)
/// - out_flow[node] = sum(svc_bw.out_flow[node][s] for all s)
fn build_node_bandwidth(
    model: &mut MetaModel<f64>,
    services: &[crate::framework::demo1::route_context::model::Service],
    normal_node_indices: &[usize],
    svc_bw: &ServiceBandwidth,
) -> Result<NodeBandwidth, Box<dyn Error>> {
    let node_count = normal_node_indices.len();
    let service_count = services.len();
    let shape = Shape::new([node_count]);

    // 构建 in_degree 多项式：sum across services
    let in_degree = SymbolCombination::new(shape.clone(), "node_bw_in", |row, _vec| {
        let node_idx = normal_node_indices[row];
        let mut monomials = Vec::new();
        for s in 0..service_count {
            let poly = svc_bw.in_degree.symbol_polynomial_at(&[row, s]);
            monomials.extend_from_slice(poly.monomials());
        }
        let id = next_auto_intermediate_symbol_id();
        LinearExpressionSymbol::new(
            id,
            &format!("node_bw_in_{}", node_idx),
            monomials,
            0.0,
        )
    });
    model.add_symbol_combination(&in_degree)?;

    // 构建 out_degree 多项式：sum across services
    let out_degree = SymbolCombination::new(shape.clone(), "node_bw_out", |row, _vec| {
        let node_idx = normal_node_indices[row];
        let mut monomials = Vec::new();
        for s in 0..service_count {
            let poly = svc_bw.out_degree.symbol_polynomial_at(&[row, s]);
            monomials.extend_from_slice(poly.monomials());
        }
        let id = next_auto_intermediate_symbol_id();
        LinearExpressionSymbol::new(
            id,
            &format!("node_bw_out_{}", node_idx),
            monomials,
            0.0,
        )
    });
    model.add_symbol_combination(&out_degree)?;

    // 构建 out_flow 多项式：sum across services
    let out_flow = SymbolCombination::new(shape, "node_bw_flow", |row, _vec| {
        let node_idx = normal_node_indices[row];
        let mut monomials = Vec::new();
        for s in 0..service_count {
            let poly = svc_bw.out_flow.symbol_polynomial_at(&[row, s]);
            monomials.extend_from_slice(poly.monomials());
        }
        let id = next_auto_intermediate_symbol_id();
        LinearExpressionSymbol::new(
            id,
            &format!("node_bw_flow_{}", node_idx),
            monomials,
            0.0,
        )
    });
    model.add_symbol_combination(&out_flow)?;

    Ok(NodeBandwidth { in_degree, out_degree, out_flow })
}

/// 从 bandwidth[e] 的多项式中提取指定 service s 的系数
fn extract_service_coeff_from_bw(
    bandwidth: &SymbolCombination<f64, LinearExpressionSymbol<f64>, Shape<1>>,
    edge_index: usize,
    service_index: usize,
    y_idx: &ospf_rust_multiarray::MultiArray<usize, Shape<2>>,
) -> Option<f64> {
    let target_var = y_idx[&[edge_index, service_index]];
    let poly = bandwidth.symbol_polynomial(edge_index);
    poly.monomials()
        .iter()
        .find(|m| m.var_index() == target_var)
        .map(|m| *m.coefficient())
}
