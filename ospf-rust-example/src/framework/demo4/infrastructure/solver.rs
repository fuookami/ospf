/// 线性求解器构建器 / Linear solver builder
/// 对齐 Kotlin LinearSolverBuilder
#[derive(Debug, Clone)]
pub struct LinearSolverBuilder {
    pub solver_type: SolverType,
}

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
