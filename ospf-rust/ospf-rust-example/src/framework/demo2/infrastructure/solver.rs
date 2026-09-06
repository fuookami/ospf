//! 求解器构建器 / Solver builder
/// 求解器构建器 / Solver builder (对齐 Kotlin LinearSolverBuilder / Aligned with Kotlin LinearSolverBuilder)
///
/// 用于构建列生成求解器 / Used to build column generation solvers
#[derive(Debug, Clone)]
pub struct LinearSolverBuilder {
    /// 求解器类型 / Solver type
    pub solver_type: SolverType,
}

/// 求解器类型 / Solver type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverType {
    /// Gurobi 求解器 / Gurobi solver
    Gurobi,
    /// SCIP 求解器 / SCIP solver
    Scip,
}

impl LinearSolverBuilder {
    /// 创建新的求解器构建器 / Create a new solver builder
    pub fn new(solver_type: SolverType) -> Self {
        Self { solver_type }
    }
}
