//! Demo5 solver selection / Demo5 solver selection.

use ospf_rust_core::model::ObjectiveCategory;
use ospf_rust_core::model::intermediate::{BasicLinearTriadModel, LinearTriadModel};
use ospf_rust_core::solver::{LinearSolver, SolverCapability, SolverInfo, SolverOutput};
use ospf_rust_core::token::Token;
use ospf_rust_core::variable::{UContinuousVariableItem, VariableId, VariableType};

/// Demo5 支持的 solver 后端 / Solver backends supported by demo5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Demo5Solver {
    /// Gurobi / Gurobi.
    Gurobi,
    /// SCIP / SCIP.
    Scip,
}

/// 探测 native solver 是否可用 / Probe whether a native solver is available.
///
/// 该探测只用于 solver-gated 测试；外部环境不可用时返回 `false` 并输出明确诊断，
/// 调用方应将其作为 native gate 失败处理，而不是静默返回通过。
/// This probe is only for solver-gated tests; it returns `false` with an explicit diagnostic
/// when the external environment is unavailable. Callers must treat that as a failed native
/// gate rather than silently passing.
pub fn native_solver_available<S>(solver: &S) -> bool
where
    S: LinearSolver,
{
    let mut basic = BasicLinearTriadModel::new("demo5_native_solver_probe");
    let variable = UContinuousVariableItem::create(
        VariableId::standalone(9_000_001),
        "demo5_native_solver_probe_x",
    );
    basic.add_variable_with_bounds(
        Token::from_generic(variable, 0),
        0.0,
        1.0,
        VariableType::Continuous,
    );
    let mut model = LinearTriadModel::from_basic(basic);
    model.set_objective(vec![0.0], ObjectiveCategory::Minimum);

    match solver.solve_linear(&model) {
        Ok(output) if output.status.is_feasible() && output.solution.is_some() => true,
        Ok(output) => {
            eprintln!(
                "[SKIP] Demo5 native solver '{}' returned an unusable probe status: {:?}",
                solver.name(),
                output.status
            );
            false
        }
        Err(error) => {
            eprintln!(
                "[SKIP] Demo5 native solver '{}' is unavailable: {}",
                solver.name(),
                error
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_core::error::{CoreError, SolverEnvironmentLostError};
    use ospf_rust_core::solver::SolverStatus;

    struct UnavailableSolver;

    impl SolverInfo for UnavailableSolver {
        fn name(&self) -> &str {
            "unavailable-test-solver"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear]
        }
    }

    impl LinearSolver for UnavailableSolver {
        fn solve_linear(&self, _model: &LinearTriadModel) -> ospf_rust_core::Result<SolverOutput> {
            Err(CoreError::SolverEnvironmentLost(
                SolverEnvironmentLostError::new("test-only native environment is unavailable"),
            ))
        }
    }

    struct UnusableSolver;

    impl SolverInfo for UnusableSolver {
        fn name(&self) -> &str {
            "unusable-test-solver"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear]
        }
    }

    impl LinearSolver for UnusableSolver {
        fn solve_linear(&self, _model: &LinearTriadModel) -> ospf_rust_core::Result<SolverOutput> {
            Ok(SolverOutput::new(SolverStatus::Unknown))
        }
    }

    #[test]
    fn unavailable_native_solver_is_explicitly_skipped() {
        assert!(!native_solver_available(&UnavailableSolver));
    }

    #[test]
    fn unusable_native_solver_status_is_explicitly_skipped() {
        assert!(!native_solver_available(&UnusableSolver));
    }
}
