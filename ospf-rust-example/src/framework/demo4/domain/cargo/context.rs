/// 货物上下文 / Cargo context
/// 对齐 Kotlin CargoContext
#[derive(Debug)]
pub struct CargoContext {
    pub aggregation: super::Aggregation,
}

impl CargoContext {
    pub fn new() -> Self {
        Self {
            aggregation: super::Aggregation::new(),
        }
    }
}
