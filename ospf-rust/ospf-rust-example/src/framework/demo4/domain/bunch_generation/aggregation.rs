//! 编组生成聚合模块 / Bunch generation aggregation module
use super::model::Graph;

/// Bunch 生成聚合 / Bunch generation aggregation
/// 对齐 Kotlin bunch_generation Aggregation
#[derive(Debug)]
pub struct Aggregation {
    /// 路线图 / Route graph
    pub graph: Graph,
}

impl Aggregation {
    /// 创建新的聚合 / Create new aggregation
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }
}
