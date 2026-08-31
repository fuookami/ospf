//! 编组选择领域模块 / Bunch selection domain module.
/// 编组选择服务模块 / Bunch selection service module
pub mod service;

/// Bunch 选择上下文 / Bunch selection context
/// 对齐 Kotlin BunchSelectionContext
#[derive(Debug)]
pub struct BunchSelectionContext {
    /// 最大迭代次数 / Maximum iteration count
    pub max_iterations: usize,
    /// 收敛容差 / Convergence tolerance
    pub tolerance: f64,
}

impl BunchSelectionContext {
    /// 创建新的编组选择上下文 / Create new bunch selection context
    pub fn new() -> Self {
        Self {
            max_iterations: 100,
            tolerance: 1e-6,
        }
    }
}
