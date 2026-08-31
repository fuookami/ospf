//! Benders 分解求解器 / Benders decomposition solver
/// Benders 分解求解器 / Benders decomposition solver
///
/// 对齐 Kotlin BendersSolver / Aligned with Kotlin BendersSolver
#[derive(Debug, Clone)]
pub struct BendersSolver {
    /// 最大迭代次数 / Maximum number of iterations
    pub max_iterations: usize,
    /// 收敛容差 / Convergence tolerance
    pub tolerance: f64,
}

impl BendersSolver {
    /// 创建新的 Benders 求解器 / Create a new Benders solver
    pub fn new(max_iterations: usize, tolerance: f64) -> Self {
        Self { max_iterations, tolerance }
    }
}
