//! 机组领域模块 / Crew domain module.
/// 机组上下文模块 / Crew context module
pub mod context;
/// 机组模型模块 / Crew model module
pub mod model;

/// 机组领域聚合 / Crew domain aggregation
#[derive(Debug)]
pub struct Aggregation {
    /// 机组列表 / Crew list
    pub crews: Vec<model::Crew>,
}
