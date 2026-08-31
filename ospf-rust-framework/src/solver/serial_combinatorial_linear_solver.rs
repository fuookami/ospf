//! 串行组合线性求解器
//! Serial Combinatorial Linear Solver
//!
//! 本模块提供串行执行的组合线性求解器。
//! This module provides serial-executing combinatorial linear solvers.

use super::column_generation_solver::{
    CombinatorialFallbackPolicy, CombinatorialSelection, aggregate_attempt_id,
    aggregate_combinatorial_reports_with_metadata, child_attempt_id, preserve_cancelled_attempts,
};
use super::framework_solve_options::child_cancellation_handle;
use super::parallel_combinatorial_linear_solver::LinearSolver;
use super::{FeasibleSolution, FrameworkSolveOptions, legacy_cancellation_error};
use ospf_rust_core::error::{CoreError, Result, SolverError, SolverNotFoundError};
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::solver::{
    CombinatorialSolveReport, SolveReport, SolverProvenance, cancelled_solve_report,
};
use std::sync::Arc;

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
        Self::should_stop_on_error_with_policy(error, &CombinatorialFallbackPolicy::default())
    }

    fn should_stop_on_error_with_policy(
        error: &CoreError,
        policy: &CombinatorialFallbackPolicy,
    ) -> bool {
        policy.should_stop_on_error(error)
    }

    fn should_stop_on_report_with_policy(
        report: &SolveReport<f64>,
        policy: &CombinatorialFallbackPolicy,
    ) -> bool {
        policy.should_stop_on_report(report)
    }

    fn cancellation_requested(options: &FrameworkSolveOptions) -> bool {
        options
            .cancellation_handle
            .as_ref()
            .is_some_and(|handle| handle.is_cancelled())
    }

    fn should_stop_on_legacy_error(error: &CoreError, options: &FrameworkSolveOptions) -> bool {
        Self::should_stop_on_error_with_policy(error, &options.fallback_policy)
            || Self::cancellation_requested(options)
    }

    fn pre_cancelled_report(
        &self,
        options: &FrameworkSolveOptions,
    ) -> Result<Option<CombinatorialSolveReport<f64>>> {
        let Some(handle) = options.cancellation_handle.as_ref() else {
            return Ok(None);
        };
        if !handle.is_cancelled() {
            return Ok(None);
        }
        let report = cancelled_solve_report(
            SolverProvenance {
                solver_id: self.name.clone(),
                backend_name: "framework-combinatorial".to_owned(),
                ..SolverProvenance::default()
            },
            handle,
        )?;
        Ok(Some(CombinatorialSolveReport::single(
            report,
            format!("{}#0", self.name),
        )))
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl LinearSolver for SerialCombinatorialLinearSolver {
    fn name(&self) -> &str {
        &self.name
    }

    async fn solve_combinatorial_report_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<CombinatorialSolveReport<f64>> {
        if let Some(report) = self.pre_cancelled_report(&options)? {
            return Ok(report);
        }
        let mut attempts = Vec::new();
        let mut child_handles = Vec::new();
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver_name = solver.name().to_owned();
            let child_handle = child_cancellation_handle(&options);
            let attempt_options = options
                .clone()
                .with_cancellation_handle(Some(child_handle.clone()));
            let result = solver
                .solve_report_with_options(model, attempt_options)
                .await;
            child_handle.mark_completed();
            let should_stop = child_handle.cancellation_preceded_completion()
                || match &result {
                    Ok(report) => {
                        Self::should_stop_on_report_with_policy(report, &options.fallback_policy)
                    }
                    Err(error) => {
                        Self::should_stop_on_error_with_policy(error, &options.fallback_policy)
                            || Self::cancellation_requested(&options)
                    }
                };
            child_handles.push(child_handle);
            attempts.push((solver_index, solver_name, result));
            if should_stop {
                break;
            }
        }
        aggregate_combinatorial_reports_with_metadata(
            self.name(),
            CombinatorialSelection::First,
            preserve_cancelled_attempts(attempts, &child_handles),
        )
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
        Err(CoreError::SolverNotFound(SolverNotFoundError::new(
            "No solver valid.",
        )))
    }

    fn solve_with_options<'a>(
        &'a self,
        model: &'a LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FeasibleSolution>> + Send + 'a>>
    {
        Box::pin(async move {
            if Self::cancellation_requested(&options) {
                return Err(legacy_cancellation_error(&options));
            }
            for solver in &self.solvers {
                match solver.solve_with_options(model, options.clone()).await {
                    Ok(solution) => {
                        log::info!("Solver {} found a solution.", solver.name());
                        return Ok(solution);
                    }
                    Err(error) => {
                        if Self::should_stop_on_legacy_error(&error, &options) {
                            return Err(error);
                        }
                        log::warn!("Solver {} failed with error: {}", solver.name(), error);
                    }
                }
            }
            Err(CoreError::SolverNotFound(SolverNotFoundError::new(
                "No solver valid.",
            )))
        })
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
            if Self::cancellation_requested(&options) {
                return Err(legacy_cancellation_error(&options));
            }
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
                        if Self::should_stop_on_legacy_error(&e, &options) {
                            return Err(e);
                        }
                        log::warn!("Solver {} failed with error: {}", solver.name(), e);
                    }
                }
            }
            Err(CoreError::SolverNotFound(SolverNotFoundError::new(
                "No solver valid.",
            )))
        })
    }
}

