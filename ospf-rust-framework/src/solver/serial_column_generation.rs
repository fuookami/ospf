//! 串行组合列生成求解器
//! Serial Combinatorial Column Generation Solver
//!
//! 本模块提供串行执行的组合列生成求解器。
//! This module provides serial-executing combinatorial column generation solvers.

use super::column_generation::{RegistrationStatusCallback, SolvingStatusCallback};
use super::{ColumnGenerationSolver, FeasibleSolution, LPResult, SolveOptions};
use ospf_rust_core::error::{CoreError, Result, SolverError};
use ospf_rust_core::model::intermediate::LinearTriadModel;
use std::sync::Arc;

/// 串行组合列生成求解器 / Serial Combinatorial Column Generation Solver
///
/// 按顺序尝试多个求解器，返回第一个成功的结果。
/// Tries multiple solvers in sequence, returning the first successful result.
pub struct SerialCombinatorialColumnGenerationSolver {
    /// 求解器列表 / Solver list
    solvers: Vec<Arc<dyn ColumnGenerationSolver>>,
    /// 名称缓存 / Cached name
    name: String,
}

impl SerialCombinatorialColumnGenerationSolver {
    /// 创建新的串行组合列生成求解器 / Create new serial combinatorial column generation solver
    pub fn new(solvers: Vec<Arc<dyn ColumnGenerationSolver>>) -> Self {
        let names: Vec<&str> = solvers.iter().map(|s| s.name()).collect();
        let name = format!("SerialCombinatorial({})", names.join(","));
        Self { solvers, name }
    }

