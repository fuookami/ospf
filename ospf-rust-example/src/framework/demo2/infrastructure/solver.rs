/// 求解器构建器 / Solver builder (对齐 Kotlin LinearSolverBuilder)
/// 用于构建列生成求解器
#[derive(Debug, Clone)]
pub struct LinearSolverBuilder {
    pub solver_type: SolverType,
}

/// 求解器类型 / Solver type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverType {
    Gurobi,
    Scip,
}

impl LinearSolverBuilder {
    pub fn new(solver_type: SolverType) -> Self {
        Self { solver_type }
    }
}
