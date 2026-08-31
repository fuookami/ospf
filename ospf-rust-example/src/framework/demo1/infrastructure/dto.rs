/// 边的数据传输对象 / Edge data transfer object
#[derive(Debug, Clone)]
pub struct EdgeDTO {
    pub from_node_id: u64,
    pub to_node_id: u64,
    pub max_bandwidth: u64,
    pub cost_per_bandwidth: u64,
}

/// 客户节点数据传输对象 / Client node data transfer object
#[derive(Debug, Clone)]
pub struct ClientNodeDTO {
    pub id: u64,
    pub normal_node_id: u64,
    pub demand: u64,
}

/// 输入数据 / Input data
#[derive(Debug, Clone)]
pub struct Input {
    pub service_cost: u64,
    pub normal_node_amount: usize,
    pub edges: Vec<EdgeDTO>,
    pub client_nodes: Vec<ClientNodeDTO>,
}

/// 输出数据 / Output data
#[derive(Debug, Clone)]
pub struct Output {
    pub links: Vec<Vec<u64>>,
}
