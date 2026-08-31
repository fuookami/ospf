/// 边 / Edge (对齐 Kotlin Edge)
#[derive(Debug, Clone)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub max_bandwidth: f64,
    pub cost_per_bandwidth: f64,
}
