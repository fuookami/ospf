//! 统一求解参数对象
//! Unified solve options object

use super::column_generation_solver::{
    CombinatorialFallbackPolicy, RegistrationStatusCallback, SolvingStatusCallback,
};
use ospf_rust_core::error::CoreError;
use ospf_rust_core::model::ModelBuildingStatusCallback;
use ospf_rust_core::solver::{
    AuditFingerprint, CancellationOrigin, SolveHandle, SolveOptions as CoreSolveOptions,
    SolveProgressReporter, SolveValueConversionPolicy,
    SolvingStatusCallback as CoreSolvingStatusCallback,
};
use std::time::Duration;

/// 统一求解参数 / Unified solve options
#[derive(Clone)]
pub struct FrameworkSolveOptions {
    /// 自定义求解名称 / Custom solve name
    pub name: Option<String>,
    /// 是否记录模型 / Whether to log model
    pub to_log_model: bool,
    /// 期望的解数量（多解接口）/ Expected solution amount (for multi-solution APIs)
    pub solution_amount: usize,
    /// 单次求解时间上限 / Per-solve time limit.
    pub time_limit: Option<Duration>,
    /// 单次求解节点上限 / Per-solve node limit.
    pub node_limit: Option<usize>,
    /// 单次求解可行解数量上限 / Per-solve feasible-solution limit.
    pub solution_limit: Option<usize>,
    /// 建模状态回调 / Model-building status callback
    pub model_building_status_callback: Option<ModelBuildingStatusCallback>,
    /// 注册状态回调 / Registration status callback
    pub registration_status_callback: Option<RegistrationStatusCallback>,
    /// 求解状态回调 / Solving status callback
    pub solving_status_callback: Option<SolvingStatusCallback>,
    /// Benders 最大迭代次数 / Benders max iterations
    pub max_iterations: usize,
    /// Benders 收敛容忍度 / Benders convergence tolerance
    pub tolerance: f64,
    /// Benders 停滞窗口（连续无新 cut 的最大轮数）/
    /// Benders stall window (max consecutive iterations without new cuts)
    pub max_stall_iterations: Option<usize>,
    /// Benders 目标改进停滞窗口（连续改进低于 tolerance 的最大轮数）/
    /// Benders objective stall window (max consecutive iterations with improvement below tolerance)
    pub objective_stall_iterations: Option<usize>,
    /// Exact Logic-Based Benders 预期的 CP snapshot 指纹 / Expected CP snapshot fingerprint for Exact Logic-Based Benders.
    ///
    /// Exact 求解不得从第一条 subproblem 结果自举 snapshot 身份；调用方必须显式提供该指纹。
    /// Exact solves must not bootstrap snapshot identity from the first subproblem result; callers
    /// must provide this fingerprint explicitly.
    pub expected_cp_snapshot_fingerprint: Option<AuditFingerprint>,
    /// 数值转换策略 / Numeric conversion policy
    pub value_conversion_policy: SolveValueConversionPolicy,
    /// 独立取消句柄 / Independent cancellation handle
    pub cancellation_handle: Option<SolveHandle>,
    /// 统一进度上报器 / Unified progress reporter
    pub progress_reporter: Option<SolveProgressReporter>,
    /// 串行组合 fallback 策略 / Serial combinatorial fallback policy
    pub fallback_policy: CombinatorialFallbackPolicy,
}

impl Default for FrameworkSolveOptions {
    fn default() -> Self {
        Self {
            name: None,
            to_log_model: false,
            solution_amount: 1,
            time_limit: None,
            node_limit: None,
            solution_limit: None,
            model_building_status_callback: None,
            registration_status_callback: None,
            solving_status_callback: None,
            max_iterations: 100,
            tolerance: 1e-9,
            max_stall_iterations: None,
            objective_stall_iterations: Some(1),
            expected_cp_snapshot_fingerprint: None,
            value_conversion_policy: SolveValueConversionPolicy::Strict,
            cancellation_handle: None,
            progress_reporter: None,
            fallback_policy: CombinatorialFallbackPolicy::default(),
        }
    }
}

impl FrameworkSolveOptions {
    /// 创建默认参数 / Create default options
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置求解名称 / Set solve name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// 设置模型日志开关 / Set model logging flag
    pub fn with_log_model(mut self, to_log_model: bool) -> Self {
        self.to_log_model = to_log_model;
        self
    }

    /// 设置解数量 / Set solution amount
    pub fn with_solution_amount(mut self, solution_amount: usize) -> Self {
        self.solution_amount = solution_amount.max(1);
        self
    }

