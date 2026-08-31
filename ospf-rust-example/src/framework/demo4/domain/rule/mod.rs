pub mod context;
pub mod model;
pub mod service;

/// 规则领域聚合 / Rule domain aggregation
#[derive(Debug)]
pub struct Aggregation {
    pub locks: Vec<model::Lock>,
    pub links: Vec<model::Link>,
    pub restrictions: Vec<model::Restriction>,
}
