pub mod context;
pub mod model;

/// 机组领域聚合 / Crew domain aggregation
#[derive(Debug)]
pub struct Aggregation {
    pub crews: Vec<model::Crew>,
}
