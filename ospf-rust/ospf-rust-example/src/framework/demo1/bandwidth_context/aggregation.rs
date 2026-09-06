//! 带宽聚合数据 / Bandwidth aggregation data

use super::model::{EdgeBandwidth, NodeBandwidth, ServiceBandwidth};

/// 带宽聚合 / Bandwidth aggregation
pub struct Aggregation {
    /// 边带宽模型 / Edge bandwidth model
    pub edge_bandwidth: EdgeBandwidth,
    /// 服务带宽模型 / Service bandwidth model
    pub service_bandwidth: ServiceBandwidth,
    /// 节点带宽模型 / Node bandwidth model
    pub node_bandwidth: NodeBandwidth,
}

impl Aggregation {
    /// 创建新的带宽聚合 / Create a new bandwidth aggregation
    pub fn new(
        edge_bandwidth: EdgeBandwidth,
        service_bandwidth: ServiceBandwidth,
        node_bandwidth: NodeBandwidth,
    ) -> Self {
        Self {
            edge_bandwidth,
            service_bandwidth,
            node_bandwidth,
        }
    }
}