#[cfg(not(feature = "async"))]
impl LinearSolver for SerialCombinatorialLinearSolver {
    fn name(&self) -> &str {
        &self.name
    }

    fn solve_combinatorial_report_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<CombinatorialSolveReport<f64>> {
        if let Some(report) = self.pre_cancelled_report(&options)? {
            return Ok(report);
        }
        let mut attempts = Vec::new();
        let mut child_handles = Vec::new();
        for (solver_index, solver) in self.solvers.iter().enumerate() {
            let solver_name = solver.name().to_owned();
            let child_handle = child_cancellation_handle(&options);
            let attempt_options = options
                .clone()
                .with_cancellation_handle(Some(child_handle.clone()));
            let result = solver.solve_report_with_options(model, attempt_options);
            child_handle.mark_completed();
            let should_stop = child_handle.cancellation_preceded_completion()
                || match &result {
                    Ok(report) => {
                        Self::should_stop_on_report_with_policy(report, &options.fallback_policy)
                    }
                    Err(error) => {
                        Self::should_stop_on_error_with_policy(error, &options.fallback_policy)
                            || Self::cancellation_requested(&options)
                    }
                };
            child_handles.push(child_handle);
            attempts.push((solver_index, solver_name, result));
            if should_stop {
                break;
            }
        }
        aggregate_combinatorial_reports_with_metadata(
            self.name(),
            CombinatorialSelection::First,
            preserve_cancelled_attempts(attempts, &child_handles),
        )
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
        Err(CoreError::SolverNotFound(SolverNotFoundError::new(
            "No solver valid.",
        )))
    }

    fn solve_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<FeasibleSolution> {
        if Self::cancellation_requested(&options) {
            return Err(legacy_cancellation_error(&options));
        }
        for solver in &self.solvers {
            match solver.solve_with_options(model, options.clone()) {
                Ok(solution) => {
                    log::info!("Solver {} found a solution.", solver.name());
                    return Ok(solution);
                }
                Err(error) => {
                    if Self::should_stop_on_legacy_error(&error, &options) {
                        return Err(error);
                    }
                    log::warn!("Solver {} failed with error: {}", solver.name(), error);
                }
            }
        }
        Err(CoreError::SolverNotFound(SolverNotFoundError::new(
            "No solver valid.",
        )))
    }

