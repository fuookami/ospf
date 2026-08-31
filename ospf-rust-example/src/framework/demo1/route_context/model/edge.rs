//! 边模型 / Edge model

/// 边模型 / Edge model (对齐 Kotlin Edge)
#[derive(Debug, Clone)]
pub struct Edge {
    /// 起始节点索引 / Source node index
    pub from: usize,
    /// 目标节点索引 / Target node index
    pub to: usize,
    /// 最大带宽 / Maximum bandwidth
    pub max_bandwidth: f64,
    /// 单位带宽成本 / Cost per unit bandwidth
    pub cost_per_bandwidth: f64,
}
