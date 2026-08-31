use super::model::{EdgeBandwidth, NodeBandwidth, ServiceBandwidth};

pub struct Aggregation {
    pub edge_bandwidth: EdgeBandwidth,
    pub service_bandwidth: ServiceBandwidth,
    pub node_bandwidth: NodeBandwidth,
}

impl Aggregation {
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
