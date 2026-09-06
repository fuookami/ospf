//! 货物上下文模块 / Cargo context module.
/// 货物上下文 / Cargo context
/// 对齐 Kotlin CargoContext / Aligned with Kotlin CargoContext
#[derive(Debug)]
pub struct CargoContext {
    /// 货物聚合 / Cargo aggregation
    pub aggregation: super::Aggregation,
}

impl CargoContext {
    /// 创建新的货物上下文 / Create new cargo context
    pub fn new() -> Self {
        Self {
            aggregation: super::Aggregation::new(),
        }
    }
}