    fn solve_multi_with_options(
        &self,
        model: &LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> Result<(FeasibleSolution, Vec<Vec<f64>>)> {
        if Self::cancellation_requested(&options) {
            return Err(legacy_cancellation_error(&options));
        }
        for solver in &self.solvers {
            match solver.solve_multi_with_options(model, options.clone()) {
                Ok(result) => {
                    log::info!("Solver {} found a solution.", solver.name());
                    return Ok(result);
                }
                Err(e) => {
                    if Self::should_stop_on_legacy_error(&e, &options) {
                        return Err(e);
                    }
                    log::warn!("Solver {} failed with error: {}", solver.name(), e);
                }
            }
        }
        Err(CoreError::SolverNotFound(SolverNotFoundError::new(
            "No solver valid.",
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
    use ospf_rust_core::solver::{ProblemStatus, SolveReport, TerminationReason};
    #[cfg(not(feature = "async"))]
    use ospf_rust_core::variable::ContinuousVariableItem;
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

    struct CancelledReportMockSolver {
        name: String,
        calls: Arc<AtomicUsize>,
        cancelled: bool,
        cancel_on_success: bool,
    }

    fn cancelled_report() -> SolveReport<f64> {
        SolveReport::builder(ProblemStatus::Unknown, TerminationReason::Cancelled)
            .build()
            .expect("cancelled report should be valid")
    }

    #[test]
    fn terminal_errors_stop_the_linear_fallback_chain() {
        for error in [
            CoreError::Solver(SolverError::Cancelled("backend".to_owned())),
            CoreError::Solver(SolverError::Interrupted("external".to_owned())),
            CoreError::Solver(SolverError::Timeout(std::time::Duration::from_secs(1))),
        ] {
            assert!(SerialCombinatorialLinearSolver::should_stop_on_error(
                &error
            ));
        }
        assert!(!SerialCombinatorialLinearSolver::should_stop_on_error(
            &CoreError::Solver(SolverError::SolveFailed("fallback".to_owned()))
        ));
    }

    #[cfg_attr(feature = "async", async_trait::async_trait)]
    impl LinearSolver for CancelledReportMockSolver {
        fn name(&self) -> &str {
            &self.name
        }

        #[cfg(feature = "async")]
        async fn solve(&self, _model: &LinearTriadModel) -> Result<FeasibleSolution> {
            Ok(FeasibleSolution::new(1.0, vec![1.0]))
        }

        #[cfg(not(feature = "async"))]
        fn solve(&self, _model: &LinearTriadModel) -> Result<FeasibleSolution> {
            Ok(FeasibleSolution::new(1.0, vec![1.0]))
        }

        #[cfg(feature = "async")]
        async fn solve_report_with_options(
            &self,
            _model: &LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> Result<SolveReport<f64>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.cancelled {
                Ok(cancelled_report())
            } else {
                if self.cancel_on_success {
                    let handle = options
                        .cancellation_handle
                        .as_ref()
                        .expect("serial attempt should receive a child cancellation handle");
                    assert!(handle.cancel(ospf_rust_core::solver::CancellationOrigin::Callback));
                }
                FeasibleSolution::new(1.0, vec![1.0]).to_solve_report(&self.name)
            }
        }

        #[cfg(not(feature = "async"))]
        fn solve_report_with_options(
            &self,
            _model: &LinearTriadModel,
            options: FrameworkSolveOptions,
        ) -> Result<SolveReport<f64>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.cancelled {
                Ok(cancelled_report())
            } else {
                if self.cancel_on_success {
                    let handle = options
                        .cancellation_handle
                        .as_ref()
                        .expect("serial attempt should receive a child cancellation handle");
                    assert!(handle.cancel(ospf_rust_core::solver::CancellationOrigin::Callback));
                }
                FeasibleSolution::new(1.0, vec![1.0]).to_solve_report(&self.name)
            }
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
    fn report_entry_keeps_failed_attempt_before_selected_success() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![
            Arc::new(MockSolver {
                name: "report-failing".to_owned(),
                should_fail: true,
            }),
            Arc::new(MockSolver {
                name: "report-success".to_owned(),
                should_fail: false,
            }),
        ];
        let solver = SerialCombinatorialLinearSolver::new(solvers);
        let aggregate = solver
            .solve_combinatorial_report_with_options(
                &LinearTriadModel::new("serial_report_contract"),
                FrameworkSolveOptions::default(),
            )
            .expect("serial report entry should select the second solver");

        assert_eq!(aggregate.attempts.len(), 2);
        assert_eq!(
            aggregate.selected_attempt_id.as_deref(),
            Some(
                child_attempt_id(&aggregate_attempt_id(solver.name()), "report-success", 1,)
                    .as_str(),
            )
        );
        assert!(aggregate.report.has_incumbent());
        assert!(aggregate.report.diagnostics.issues.iter().any(|issue| {
            issue.code == "AttemptFailed" && issue.message.contains("Mock error")
        }));
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn report_entry_keeps_failed_attempt_before_selected_success() {
        let solvers: Vec<Arc<dyn LinearSolver>> = vec![
            Arc::new(MockSolver {
                name: "report-failing".to_owned(),
                should_fail: true,
            }),
            Arc::new(MockSolver {
                name: "report-success".to_owned(),
                should_fail: false,
            }),
        ];
        let solver = SerialCombinatorialLinearSolver::new(solvers);
        let aggregate = solver
            .solve_combinatorial_report_with_options(
                &LinearTriadModel::new("serial_report_contract"),
                FrameworkSolveOptions::default(),
            )
            .await
            .expect("serial report entry should select the second solver");

        assert_eq!(aggregate.attempts.len(), 2);
        assert_eq!(
            aggregate.selected_attempt_id.as_deref(),
            Some(
                child_attempt_id(&aggregate_attempt_id(solver.name()), "report-success", 1,)
                    .as_str(),
            )
        );
        assert!(aggregate.report.has_incumbent());
        assert!(aggregate.report.diagnostics.issues.iter().any(|issue| {
            issue.code == "AttemptFailed" && issue.message.contains("Mock error")
        }));
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn report_cancellation_stops_the_fallback_chain() {
        let first_calls = Arc::new(AtomicUsize::new(0));
        let second_calls = Arc::new(AtomicUsize::new(0));
        let solver = SerialCombinatorialLinearSolver::new(vec![
            Arc::new(CancelledReportMockSolver {
                name: "cancelled".to_owned(),
                calls: Arc::clone(&first_calls),
                cancelled: true,
                cancel_on_success: false,
            }),
            Arc::new(CancelledReportMockSolver {
                name: "fallback".to_owned(),
                calls: Arc::clone(&second_calls),
                cancelled: false,
                cancel_on_success: false,
            }),
        ]);

        let aggregate = solver
            .solve_combinatorial_report_with_options(
                &LinearTriadModel::new("serial_cancelled_report"),
                FrameworkSolveOptions::default(),
            )
            .expect("cancelled report should be aggregated");
        assert_eq!(
            aggregate.report.termination_reason,
            TerminationReason::Cancelled
        );
        assert_eq!(aggregate.attempts.len(), 1);
        assert_eq!(first_calls.load(Ordering::SeqCst), 1);
        assert_eq!(second_calls.load(Ordering::SeqCst), 0);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn report_cancellation_stops_the_fallback_chain() {
        let first_calls = Arc::new(AtomicUsize::new(0));
        let second_calls = Arc::new(AtomicUsize::new(0));
        let solver = SerialCombinatorialLinearSolver::new(vec![
            Arc::new(CancelledReportMockSolver {
                name: "cancelled".to_owned(),
                calls: Arc::clone(&first_calls),
                cancelled: true,
                cancel_on_success: false,
            }),
            Arc::new(CancelledReportMockSolver {
                name: "fallback".to_owned(),
                calls: Arc::clone(&second_calls),
                cancelled: false,
                cancel_on_success: false,
            }),
        ]);

        let aggregate = solver
            .solve_combinatorial_report_with_options(
                &LinearTriadModel::new("serial_cancelled_report"),
                FrameworkSolveOptions::default(),
            )
            .await
            .expect("cancelled report should be aggregated");
        assert_eq!(
            aggregate.report.termination_reason,
            TerminationReason::Cancelled
        );
        assert_eq!(aggregate.attempts.len(), 1);
        assert_eq!(first_calls.load(Ordering::SeqCst), 1);
        assert_eq!(second_calls.load(Ordering::SeqCst), 0);
    }

    #[cfg(not(feature = "async"))]
    #[test]
    fn report_success_after_attempt_cancellation_is_not_selected() {
        let first_calls = Arc::new(AtomicUsize::new(0));
        let second_calls = Arc::new(AtomicUsize::new(0));
        let solver = SerialCombinatorialLinearSolver::new(vec![
            Arc::new(CancelledReportMockSolver {
                name: "cancel-after-start".to_owned(),
                calls: Arc::clone(&first_calls),
                cancelled: false,
                cancel_on_success: true,
            }),
            Arc::new(CancelledReportMockSolver {
                name: "must-not-run".to_owned(),
                calls: Arc::clone(&second_calls),
                cancelled: false,
                cancel_on_success: false,
            }),
        ]);

        let aggregate = solver
            .solve_combinatorial_report_with_options(
                &LinearTriadModel::new("serial_cancel_after_start"),
                FrameworkSolveOptions::default(),
            )
            .expect("cancelled attempt should produce a structured report");

        assert_eq!(
            aggregate.report.termination_reason,
            TerminationReason::Cancelled
        );
        assert!(!aggregate.report.has_incumbent());
        assert_eq!(aggregate.attempts.len(), 1);
        assert_eq!(first_calls.load(Ordering::SeqCst), 1);
        assert_eq!(second_calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            aggregate.attempts[0].cancellation_reason.as_deref(),
            Some("CALLBACK")
        );
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn report_success_after_attempt_cancellation_is_not_selected() {
        let first_calls = Arc::new(AtomicUsize::new(0));
        let second_calls = Arc::new(AtomicUsize::new(0));
        let solver = SerialCombinatorialLinearSolver::new(vec![
            Arc::new(CancelledReportMockSolver {
                name: "cancel-after-start".to_owned(),
                calls: Arc::clone(&first_calls),
                cancelled: false,
                cancel_on_success: true,
            }),
            Arc::new(CancelledReportMockSolver {
                name: "must-not-run".to_owned(),
                calls: Arc::clone(&second_calls),
                cancelled: false,
                cancel_on_success: false,
            }),
        ]);

        let aggregate = solver
            .solve_combinatorial_report_with_options(
                &LinearTriadModel::new("serial_cancel_after_start"),
                FrameworkSolveOptions::default(),
            )
            .await
            .expect("cancelled attempt should produce a structured report");

        assert_eq!(
            aggregate.report.termination_reason,
            TerminationReason::Cancelled
        );
        assert!(!aggregate.report.has_incumbent());
        assert_eq!(aggregate.attempts.len(), 1);
        assert_eq!(first_calls.load(Ordering::SeqCst), 1);
        assert_eq!(second_calls.load(Ordering::SeqCst), 0);
        assert_eq!(
            aggregate.attempts[0].cancellation_reason.as_deref(),
            Some("CALLBACK")
        );
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
