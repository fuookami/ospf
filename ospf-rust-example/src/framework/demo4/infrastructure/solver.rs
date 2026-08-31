//! 求解器构建模块 / Solver builder module
//!
//! 提供线性求解器的构建配置，支持 Gurobi 和 SCIP 两种求解器后端。
//! Provides linear solver build configuration, supporting Gurobi and SCIP solver backends.

/// 线性求解器构建器 / Linear solver builder
/// 对齐 Kotlin LinearSolverBuilder
#[derive(Debug, Clone)]
pub struct LinearSolverBuilder {
    /// 求解器类型 / Solver type
    pub solver_type: SolverType,
}

/// 求解器类型枚举 / Solver type enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverType {
    /// Gurobi 求解器 / Gurobi solver
    Gurobi,
    /// SCIP 求解器 / SCIP solver
    Scip,
}

impl LinearSolverBuilder {
    /// 创建新的线性求解器构建器 / Create a new linear solver builder
    pub fn new(solver_type: SolverType) -> Self {
        Self { solver_type }
    }
}
