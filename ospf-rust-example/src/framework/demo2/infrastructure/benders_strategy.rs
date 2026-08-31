//! Benders 分解策略 / Benders decomposition strategy
/// Benders 分解策略 / Benders decomposition strategy
///
/// 对齐 Kotlin BendersStrategy / Aligned with Kotlin BendersStrategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BendersStrategy {
    /// 标准 Benders 分解 / Standard Benders decomposition
    Standard,
    /// 自适应 Benders 分解 / Adaptive Benders decomposition
    Adaptive,
    /// 无 Benders（直接 MILP） / No Benders (direct MILP)
    None,
}
