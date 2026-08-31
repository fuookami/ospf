//! 串行组合线性求解器
//! Serial Combinatorial Linear Solver
//!
//! 本模块提供串行执行的组合线性求解器。
//! This module provides serial-executing combinatorial linear solvers.

use std::sync::Arc;
use ospf_rust_core::error::{CoreError, Result, SolverError};
use ospf_rust_core::model::intermediate::LinearTriadModel;
use super::parallel_combinatorial_linear_solver::LinearSolver;
use super::{FeasibleSolution, FrameworkSolveOptions};

/// 串行组合线性求解器 / Serial Combinatorial Linear Solver
///
/// 按顺序尝试多个求解器，返回第一个成功的结果。
/// Tries multiple solvers in sequence, returning the first successful result.
pub struct SerialCombinatorialLinearSolver {
    /// 求解器列表 / Solver list
    solvers: Vec<Arc<dyn LinearSolver>>,
    /// 名称缓存 / Cached name
    name: String,
}

impl SerialCombinatorialLinearSolver {
    /// 创建新的串行组合线性求解器 / Create new serial combinatorial linear solver
    pub fn new(solvers: Vec<Arc<dyn LinearSolver>>) -> Self {
        let names: Vec<&str> = solvers.iter().map(|s| s.name()).collect();
        let name = format!("SerialCombinatorial({})", names.join(","));
        Self { solvers, name }
    }

    /// 获取求解器名称 / Get solver name
    pub fn name(&self) -> &str {
        &self.name
    }

