use super::model::Graph;

/// Bunch 生成聚合 / Bunch generation aggregation
/// 对齐 Kotlin bunch_generation Aggregation
#[derive(Debug)]
pub struct Aggregation {
    pub graph: Graph,
}

impl Aggregation {
    pub fn new(graph: Graph) -> Self {
        Self { graph }
    }
}
