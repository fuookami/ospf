#[derive(Debug, Clone)]
pub struct EdgeDTO {
    pub from_node_id: u64,
    pub to_node_id: u64,
    pub max_bandwidth: u64,
    pub cost_per_bandwidth: u64,
}

#[derive(Debug, Clone)]
pub struct ClientNodeDTO {
    pub id: u64,
    pub normal_node_id: u64,
    pub demand: u64,
}

#[derive(Debug, Clone)]
pub struct Input {
    pub service_cost: u64,
    pub normal_node_amount: usize,
    pub edges: Vec<EdgeDTO>,
    pub client_nodes: Vec<ClientNodeDTO>,
}

#[derive(Debug, Clone)]
pub struct Output {
    pub links: Vec<Vec<u64>>,
}
