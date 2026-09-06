//! 规则领域模块 / Rule domain module.
/// 规则上下文模块 / Rule context module
pub mod context;
/// 规则模型模块 / Rule model module
pub mod model;
/// 规则服务模块 / Rule service module
pub mod service;

/// 规则领域聚合 / Rule domain aggregation
#[derive(Debug)]
pub struct Aggregation {
    /// 锁定列表 / Lock list
    pub locks: Vec<model::Lock>,
    /// 链接列表 / Link list
    pub links: Vec<model::Link>,
    /// 限制列表 / Restriction list
    pub restrictions: Vec<model::Restriction>,
}
