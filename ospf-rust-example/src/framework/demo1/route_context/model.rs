#[derive(Debug, Clone)]
pub struct Node {
    pub id: u64,
    pub is_client: bool,
    pub demand: f64,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub max_bandwidth: f64,
    pub cost_per_bandwidth: f64,
}

#[derive(Debug, Clone)]
pub struct Service {
    pub capacity: f64,
    pub cost: f64,
}

#[derive(Debug, Clone)]
pub struct Assignment {
    pub normal_node_indices: Vec<usize>,
    pub x_idx: Vec<Vec<usize>>,
}