    /// 设置单次求解时间上限 / Set the per-solve time limit.
    pub fn with_time_limit(mut self, limit: Option<Duration>) -> Self {
        self.time_limit = limit;
        self
    }

    /// 设置单次求解节点上限 / Set the per-solve node limit.
    pub fn with_node_limit(mut self, limit: Option<usize>) -> Self {
        self.node_limit = limit;
        self
    }

    /// 设置单次求解可行解数量上限 / Set the per-solve feasible-solution limit.
    pub fn with_solution_limit(mut self, limit: Option<usize>) -> Self {
        self.solution_limit = limit;
        self
    }

    /// 设置建模回调 / Set model-building callback
    pub fn with_building_callback(mut self, callback: Option<ModelBuildingStatusCallback>) -> Self {
        self.model_building_status_callback = callback;
        self
    }

    /// 设置注册回调 / Set registration callback
    pub fn with_registration_callback(
        mut self,
        callback: Option<RegistrationStatusCallback>,
    ) -> Self {
        self.registration_status_callback = callback;
        self
    }

    /// 设置求解回调 / Set solving callback
    pub fn with_solving_callback(mut self, callback: Option<SolvingStatusCallback>) -> Self {
        self.solving_status_callback = callback;
        self
    }

    /// 设置 Benders 迭代参数 / Set Benders iteration options
    pub fn with_iterations(mut self, max_iterations: usize, tolerance: f64) -> Self {
        self.max_iterations = max_iterations;
        self.tolerance = tolerance;
        self
    }

    /// 设置 Benders 最大迭代次数（Kotlin 概念别名）/
    /// Set Benders max iterations (Kotlin-concept alias)
    pub fn with_benders_iteration_limit(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// 设置 Benders 停滞窗口 / Set Benders stall window
    pub fn with_stall_iterations(mut self, max_stall_iterations: usize) -> Self {
        self.max_stall_iterations = Some(max_stall_iterations.max(1));
        self
    }

    /// 设置 Exact CP snapshot 预期身份 / Set the expected Exact CP snapshot identity.
    pub fn with_expected_cp_snapshot_fingerprint(mut self, fingerprint: AuditFingerprint) -> Self {
        self.expected_cp_snapshot_fingerprint = Some(fingerprint);
        self
    }

    /// 设置 Benders 停滞窗口（Kotlin 概念别名）/
    /// Set Benders stall window (Kotlin-concept alias)
    pub fn with_benders_stall_iteration_limit(self, max_stall_iterations: usize) -> Self {
        self.with_stall_iterations(max_stall_iterations)
    }

    /// 设置 Benders 目标改进停滞窗口 / Set Benders objective stall window
    pub fn with_objective_stall_iterations(mut self, objective_stall_iterations: usize) -> Self {
        self.objective_stall_iterations = Some(objective_stall_iterations.max(1));
        self
    }

    /// 设置数值转换策略 / Set numeric conversion policy
    pub fn with_value_conversion_policy(
        mut self,
        value_conversion_policy: SolveValueConversionPolicy,
    ) -> Self {
        self.value_conversion_policy = value_conversion_policy;
        self
    }

    /// 设置取消句柄 / Set cancellation handle
    pub fn with_cancellation_handle(mut self, handle: Option<SolveHandle>) -> Self {
        self.cancellation_handle = handle;
        self
    }

    /// 设置统一进度上报器 / Set the unified progress reporter
    pub fn with_progress_reporter(mut self, reporter: Option<SolveProgressReporter>) -> Self {
        self.progress_reporter = reporter;
        self
    }

    /// 设置组合 fallback 策略 / Set the combinatorial fallback policy
    pub fn with_fallback_policy(mut self, fallback_policy: CombinatorialFallbackPolicy) -> Self {
        self.fallback_policy = fallback_policy;
        self
    }

    /// 转换为 core 层参数对象 / Convert into core solve options
    pub fn to_core_solve_options<'a>(
        &'a self,
        solving_status_callback: Option<&'a CoreSolvingStatusCallback>,
    ) -> CoreSolveOptions<'a> {
        CoreSolveOptions::new()
            .with_solution_amount(self.solution_amount)
            .with_time_limit(self.time_limit)
            .with_node_limit(self.node_limit)
            .with_solution_limit(self.solution_limit)
            .with_value_conversion_policy(self.value_conversion_policy)
            .with_solving_callback(solving_status_callback)
            .with_cancellation_handle(self.cancellation_handle.as_ref())
            .with_progress_reporter(self.progress_reporter.as_ref())
    }
}

