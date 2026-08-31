/// Benders 分解求解器 / Benders decomposition solver
/// 对齐 Kotlin BendersSolver
#[derive(Debug, Clone)]
pub struct BendersSolver {
    pub max_iterations: usize,
    pub tolerance: f64,
}

impl BendersSolver {
    pub fn new(max_iterations: usize, tolerance: f64) -> Self {
        Self { max_iterations, tolerance }
    }
}
