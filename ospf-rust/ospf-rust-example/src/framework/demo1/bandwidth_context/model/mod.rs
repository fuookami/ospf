//! 带宽模型模块 / Bandwidth model module

/// 边带宽模型 / Edge bandwidth model
pub mod edge_bandwidth;
/// 节点带宽模型 / Node bandwidth model
pub mod node_bandwidth;
/// 服务带宽模型 / Service bandwidth model
pub mod service_bandwidth;

/// 边带宽 / Edge bandwidth
pub use edge_bandwidth::EdgeBandwidth;
/// 节点带宽 / Node bandwidth
pub use node_bandwidth::NodeBandwidth;
/// 服务带宽 / Service bandwidth
pub use service_bandwidth::ServiceBandwidth;
