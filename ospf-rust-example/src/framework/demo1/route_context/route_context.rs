//! 路由上下文 / Route context

use super::aggregation::Aggregation;
use super::model::{Assignment, Edge, Graph, Node, NodeKind, Service};
use crate::framework::demo1::infrastructure::dto::Input;
use ospf_rust_core::model::MetaModel;
use std::error::Error;

/// 路由上下文 / Route context
pub struct RouteContext {
    /// 聚合数据 / Aggregation data
    pub aggregation: Option<Aggregation>,
}

impl RouteContext {
    /// 创建新的路由上下文 / Create a new route context
    pub fn new() -> Self {
        Self { aggregation: None }
    }

    /// 从输入数据初始化路由上下文 / Initialize route context from input data
    pub fn init(&mut self, input: &Input) -> Result<(), Box<dyn Error>> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut services = Vec::new();

        let total_demand: f64 = input.client_nodes.iter().map(|c| c.demand as f64).sum();
        let service_count = (input.normal_node_amount / 2).max(1);
        for i in 0..service_count {
            services.push(Service {
                id: i as u64,
                capacity: total_demand,
                cost: input.service_cost as f64,
            });
        }

        for id in 0..input.normal_node_amount {
            nodes.push(Node::normal(id as u64));
        }

        for client in &input.client_nodes {
            nodes.push(Node::client(client.id, client.demand as f64));
        }

        for edge in &input.edges {
            let e1 = edges.len();
            edges.push(Edge {
                from: edge.from_node_id as usize,
                to: edge.to_node_id as usize,
                max_bandwidth: edge.max_bandwidth as f64,
                cost_per_bandwidth: edge.cost_per_bandwidth as f64,
            });
            nodes[edge.from_node_id as usize].add_edge(e1);

            let e2 = edges.len();
            edges.push(Edge {
                from: edge.to_node_id as usize,
                to: edge.from_node_id as usize,
                max_bandwidth: edge.max_bandwidth as f64,
                cost_per_bandwidth: edge.cost_per_bandwidth as f64,
            });
            nodes[edge.to_node_id as usize].add_edge(e2);
        }

        for (offset, client) in input.client_nodes.iter().enumerate() {
            let client_index = input.normal_node_amount + offset;
            let e = edges.len();
            edges.push(Edge {
                from: client.normal_node_id as usize,
                to: client_index,
                max_bandwidth: client.demand as f64,
                cost_per_bandwidth: 0.0,
            });
            nodes[client.normal_node_id as usize].add_edge(e);
        }

        let normal_node_indices: Vec<usize> = nodes
            .iter()
            .enumerate()
            .filter_map(|(idx, node)| if node.is_normal() { Some(idx) } else { None })
            .collect();

        let graph = Graph { nodes, edges };
        let assignment = Assignment::new(normal_node_indices);

        self.aggregation = Some(Aggregation::new(graph, services, assignment));
        Ok(())
    }

    /// 注册路由相关变量和符号到模型 / Register route-related variables and symbols into the model
    pub fn register(&mut self, model: &mut MetaModel<f64>) -> Result<(), Box<dyn Error>> {
        let agg = self
            .aggregation
            .as_mut()
            .ok_or("route context not initialized")?;

        let service_count = agg.services.len();
        agg.assignment.register(model, service_count)?;
        Ok(())
    }

    /// 构建路由相关约束和目标 / Construct route-related constraints and objectives
    pub fn construct(&self, model: &mut MetaModel<f64>) -> Result<(), Box<dyn Error>> {
        let agg = self
            .aggregation
            .as_ref()
            .ok_or("route context not initialized")?;

        super::service::generate_pipelines(agg, model)
    }

    /// 获取指定普通节点和服务的分配变量索引 / Get assignment variable index for specified normal node and service
    pub fn assignment_variable(&self, normal_node_idx: usize, service_idx: usize) -> Option<usize> {
        let agg = self.aggregation.as_ref()?;
        let row = agg
            .assignment
            .normal_node_indices
            .iter()
            .position(|idx| *idx == normal_node_idx)?;
        Some(agg.assignment.x_idx[&[row, service_idx]])
    }
}
