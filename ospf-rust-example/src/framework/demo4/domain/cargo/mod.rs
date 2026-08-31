pub mod context;

/// 货物领域聚合 / Cargo domain aggregation
/// 对齐 Kotlin cargo Aggregation
#[derive(Debug)]
pub struct Aggregation {
    pub cargo_count: usize,
}

impl Aggregation {
    pub fn new() -> Self {
        Self { cargo_count: 0 }
    }
}
