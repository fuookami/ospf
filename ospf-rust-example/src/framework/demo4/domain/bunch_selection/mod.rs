pub mod service;

/// Bunch 选择上下文 / Bunch selection context
/// 对齐 Kotlin BunchSelectionContext
#[derive(Debug)]
pub struct BunchSelectionContext {
    pub max_iterations: usize,
    pub tolerance: f64,
}

impl BunchSelectionContext {
    pub fn new() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-6,
        }
    }
}
