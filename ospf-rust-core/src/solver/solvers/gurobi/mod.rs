//! Gurobi 求解器接口
//! Gurobi Solver Interface
//!
//! 此模块仅在启用 `gurobi*` feature 时可用。
//! This module is only available when a `gurobi*` feature is enabled.
//!
//! 原生回调语义说明 / Native callback semantics:
//! - `with_native_callback`：覆盖已有原生回调。
//! - `add_native_callback`：同样是覆盖语义（不是多播聚合）。
//! - 原因：`grb::callback::Where<'_>` 为按值消费且不可克隆，无法安全复用到多个 handler。
//! - 如需多 handler，请使用 `add_native_observer`（基于只读 snapshot 的多播观察回调）。
//! - 如果需要多 handler，请在一个原生回调中自行分发。
//!   If multiple handlers are needed, dispatch them inside one native callback.

mod config;
mod linear;
mod quadratic;
mod solver;

pub use config::GurobiConfig;
pub use config::{
    GurobiEnvCallback, GurobiNativeCallback, GurobiNativeControl, GurobiNativeObserver,
    GurobiNativeSnapshot, GurobiNativeWhere, GurobiNumericDiagnostics,
    GurobiNumericDiagnosticsCallback, GurobiNumericProfile, GurobiStage, GurobiStageCallback,
    GurobiStageStatus, GurobiTelemetryCallback, GurobiTelemetryStatus,
};
pub use solver::GurobiSolver;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::{SolverCapability, SolverInfo, SolverStatus};
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[test]
    fn test_gurobi_config() {
        let native_callback: GurobiNativeCallback = Arc::new(|_| Ok(()));
        let native_observer: GurobiNativeObserver = Arc::new(|_| Ok(GurobiNativeControl::Continue));
        let env_callback: GurobiEnvCallback = Arc::new(|_| Ok(()));
        let telemetry_callback: GurobiTelemetryCallback = Arc::new(|_| Ok(()));
        let config = GurobiConfig::new()
            .with_time_limit(60.0)
            .with_mip_gap(0.01)
            .with_output(false)
            .with_coefficient_zero_tolerance(1e-11)
            .with_numeric_focus(3)
            .with_scale_flag(2)
            .with_node_limit(1000)
            .with_mem_limit(8.0)
            .with_compute_server("127.0.0.1:61000")
            .with_server_password("password")
            .with_server_timeout(30)
            .with_cs_queue_timeout(30.0)
            .with_no_improvement_time_limit(15.0)
            .with_improve_threshold(1e-6)
            .with_telemetry_min_interval(0.25)
            .with_stage_callback(Some(Arc::new(|_| Ok(()))))
            .with_telemetry_callback(Some(telemetry_callback))
            .with_native_callback(Some(native_callback))
            .add_native_observer(native_observer)
            .with_env_callback(Some(env_callback));

        assert_eq!(config.time_limit, Some(60.0));
        assert_eq!(config.mip_gap, Some(0.01));
        assert!(!config.output_flag);
        assert_eq!(config.coefficient_zero_tolerance, 1e-11);
        assert_eq!(config.numeric_focus, Some(3));
        assert_eq!(config.scale_flag, Some(2));
        assert_eq!(config.node_limit, Some(1000));
        assert_eq!(config.mem_limit, Some(8.0));
        assert_eq!(config.compute_server.as_deref(), Some("127.0.0.1:61000"));
        assert_eq!(config.server_password.as_deref(), Some("password"));
        assert_eq!(config.server_timeout, Some(30));
        assert_eq!(config.cs_queue_timeout, Some(30.0));
        assert_eq!(config.no_improvement_time_limit, Some(15.0));
        assert_eq!(config.improve_threshold, Some(1e-6));
        assert_eq!(config.telemetry_min_interval, Some(0.25));
        assert!(config.stage_callback.is_some());
        assert!(config.telemetry_callback.is_some());
        assert!(config.native_callback.is_some());
        assert_eq!(config.native_observers.len(), 1);
        assert!(config.env_callback.is_some());
    }

    #[test]
    fn test_inf_or_unbd_status_mapping() {
        assert_eq!(
            super::solver::GurobiSolver::convert_status(grb::Status::InfOrUnbd),
            SolverStatus::InfeasibleOrUnbounded
        );
    }

    #[test]
    fn test_gurobi_solver() {
        let solver = GurobiSolver::new();
        assert_eq!(solver.name(), "Gurobi");
        assert!(solver.supports(SolverCapability::Linear));
        assert!(solver.supports(SolverCapability::Mip));
        assert!(solver.supports(SolverCapability::Quadratic));
        assert!(solver.supports(SolverCapability::NativeIndicator));
        assert!(solver.supports(SolverCapability::NativeSOS1));
    }

    #[test]
    fn test_recommended_defaults() {
        let config = GurobiConfig::recommended_defaults();
        assert_eq!(config.time_limit, Some(30.0));
        assert_eq!(config.mip_gap, Some(0.0));
        assert!(
            config
                .threads
                .is_some_and(|threads| threads > 0 && threads <= 32)
        );
        assert_eq!(config.coefficient_zero_tolerance, 1e-13);
        assert!(config.numeric_focus.is_none());
        assert!(config.scale_flag.is_none());

        let merged = GurobiConfig::new()
            .with_time_limit(12.0)
            .with_recommended_defaults();
        assert_eq!(merged.time_limit, Some(12.0));
        assert_eq!(merged.mip_gap, Some(0.0));
        assert!(
            merged
                .threads
                .is_some_and(|threads| threads > 0 && threads <= 32)
        );
    }

    #[test]
    fn test_numeric_profiles_defaults() {
        let robust = GurobiConfig::robust_defaults();
        assert_eq!(robust.time_limit, Some(30.0));
        assert_eq!(robust.mip_gap, Some(0.0));
        assert!(
            robust
                .threads
                .is_some_and(|threads| threads > 0 && threads <= 32)
        );
        assert_eq!(robust.coefficient_zero_tolerance, 1e-10);
        assert_eq!(robust.numeric_focus, Some(3));
        assert_eq!(robust.scale_flag, Some(2));

        let performance = GurobiConfig::performance_defaults();
        assert_eq!(performance.time_limit, Some(30.0));
        assert_eq!(performance.mip_gap, Some(0.0));
        assert!(
            performance
                .threads
                .is_some_and(|threads| threads > 0 && threads <= 32)
        );
        assert_eq!(performance.coefficient_zero_tolerance, 1e-13);
        assert_eq!(performance.numeric_focus, Some(0));
        assert_eq!(performance.scale_flag, Some(-1));

        let balanced = GurobiConfig::balanced_defaults();
        assert_eq!(balanced.time_limit, Some(30.0));
        assert_eq!(balanced.mip_gap, Some(0.0));
        assert!(
            balanced
                .threads
                .is_some_and(|threads| threads > 0 && threads <= 32)
        );
        assert_eq!(balanced.coefficient_zero_tolerance, 1e-12);
        assert_eq!(balanced.numeric_focus, Some(1));
        assert_eq!(balanced.scale_flag, Some(1));
    }

    #[test]
    fn test_numeric_profiles_merge_semantics() {
        let robust_merged = GurobiConfig::new()
            .with_numeric_focus(1)
            .with_scale_flag(1)
            .with_coefficient_zero_tolerance(1e-12)
            .with_robust_defaults();
        assert_eq!(robust_merged.numeric_focus, Some(1));
        assert_eq!(robust_merged.scale_flag, Some(1));
        assert_eq!(robust_merged.coefficient_zero_tolerance, 1e-10);

        let performance_merged = GurobiConfig::new()
            .with_numeric_focus(2)
            .with_scale_flag(3)
            .with_coefficient_zero_tolerance(1e-11)
            .with_performance_defaults();
        assert_eq!(performance_merged.numeric_focus, Some(2));
        assert_eq!(performance_merged.scale_flag, Some(3));
        assert_eq!(performance_merged.coefficient_zero_tolerance, 1e-11);

        let balanced_merged = GurobiConfig::new()
            .with_numeric_focus(2)
            .with_scale_flag(3)
            .with_coefficient_zero_tolerance(1e-13)
            .with_balanced_defaults();
        assert_eq!(balanced_merged.numeric_focus, Some(2));
        assert_eq!(balanced_merged.scale_flag, Some(3));
        assert_eq!(balanced_merged.coefficient_zero_tolerance, 1e-12);
    }

    #[test]
    fn test_numeric_diagnostics_callback_and_auto_apply_profile() {
        let diagnostics_snapshots: Arc<Mutex<Vec<GurobiNumericProfile>>> =
            Arc::new(Mutex::new(Vec::new()));
        let diagnostics_ref = diagnostics_snapshots.clone();
        let callback: GurobiNumericDiagnosticsCallback = Arc::new(move |diagnostics| {
            diagnostics_ref
                .lock()
                .expect("diagnostics lock poisoned")
                .push(diagnostics.recommended_profile);
            Ok(())
        });

        let solver = GurobiSolver::with_config(
            GurobiConfig::new()
                .with_numeric_diagnostics(true)
                .with_auto_apply_numeric_profile(true)
                .with_numeric_diagnostics_callback(Some(callback)),
        );
        let settings = solver
            .resolve_numeric_settings(&[1e12, 1e-12, 0.0, 3.0])
            .expect("diagnostics should succeed");
        assert_eq!(settings.numeric_focus, Some(3));
        assert_eq!(settings.scale_flag, Some(2));
        assert_eq!(settings.zero_tolerance, 1e-10);

        let snapshots = diagnostics_snapshots
            .lock()
            .expect("diagnostics lock poisoned");
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0], GurobiNumericProfile::Robust);
    }

    #[test]
    fn test_numeric_diagnostics_rejects_non_finite_coefficients() {
        let solver = GurobiSolver::with_config(GurobiConfig::new().with_numeric_diagnostics(true));
        let error = solver
            .resolve_numeric_settings(&[1.0, f64::NAN])
            .expect_err("non-finite coefficients should fail diagnostics");
        let message = error.to_string();
        assert!(
            message.to_lowercase().contains("non-finite"),
            "unexpected error message: {}",
            message
        );
    }

    #[test]
    fn test_numeric_diagnostics_callback_aggregates_handlers() {
        let hit_counter = Arc::new(AtomicUsize::new(0));
        let first_counter = hit_counter.clone();
        let second_counter = hit_counter.clone();
        let first: GurobiNumericDiagnosticsCallback = Arc::new(move |_| {
            first_counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        let second: GurobiNumericDiagnosticsCallback = Arc::new(move |_| {
            second_counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        let config = GurobiConfig::new()
            .add_numeric_diagnostics_callback(first)
            .add_numeric_diagnostics_callback(second);
        let callback = config
            .numeric_diagnostics_callback
            .as_ref()
            .expect("numeric diagnostics callback should be registered");
        callback(&GurobiNumericDiagnostics {
            nonzero_coefficient_count: 1,
            non_finite_coefficient_count: 0,
            min_abs_nonzero_coefficient: Some(1.0),
            max_abs_coefficient: Some(1.0),
            dynamic_range: Some(1.0),
            tiny_coefficient_count: 0,
            large_coefficient_count: 0,
            recommended_profile: GurobiNumericProfile::Performance,
        })
        .expect("aggregated diagnostics callbacks should succeed");
        assert_eq!(hit_counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_add_native_callback_overrides_previous_callback() {
        let first: GurobiNativeCallback = Arc::new(|_| Ok(()));
        let second: GurobiNativeCallback = Arc::new(|_| Ok(()));
        let config = GurobiConfig::new()
            .add_native_callback(first.clone())
            .add_native_callback(second.clone());
        let registered = config
            .native_callback
            .as_ref()
            .expect("native callback should be registered");
        assert!(
            Arc::ptr_eq(registered, &second),
            "last added native callback should override previous one"
        );
        assert!(
            !Arc::ptr_eq(registered, &first),
            "previous native callback should be replaced"
        );
    }

    #[test]
    fn test_stage_callback_for_filters_stage() {
        let hit_counter = Arc::new(AtomicUsize::new(0));
        let hit_counter_ref = hit_counter.clone();
        let callback: GurobiStageCallback = Arc::new(move |_| {
            hit_counter_ref.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        let config =
            GurobiConfig::new().add_stage_callback_for(GurobiStage::AfterFailure, callback);
        let chain = config
            .stage_callback
            .as_ref()
            .expect("stage callback should be registered");

        chain(&GurobiStageStatus {
            stage: GurobiStage::AfterModeling,
            solver_status: None,
            solve_time: Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        })
        .expect("non-matching stage should be ignored");
        assert_eq!(hit_counter.load(Ordering::SeqCst), 0);

        chain(&GurobiStageStatus {
            stage: GurobiStage::AfterFailure,
            solver_status: None,
            solve_time: Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        })
        .expect("matching stage should execute callback");
        assert_eq!(hit_counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_stage_callback_for_can_register_multiple_stages() {
        let modeling_counter = Arc::new(AtomicUsize::new(0));
        let failure_counter = Arc::new(AtomicUsize::new(0));
        let modeling_counter_ref = modeling_counter.clone();
        let failure_counter_ref = failure_counter.clone();
        let on_modeling: GurobiStageCallback = Arc::new(move |_| {
            modeling_counter_ref.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        let on_failure: GurobiStageCallback = Arc::new(move |_| {
            failure_counter_ref.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });

        let config = GurobiConfig::new()
            .add_stage_callback_for(GurobiStage::AfterModeling, on_modeling)
            .add_stage_callback_for(GurobiStage::AfterFailure, on_failure);
        let chain = config
            .stage_callback
            .as_ref()
            .expect("stage callback should be registered");

        chain(&GurobiStageStatus {
            stage: GurobiStage::AfterModeling,
            solver_status: None,
            solve_time: Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        })
        .expect("after-modeling stage should succeed");
        assert_eq!(modeling_counter.load(Ordering::SeqCst), 1);
        assert_eq!(failure_counter.load(Ordering::SeqCst), 0);

        chain(&GurobiStageStatus {
            stage: GurobiStage::AfterFailure,
            solver_status: None,
            solve_time: Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        })
        .expect("after-failure stage should succeed");
        assert_eq!(modeling_counter.load(Ordering::SeqCst), 1);
        assert_eq!(failure_counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_named_stage_helper_callbacks_filter_correctly() {
        let modeling_counter = Arc::new(AtomicUsize::new(0));
        let configuration_counter = Arc::new(AtomicUsize::new(0));
        let analyzing_counter = Arc::new(AtomicUsize::new(0));
        let failure_counter = Arc::new(AtomicUsize::new(0));
        let modeling_counter_ref = modeling_counter.clone();
        let configuration_counter_ref = configuration_counter.clone();
        let analyzing_counter_ref = analyzing_counter.clone();
        let failure_counter_ref = failure_counter.clone();
        let on_modeling: GurobiStageCallback = Arc::new(move |_| {
            modeling_counter_ref.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        let on_configuration: GurobiStageCallback = Arc::new(move |_| {
            configuration_counter_ref.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        let on_analyzing: GurobiStageCallback = Arc::new(move |_| {
            analyzing_counter_ref.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        let on_failure: GurobiStageCallback = Arc::new(move |_| {
            failure_counter_ref.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });

        let config = GurobiConfig::new()
            .add_after_modeling_callback(on_modeling)
            .add_configuration_callback(on_configuration)
            .add_analyzing_solution_callback(on_analyzing)
            .add_after_failure_callback(on_failure);
        let chain = config
            .stage_callback
            .as_ref()
            .expect("stage callback should be registered");

        chain(&GurobiStageStatus {
            stage: GurobiStage::Configuration,
            solver_status: None,
            solve_time: Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        })
        .expect("unrelated stage should be ignored");
        assert_eq!(modeling_counter.load(Ordering::SeqCst), 0);
        assert_eq!(configuration_counter.load(Ordering::SeqCst), 1);
        assert_eq!(analyzing_counter.load(Ordering::SeqCst), 0);
        assert_eq!(failure_counter.load(Ordering::SeqCst), 0);

        chain(&GurobiStageStatus {
            stage: GurobiStage::AfterModeling,
            solver_status: None,
            solve_time: Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        })
        .expect("after-modeling stage should execute modeling callback");
        assert_eq!(modeling_counter.load(Ordering::SeqCst), 1);
        assert_eq!(configuration_counter.load(Ordering::SeqCst), 1);
        assert_eq!(analyzing_counter.load(Ordering::SeqCst), 0);
        assert_eq!(failure_counter.load(Ordering::SeqCst), 0);

        chain(&GurobiStageStatus {
            stage: GurobiStage::AnalyzingSolution,
            solver_status: None,
            solve_time: Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        })
        .expect("analyzing stage should execute analyzing callback");
        assert_eq!(modeling_counter.load(Ordering::SeqCst), 1);
        assert_eq!(configuration_counter.load(Ordering::SeqCst), 1);
        assert_eq!(analyzing_counter.load(Ordering::SeqCst), 1);
        assert_eq!(failure_counter.load(Ordering::SeqCst), 0);

        chain(&GurobiStageStatus {
            stage: GurobiStage::AfterFailure,
            solver_status: None,
            solve_time: Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        })
        .expect("after-failure stage should execute failure callback");
        assert_eq!(modeling_counter.load(Ordering::SeqCst), 1);
        assert_eq!(configuration_counter.load(Ordering::SeqCst), 1);
        assert_eq!(analyzing_counter.load(Ordering::SeqCst), 1);
        assert_eq!(failure_counter.load(Ordering::SeqCst), 1);
    }
}
