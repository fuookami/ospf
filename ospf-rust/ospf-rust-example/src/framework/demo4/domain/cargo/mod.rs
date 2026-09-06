//! 货物领域模块 / Cargo domain module.
/// 货运上下文模块 / Cargo context module
pub mod context;

/// 货物领域聚合 / Cargo domain aggregation
/// 对齐 Kotlin cargo Aggregation / Aligned with Kotlin cargo Aggregation
#[derive(Debug)]
pub struct Aggregation {
    /// 货物数量 / Cargo count
    pub cargo_count: usize,
}

impl Aggregation {
    /// 创建新的货物聚合 / Create new cargo aggregation
    pub fn new() -> Self {
        Self { cargo_count: 0 }
    }
}
