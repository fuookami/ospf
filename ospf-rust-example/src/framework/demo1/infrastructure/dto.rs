//! 数据传输对象：输入/输出结构定义 / Data transfer objects: input/output structure definitions

/// 边的数据传输对象 / Edge data transfer object
#[derive(Debug, Clone)]
pub struct EdgeDTO {
    /// 起始节点标识 / Source node identifier
    pub from_node_id: u64,
    /// 目标节点标识 / Target node identifier
    pub to_node_id: u64,
    /// 最大带宽 / Maximum bandwidth
    pub max_bandwidth: u64,
    /// 单位带宽成本 / Cost per unit bandwidth
    pub cost_per_bandwidth: u64,
}

/// 客户节点数据传输对象 / Client node data transfer object
#[derive(Debug, Clone)]
pub struct ClientNodeDTO {
    /// 客户节点标识 / Client node identifier
    pub id: u64,
    /// 关联的普通节点标识 / Associated normal node identifier
    pub normal_node_id: u64,
    /// 带宽需求 / Bandwidth demand
    pub demand: u64,
}

/// 输入数据 / Input data
#[derive(Debug, Clone)]
pub struct Input {
    /// 服务成本 / Service cost
    pub service_cost: u64,
    /// 普通节点数量 / Number of normal nodes
    pub normal_node_amount: usize,
    /// 边列表 / Edge list
    pub edges: Vec<EdgeDTO>,
    /// 客户节点列表 / Client node list
    pub client_nodes: Vec<ClientNodeDTO>,
}

/// 输出数据 / Output data
#[derive(Debug, Clone)]
pub struct Output {
    /// 服务路径列表，每条路径为节点标识序列 / Service path list, each path is a sequence of node identifiers
    pub links: Vec<Vec<u64>>,
}