/// 为并行 attempt 创建独立取消句柄，并继承外部取消请求。
/// Create an independent cancellation handle for a parallel attempt and inherit external cancel.
pub(crate) fn child_cancellation_handle(options: &FrameworkSolveOptions) -> SolveHandle {
    let child = SolveHandle::new();
    let Some(parent) = options.cancellation_handle.as_ref() else {
        return child;
    };
    let child_for_parent = child.clone();
    let parent_for_callback = parent.clone();
    parent.register_interrupter(move || {
        let origin = parent_for_callback
            .cancellation()
            .map(|record| record.origin)
            .unwrap_or(CancellationOrigin::External);
        child_for_parent.cancel(origin);
    });
    child
}

/// 兼容别名：旧命名 `SolveOptions`
/// Compatibility alias: legacy name `SolveOptions`.
pub type SolveOptions = FrameworkSolveOptions;

pub(crate) fn legacy_cancellation_error(options: &FrameworkSolveOptions) -> CoreError {
    let origin = options
        .cancellation_handle
        .as_ref()
        .and_then(|handle| handle.cancellation())
        .map(|record| record.origin.to_string())
        .unwrap_or_else(|| "UNKNOWN".to_owned());
    CoreError::cancelled(origin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn to_core_solve_options_preserves_value_conversion_policy() {
        let options = FrameworkSolveOptions::new()
            .with_value_conversion_policy(SolveValueConversionPolicy::AllowRounding);
        let core_options = options.to_core_solve_options(None);

        assert_eq!(
            core_options.value_conversion_policy,
            SolveValueConversionPolicy::AllowRounding
        );
    }

    #[test]
    fn to_core_solve_options_preserves_solution_amount() {
        let options = FrameworkSolveOptions::new().with_solution_amount(5);
        let core_options = options.to_core_solve_options(None);

        assert_eq!(core_options.solution_amount, 5);
    }

    #[test]
    fn benders_aliases_preserve_existing_builder_semantics() {
        let options = FrameworkSolveOptions::new()
            .with_iterations(12, 1e-5)
            .with_benders_iteration_limit(7)
            .with_benders_stall_iteration_limit(0);

        assert_eq!(options.max_iterations, 7);
        assert_eq!(options.tolerance, 1e-5);
        assert_eq!(options.max_stall_iterations, Some(1));
    }

    #[test]
    fn to_core_solve_options_forwards_solving_callback_reference() {
        let callback: CoreSolvingStatusCallback = Arc::new(|_status| Ok(()));
        let options = FrameworkSolveOptions::new();
        let core_options = options.to_core_solve_options(Some(&callback));

        assert!(core_options.solving_status_callback.is_some());
    }

    #[test]
    fn child_cancellation_handles_propagate_external_origin_independently() {
        let parent = SolveHandle::new();
        let options = FrameworkSolveOptions::new().with_cancellation_handle(Some(parent.clone()));
        let first = child_cancellation_handle(&options);
        let second = child_cancellation_handle(&options);

        assert!(parent.cancel(CancellationOrigin::External));
        assert_eq!(
            first.cancellation().map(|record| record.origin),
            Some(CancellationOrigin::External)
        );
        assert_eq!(
            second.cancellation().map(|record| record.origin),
            Some(CancellationOrigin::External)
        );

        assert!(!first.cancel(CancellationOrigin::FrameworkLoser));
        assert_eq!(
            second.cancellation().map(|record| record.origin),
            Some(CancellationOrigin::External)
        );
        assert_eq!(
            parent.cancellation().map(|record| record.origin),
            Some(CancellationOrigin::External)
        );
    }

    #[test]
    fn child_created_after_parent_cancel_is_cancelled_without_losing_origin() {
        let parent = SolveHandle::new();
        assert!(parent.cancel(CancellationOrigin::RemoteStop));
        let options = FrameworkSolveOptions::new().with_cancellation_handle(Some(parent));
        let child = child_cancellation_handle(&options);

        assert!(child.is_cancelled());
        assert_eq!(
            child.cancellation().map(|record| record.origin),
            Some(CancellationOrigin::RemoteStop)
        );
    }

    #[test]
    fn legacy_cancellation_is_a_terminal_projection_not_backend_failure() {
        let handle = SolveHandle::new();
        assert!(handle.cancel(CancellationOrigin::User));
        let options = FrameworkSolveOptions::new().with_cancellation_handle(Some(handle));

        let error = legacy_cancellation_error(&options);

        assert!(error.is_normal_terminal());
        assert_eq!(error.solver_error_class_code(), "TERMINAL_PROJECTION");
        assert!(error.to_string().contains("USER"));
    }
}