    fn should_stop_on_error(error: &CoreError) -> bool {
        matches!(
            error,
            CoreError::Solver(SolverError::Infeasible | SolverError::Unbounded)
        )
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl LinearSolver for SerialCombinatorialLinearSolver {
    fn name(&self) -> &str {
        &self.name
    }

    async fn solve(&self, model: &LinearTriadModel) -> Result<FeasibleSolution> {
        for solver in &self.solvers {
            match solver.solve(model).await {
                Ok(solution) => {
                    log::info!("Solver {} found a solution.", solver.name());
                    return Ok(solution);
                }
                Err(e) => {
                    if Self::should_stop_on_error(&e) {
                        return Err(e);
                    }
                    log::warn!("Solver {} failed with error: {}", solver.name(), e);
                }
            }
        }
        Err(CoreError::Solver(SolverError::NotAvailable(
            "No solver valid.".into(),
        )))
    }

    fn solve_multi_with_options<'a>(
        &'a self,
        model: &'a LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<(FeasibleSolution, Vec<Vec<f64>>)>> + Send + 'a,
        >,
    > {
        Box::pin(async move {
            for solver in &self.solvers {
                match solver
                    .solve_multi_with_options(model, options.clone())
                    .await
                {
                    Ok(result) => {
                        log::info!("Solver {} found a solution.", solver.name());
                        return Ok(result);
                    }
                    Err(e) => {
                        if Self::should_stop_on_error(&e) {
                            return Err(e);
                        }
                        log::warn!("Solver {} failed with error: {}", solver.name(), e);
                    }
                }
            }
            Err(CoreError::Solver(SolverError::NotAvailable(
                "No solver valid.".into(),
            )))
        })
    }
}

#[cfg(not(feature = "async"))]
impl LinearSolver for SerialCombinatorialLinearSolver {
    fn name(&self) -> &str {
        &self.name
    }

    fn solve(&self, model: &LinearTriadModel) -> Result<FeasibleSolution> {
        for solver in &self.solvers {
            match solver.solve(model) {
                Ok(solution) => {
                    log::info!("Solver {} found a solution.", solver.name());
                    return Ok(solution);
                }
                Err(e) => {
                    if Self::should_stop_on_error(&e) {
                        return Err(e);
                    }
                    log::warn!("Solver {} failed with error: {}", solver.name(), e);
                }
            }
        }
        Err(CoreError::Solver(SolverError::NotAvailable(
            "No solver valid.".into(),
        )))
    }

    fn solve_multi_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<(FeasibleSolution, Vec<Vec<f64>>)> {
        for solver in &self.solvers {
            match solver.solve_multi_with_options(model, options.clone()) {
                Ok(result) => {
                    log::info!("Solver {} found a solution.", solver.name());
                    return Ok(result);
                }
                Err(e) => {
                    if Self::should_stop_on_error(&e) {
                        return Err(e);
                    }
                    log::warn!("Solver {} failed with error: {}", solver.name(), e);
                }
            }
        }
        Err(CoreError::Solver(SolverError::NotAvailable(
            "No solver valid.".into(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(feature = "async"))]
    use crate::solver::parallel_combinatorial_linear_solver::LinearMetaModelSolverExt;
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::model::{ConstraintRelation, MetaModel};
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::variable::ContinuousVariableItem;
    #[cfg(not(feature = "async"))]
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockSolver {
        name: String,
        should_fail: bool,
    }

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl LinearSolver for MockSolver {
        fn name(&self) -> &str {
            &self.name
        }

        #[cfg(feature = "async")]
        async fn solve(&self, _model: &LinearTriadModel) -> Result<FeasibleSolution> {
            if self.should_fail {
                Err(CoreError::Solver(SolverError::SolveFailed(
                    "Mock error".into(),
                )))
            } else {
                Ok(FeasibleSolution::new(1.0, vec![1.0]))
            }
        }

        #[cfg(not(feature = "async"))]
        fn solve(&self, _model: &LinearTriadModel) -> Result<FeasibleSolution> {
            if self.should_fail {
                Err(CoreError::Solver(SolverError::SolveFailed(
                    "Mock error".into(),
                )))
            } else {
                Ok(FeasibleSolution::new(1.0, vec![1.0]))
            }
        }

        #[cfg(feature = "async")]
        fn solve_multi_with_options<'a>(
            &'a self,
            model: &'a LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<(FeasibleSolution, Vec<Vec<f64>>)>>
                    + Send
                    + 'a,
            >,
        > {
            Box::pin(async move {
                let result = self.solve_with_options(model, options).await?;
                Ok((result.clone(), vec![result.solution]))
            })
        }

        #[cfg(not(feature = "async"))]
        fn solve_multi_with_options(
            &self,
            model: &LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> Result<(FeasibleSolution, Vec<Vec<f64>>)> {
            let result = self.solve_with_options(model, options)?;
            Ok((result.clone(), vec![result.solution]))
        }
    }

    #[test]
    fn test_serial_combinatorial_solver() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![
            Arc::new(MockSolver {
                name: "solver1".to_string(),
                should_fail: true,
            }),
            Arc::new(MockSolver {
                name: "solver2".to_string(),
                should_fail: false,
            }),
        ];

        let solver = SerialCombinatorialLinearSolver::new(solvers);
        assert!(solver.name().contains("solver1"));
        assert!(solver.name().contains("solver2"));
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_serial_stops_on_infeasible_error() {
        enum StopErrorKind {
            Infeasible,
        }

        struct StopMockSolver {
            name: String,
            error: Option<StopErrorKind>,
            call_counter: Arc<AtomicUsize>,
        }

        impl LinearSolver for StopMockSolver {
            fn name(&self) -> &str {
                &self.name
            }

            fn solve(&self, _model: &LinearTriadModel) -> Result<FeasibleSolution> {
                self.call_counter.fetch_add(1, Ordering::SeqCst);
                match &self.error {
                    Some(StopErrorKind::Infeasible) => {
                        Err(CoreError::Solver(SolverError::Infeasible))
                    }
                    None => Ok(FeasibleSolution::new(1.0, vec![1.0])),
                }
            }

            fn solve_multi_with_options(
                &self,
                model: &LinearTriadModel,
                options: FrameworkSolveOptions,
            ) -> Result<(FeasibleSolution, Vec<Vec<f64>>)> {
                let result = self.solve_with_options(model, options)?;
                Ok((result.clone(), vec![result.solution]))
            }
        }

        let first_calls = Arc::new(AtomicUsize::new(0));
        let second_calls = Arc::new(AtomicUsize::new(0));
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![
            Arc::new(StopMockSolver {
                name: "solver1".to_string(),
                error: Some(StopErrorKind::Infeasible),
                call_counter: first_calls.clone(),
            }),
            Arc::new(StopMockSolver {
                name: "solver2".to_string(),
                error: None,
                call_counter: second_calls.clone(),
            }),
        ];
        let solver = SerialCombinatorialLinearSolver::new(solvers);
        let model = LinearTriadModel::new("serial_stop");

        let result = solver.solve(&model);
        assert!(matches!(
            result,
            Err(CoreError::Solver(SolverError::Infeasible))
        ));
        assert_eq!(first_calls.load(Ordering::SeqCst), 1);
        assert_eq!(second_calls.load(Ordering::SeqCst), 0);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_serial_solver_supports_meta_model_shortcut() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![Arc::new(MockSolver {
            name: "solver_meta".to_string(),
            should_fail: false,
        })];
        let solver = SerialCombinatorialLinearSolver::new(solvers);
        let mut meta_model = MetaModel::<f64>::new("serial_linear_meta_shortcut");
        let x = ContinuousVariableItem::auto("serial_linear_meta_shortcut_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                1.0,
                "serial_linear_meta_shortcut_c",
            )
            .unwrap();

        let result = solver
            .solve_meta(&meta_model)
            .expect("serial linear solver meta shortcut should succeed");
        assert!((result.obj - 1.0).abs() <= 1e-9);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_serial_solver_meta_shortcut_rejects_non_finite_value() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![Arc::new(MockSolver {
            name: "solver_meta_non_finite".to_string(),
            should_fail: false,
        })];
        let solver = SerialCombinatorialLinearSolver::new(solvers);
        let mut meta_model = MetaModel::<f64>::new("serial_linear_meta_non_finite");
        let x = ContinuousVariableItem::auto("serial_linear_meta_non_finite_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, f64::NAN)],
                ConstraintRelation::LessEqual,
                1.0,
                "serial_linear_meta_non_finite_c",
            )
            .unwrap();

        let options = FrameworkSolveOptions::new().with_value_conversion_policy(
            ospf_rust_core::solver::SolveValueConversionPolicy::Strict,
        );
        let error = solver
            .solve_meta_with_options(&meta_model, options)
            .expect_err("strict mode should reject non-finite conversion");
        assert!(matches!(
            error,
            CoreError::Solver(SolverError::NonFinite(_))
        ));
    }
}
