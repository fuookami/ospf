use std::collections::HashMap;
use crate::framework::demo1::route_context::model::{Assignment, Edge, Graph, Node, NodeKind, Service};
use super::super::aggregation::Aggregation;

/// 从求解结果中提取服务路径（DFS 追踪）
/// 对齐 Kotlin SolutionAnalyzer
pub struct SolutionAnalyzer<'a> {
    graph: &'a Graph,
    services: &'a [Service],
    assignment: &'a Assignment,
    aggregation: &'a Aggregation,
}

impl<'a> SolutionAnalyzer<'a> {
    pub fn new(
        graph: &'a Graph,
        services: &'a [Service],
        assignment: &'a Assignment,
        aggregation: &'a Aggregation,
    ) -> Self {
        Self { graph, services, assignment, aggregation }
    }

    /// 分析求解结果，返回服务路径列表
    /// 每条路径是从 assigned node 到 client node 的节点序列
    pub fn analyze(&self, solution: &[f64]) -> Vec<Vec<u64>> {
        // 1. 找到每个服务的分配节点
        let mut node_solution: HashMap<usize, usize> = HashMap::new(); // service_idx -> node_idx
        for (row, &node_idx) in self.assignment.normal_node_indices.iter().enumerate() {
            for s in 0..self.services.len() {
                let var_idx = self.assignment.x_idx[row][s];
                let value = solution.get(var_idx).copied().unwrap_or(0.0);
                if (value - 1.0).abs() < 1e-6 {
                    node_solution.insert(s, node_idx);
                }
            }
        }

        // 2. 找到每条使用的边
        let y_idx = &self.aggregation.edge_bandwidth.y_idx;
        let mut edge_solution: HashMap<usize, Vec<(usize, usize)>> = HashMap::new(); // service_idx -> [(from, to)]
        for (e, _edge) in self.graph.edges.iter().enumerate() {
            for s in 0..self.services.len() {
                let var_idx = y_idx[e][s];
                let value = solution.get(var_idx).copied().unwrap_or(0.0);
                if value > 1e-6 {
                    edge_solution
                        .entry(s)
                        .or_insert_with(Vec::new)
                        .push((self.graph.edges[e].from, self.graph.edges[e].to));
                }
            }
        }

        // 3. DFS 追踪路径
        let mut all_links = Vec::new();
        for (&service_idx, &first_node) in &node_solution {
            if let Some(edges) = edge_solution.get(&service_idx) {
                for &(from, to) in edges {
                    if from == first_node {
                        let mut link = vec![self.graph.nodes[first_node].id, self.graph.nodes[to].id];
                        self.find_link(edges, to, &mut link, &mut all_links);
                    }
                }
            }
        }

        all_links
    }

    /// DFS 递归追踪路径
    fn find_link(
        &self,
        edges: &[(usize, usize)],
        current_node: usize,
        current_link: &mut Vec<u64>,
        all_links: &mut Vec<Vec<u64>>,
    ) {
        if self.graph.nodes[current_node].is_client() {
            all_links.push(current_link.clone());
        } else {
            for &(from, to) in edges {
                if from == current_node && !current_link.contains(&self.graph.nodes[to].id) {
                    current_link.push(self.graph.nodes[to].id);
                    self.find_link(edges, to, current_link, all_links);
                    current_link.pop();
                }
            }
        }
    }
}
