//! 路由聚合数据 / Route aggregation data

use super::model::{Assignment, Edge, Graph, Node, Service};

/// 路由聚合 / Route aggregation
pub struct Aggregation {
    /// 图结构 / Graph structure
    pub graph: Graph,
    /// 服务列表 / Service list
    pub services: Vec<Service>,
    /// 分配数据 / Assignment data
    pub assignment: Assignment,
}

impl Aggregation {
    /// 创建新的路由聚合 / Create a new route aggregation
    pub fn new(graph: Graph, services: Vec<Service>, assignment: Assignment) -> Self {
        Self {
            graph,
            services,
            assignment,
        }
    }
}