    /// 使用默认配置创建 / Create with default configuration
    pub fn with_solvers(solvers: Vec<Arc<dyn ColumnGenerationSolver>>) -> Self {
        Self::new(solvers)
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

    fn wrap_solving_status_callback(
        callback: Option<SolvingStatusCallback>,
        solver_name: String,
        solver_index: usize,
    ) -> Option<SolvingStatusCallback> {
        callback.map(|callback| {
            Arc::new(move |status: &super::column_generation::SolvingStatus| {
                let mut mapped = status.clone();
                mapped.solver = solver_name.clone();
                mapped.solver_index = solver_index;
                callback(&mapped)
            }) as SolvingStatusCallback
        })
    }

    fn wrap_registration_status_callback(
        callback: Option<RegistrationStatusCallback>,
        solver_name: String,
    ) -> Option<RegistrationStatusCallback> {
        callback.map(|callback| {
            Arc::new(
                move |status: &super::column_generation::RegistrationStatus| {
                    let mut mapped = status.clone();
                    mapped.solver = solver_name.clone();
                    callback(&mapped)
                },
            ) as RegistrationStatusCallback
        })
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl ColumnGenerationSolver for SerialCombinatorialColumnGenerationSolver {
    fn name(&self) -> &str {
        &self.name
    }

    async fn solve_milp_with_options(
        &self,
        model: &LinearTriadModel,
        options: SolveOptions,
    ) -> Result<FeasibleSolution> {
        let solve_name = options.name.clone().unwrap_or_else(|| self.name.clone());
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver_name = solver.name().to_string();
            let wrapped_registration_callback = Self::wrap_registration_status_callback(
                options.registration_status_callback.clone(),
                solver_name.clone(),
            );
            let wrapped_solving_callback = Self::wrap_solving_status_callback(
                options.solving_status_callback.clone(),
                solver_name,
                solver_index,
            );
            let options = options
                .clone()
                .with_name(solve_name.clone())
                .with_registration_callback(wrapped_registration_callback)
                .with_solving_callback(wrapped_solving_callback);
            match solver.solve_milp_with_options(model, options).await {
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

    async fn solve_lp_with_options(
        &self,
        model: &LinearTriadModel,
        options: SolveOptions,
    ) -> Result<LPResult> {
        let solve_name = options.name.clone().unwrap_or_else(|| self.name.clone());
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver_name = solver.name().to_string();
            let wrapped_registration_callback = Self::wrap_registration_status_callback(
                options.registration_status_callback.clone(),
                solver_name.clone(),
            );
            let wrapped_solving_callback = Self::wrap_solving_status_callback(
                options.solving_status_callback.clone(),
                solver_name,
                solver_index,
            );
            let options = options
                .clone()
                .with_name(solve_name.clone())
                .with_registration_callback(wrapped_registration_callback)
                .with_solving_callback(wrapped_solving_callback);
            match solver.solve_lp_with_options(model, options).await {
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

#[cfg(not(feature = "async"))]
impl ColumnGenerationSolver for SerialCombinatorialColumnGenerationSolver {
    fn name(&self) -> &str {
        &self.name
    }

    fn solve_milp_with_options(
        &self,
        model: &LinearTriadModel,
        options: SolveOptions,
    ) -> Result<FeasibleSolution> {
        let solve_name = options.name.clone().unwrap_or_else(|| self.name.clone());
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver_name = solver.name().to_string();
            let wrapped_registration_callback = Self::wrap_registration_status_callback(
                options.registration_status_callback.clone(),
                solver_name.clone(),
            );
            let wrapped_solving_callback = Self::wrap_solving_status_callback(
                options.solving_status_callback.clone(),
                solver_name,
                solver_index,
            );
            let options = options
                .clone()
                .with_name(solve_name.clone())
                .with_registration_callback(wrapped_registration_callback)
                .with_solving_callback(wrapped_solving_callback);
            match solver.solve_milp_with_options(model, options) {
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

    fn solve_lp_with_options(
        &self,
        model: &LinearTriadModel,
        options: SolveOptions,
    ) -> Result<LPResult> {
        let solve_name = options.name.clone().unwrap_or_else(|| self.name.clone());
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver_name = solver.name().to_string();
            let wrapped_registration_callback = Self::wrap_registration_status_callback(
                options.registration_status_callback.clone(),
                solver_name.clone(),
            );
            let wrapped_solving_callback = Self::wrap_solving_status_callback(
                options.solving_status_callback.clone(),
                solver_name,
                solver_index,
            );
            let options = options
                .clone()
                .with_name(solve_name.clone())
                .with_registration_callback(wrapped_registration_callback)
                .with_solving_callback(wrapped_solving_callback);
            match solver.solve_lp_with_options(model, options) {
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
    use crate::solver::LinearDualSolution;
    #[cfg(not(feature = "async"))]
    use crate::solver::column_generation::{RegistrationStatus, SolvingStatus};
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::model::{ConstraintRelation, MetaModel};
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::variable::ContinuousVariableItem;
    #[cfg(not(feature = "async"))]
    use std::sync::Mutex;
    #[cfg(not(feature = "async"))]
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockSolver {
        name: String,
        should_fail: bool,
    }

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl ColumnGenerationSolver for MockSolver {
        fn name(&self) -> &str {
            &self.name
        }

        #[cfg(feature = "async")]
        async fn solve_milp_with_options(
            &self,
            _model: &LinearTriadModel,
            _options: SolveOptions,
        ) -> Result<FeasibleSolution> {
            if self.should_fail {
                Err(CoreError::Solver(SolverError::SolveFailed(
                    "Mock error".into(),
                )))
            } else {
                Ok(FeasibleSolution::new(1.0, vec![1.0]))
            }
        }

        #[cfg(not(feature = "async"))]
        fn solve_milp_with_options(
            &self,
            _model: &LinearTriadModel,
            options: SolveOptions,
        ) -> Result<FeasibleSolution> {
            if self.should_fail {
                Err(CoreError::Solver(SolverError::SolveFailed(
                    "Mock error".into(),
                )))
            } else {
                if let Some(callback) = options.registration_status_callback {
                    callback(&RegistrationStatus::new(self.name.clone(), 1, 1, 1))?;
                }
                if let Some(callback) = options.solving_status_callback {
                    callback(&SolvingStatus::new(self.name.clone(), 0, 1.0))?;
                }
                Ok(FeasibleSolution::new(1.0, vec![1.0]))
            }
        }

        #[cfg(feature = "async")]
        async fn solve_lp_with_options(
            &self,
            _model: &LinearTriadModel,
            _options: SolveOptions,
        ) -> Result<LPResult> {
            if self.should_fail {
                Err(CoreError::Solver(SolverError::SolveFailed(
                    "Mock error".into(),
                )))
            } else {
                Ok(LPResult::new(
                    FeasibleSolution::new(1.0, vec![1.0]),
                    LinearDualSolution::default(),
                ))
            }
        }

        #[cfg(not(feature = "async"))]
        fn solve_lp_with_options(
            &self,
            _model: &LinearTriadModel,
            options: SolveOptions,
        ) -> Result<LPResult> {
            if self.should_fail {
                Err(CoreError::Solver(SolverError::SolveFailed(
                    "Mock error".into(),
                )))
            } else {
                if let Some(callback) = options.registration_status_callback {
                    callback(&RegistrationStatus::new(self.name.clone(), 1, 1, 1))?;
                }
                if let Some(callback) = options.solving_status_callback {
                    callback(&SolvingStatus::new(self.name.clone(), 0, 1.0))?;
                }
                Ok(LPResult::new(
                    FeasibleSolution::new(1.0, vec![1.0]),
                    LinearDualSolution::default(),
                ))
            }
        }
    }

    #[test]
    fn test_serial_combinatorial_column_generation_solver() {
        let solvers: Vec<Arc<dyn ColumnGenerationSolver>> = vec![
            Arc::new(MockSolver {
                name: "solver1".to_string(),
                should_fail: true,
            }),
            Arc::new(MockSolver {
                name: "solver2".to_string(),
                should_fail: false,
            }),
        ];

        let solver = SerialCombinatorialColumnGenerationSolver::with_solvers(solvers);
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

        impl ColumnGenerationSolver for StopMockSolver {
            fn name(&self) -> &str {
                &self.name
            }

            fn solve_milp_with_options(
                &self,
                _model: &LinearTriadModel,
                _options: SolveOptions,
            ) -> Result<FeasibleSolution> {
                self.call_counter.fetch_add(1, Ordering::SeqCst);
                match &self.error {
                    Some(StopErrorKind::Infeasible) => {
                        Err(CoreError::Solver(SolverError::Infeasible))
                    }
                    None => Ok(FeasibleSolution::new(1.0, vec![1.0])),
                }
            }

            fn solve_lp_with_options(
                &self,
                _model: &LinearTriadModel,
                _options: SolveOptions,
            ) -> Result<LPResult> {
                self.call_counter.fetch_add(1, Ordering::SeqCst);
                match &self.error {
                    Some(StopErrorKind::Infeasible) => {
                        Err(CoreError::Solver(SolverError::Infeasible))
                    }
                    None => Ok(LPResult::new(
                        FeasibleSolution::new(1.0, vec![1.0]),
                        LinearDualSolution::default(),
                    )),
                }
            }
        }

        let first_calls = Arc::new(AtomicUsize::new(0));
        let second_calls = Arc::new(AtomicUsize::new(0));
        let solvers: Vec<Arc<dyn ColumnGenerationSolver>> = vec![
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
        let solver = SerialCombinatorialColumnGenerationSolver::with_solvers(solvers);
        let model = LinearTriadModel::new("serial_stop_cg");

        let result = solver
            .solve_milp_with_options(&model, SolveOptions::new().with_name("serial_stop_case"));
        assert!(matches!(
            result,
            Err(CoreError::Solver(SolverError::Infeasible))
        ));
        assert_eq!(first_calls.load(Ordering::SeqCst), 1);
        assert_eq!(second_calls.load(Ordering::SeqCst), 0);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_serial_solver_forwards_callbacks_with_solver_index() {
        let solvers: Vec<Arc<dyn ColumnGenerationSolver>> = vec![
            Arc::new(MockSolver {
                name: "solver1".to_string(),
                should_fail: true,
            }),
            Arc::new(MockSolver {
                name: "solver2".to_string(),
                should_fail: false,
            }),
        ];
        let solver = SerialCombinatorialColumnGenerationSolver::with_solvers(solvers);
        let model = LinearTriadModel::new("serial_callback_model");
        let solving_statuses: Arc<Mutex<Vec<SolvingStatus>>> = Arc::new(Mutex::new(Vec::new()));
        let registration_statuses: Arc<Mutex<Vec<RegistrationStatus>>> =
            Arc::new(Mutex::new(Vec::new()));

        let solving_statuses_for_callback = solving_statuses.clone();
        let solving_callback: SolvingStatusCallback = Arc::new(move |status| {
            solving_statuses_for_callback
                .lock()
                .unwrap()
                .push(status.clone());
            Ok(())
        });
        let registration_statuses_for_callback = registration_statuses.clone();
        let registration_callback: RegistrationStatusCallback = Arc::new(move |status| {
            registration_statuses_for_callback
                .lock()
                .unwrap()
                .push(status.clone());
            Ok(())
        });

        let options = SolveOptions::new()
            .with_name("serial_callback_case")
            .with_log_model(true)
            .with_registration_callback(Some(registration_callback))
            .with_solving_callback(Some(solving_callback));
        let _ = solver
            .solve_milp_with_options(&model, options)
            .expect("serial solver should return feasible solution");

        let solving_statuses = solving_statuses.lock().unwrap();
        let registration_statuses = registration_statuses.lock().unwrap();
        assert_eq!(solving_statuses.len(), 1);
        assert_eq!(registration_statuses.len(), 1);
        assert_eq!(solving_statuses[0].solver_index, 1);
        assert_eq!(solving_statuses[0].solver, "solver2");
        assert_eq!(registration_statuses[0].solver, "solver2");
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_serial_solver_supports_meta_model_shortcut() {
        let solvers: Vec<Arc<dyn ColumnGenerationSolver>> = vec![Arc::new(MockSolver {
            name: "solver_meta".to_string(),
            should_fail: false,
        })];
        let solver = SerialCombinatorialColumnGenerationSolver::with_solvers(solvers);
        let mut meta_model = MetaModel::<f64>::new("serial_cg_meta_shortcut");
        let x = ContinuousVariableItem::auto("serial_cg_meta_shortcut_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, 1.0)],
                ConstraintRelation::LessEqual,
                1.0,
                "serial_cg_meta_shortcut_c",
            )
            .unwrap();

        let result = solver
            .solve(&meta_model)
            .expect("serial column-generation meta shortcut should succeed");
        assert!((result.obj - 1.0).abs() <= 1e-9);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn test_serial_solver_meta_shortcut_rejects_non_finite_value() {
        let solvers: Vec<Arc<dyn ColumnGenerationSolver>> = vec![Arc::new(MockSolver {
            name: "solver_meta_non_finite".to_string(),
            should_fail: false,
        })];
        let solver = SerialCombinatorialColumnGenerationSolver::with_solvers(solvers);
        let mut meta_model = MetaModel::<f64>::new("serial_cg_meta_non_finite");
        let x = ContinuousVariableItem::auto("serial_cg_meta_non_finite_x");
        let x_index = meta_model.register_variable(x).unwrap();
        meta_model
            .add_linear_constraint(
                &[(x_index, f64::NAN)],
                ConstraintRelation::LessEqual,
                1.0,
                "serial_cg_meta_non_finite_c",
            )
            .unwrap();

        let options = SolveOptions::new().with_value_conversion_policy(
            ospf_rust_core::solver::SolveValueConversionPolicy::Strict,
        );
        let error = solver
            .solve_with_options(&meta_model, options)
            .expect_err("strict mode should reject non-finite conversion");
        assert!(matches!(
            error,
            CoreError::Solver(SolverError::NonFinite(_))
        ));
    }
}
