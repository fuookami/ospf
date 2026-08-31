//! Gurobi 扩展求解器接口实现
//! Gurobi extension solver interfaces
//!
//! 提供 framework 层对 Gurobi 的列生成与 Benders 分解适配器。
//! Provides framework-level column-generation and Benders adapters for Gurobi.
//!
//! 原生回调语义说明 / Native callback semantics:
//! - `with_native_callback` 与 `add_native_callback` 均为覆盖语义。
//! - 这是对 core `GurobiConfig` 语义的直接透传。
//! - This directly forwards core `GurobiConfig` semantics.
//! - 如需多个 handler，请在单个 native callback 内部分发。
use std::sync::Arc;

use ospf_rust_core::model::mechanism::MechanismModel;
use ospf_rust_core::solver::solvers::GurobiSolver as CoreGurobiSolver;
use ospf_rust_core::solver::solvers::gurobi::{
    GurobiConfig, GurobiEnvCallback, GurobiNativeCallback, GurobiNativeControl,
    GurobiNativeObserver, GurobiNumericDiagnosticsCallback, GurobiStage, GurobiStageCallback,
    GurobiTelemetryCallback,
};
use ospf_rust_core::variable::VariableId;

use super::FrameworkSolveOptions;
use super::column_generation_solver::{ColumnGenerationSolver, FeasibleSolution, LPResult};
use super::core_extensions::{
    BendersCutContext, CoreColumnGenerationAdapter, CoreLinearBendersAdapter,
    CoreQuadraticBendersAdapter,
};
use super::linear_benders_decomposition_solver::{
    LinearBendersDecompositionSolver, LinearCut, LinearSubResult,
};
use super::quadratic_benders_decomposition_solver::{
    QuadraticBendersDecompositionSolver, QuadraticCut, QuadraticSubResult,
};

/// Gurobi 列生成求解器 / Gurobi column generation solver
#[derive(Debug)]
pub struct GurobiColumnGenerationSolver {
    inner: CoreColumnGenerationAdapter<CoreGurobiSolver>,
}

impl Default for GurobiColumnGenerationSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl GurobiColumnGenerationSolver {
    fn map_config(mut self, mapper: impl FnOnce(GurobiConfig) -> GurobiConfig) -> Self {
        let current = self.inner.solver().config().clone();
        *self.inner.solver_mut().config_mut() = mapper(current);
        self
    }

    /// 创建默认求解器 / Create default solver
    pub fn new() -> Self {
        Self::with_solver(CoreGurobiSolver::new())
    }

    /// 使用配置创建求解器 / Create solver with configuration
    pub fn with_config(config: GurobiConfig) -> Self {
        Self::with_solver(CoreGurobiSolver::with_config(config))
    }

    /// 使用指定 core 求解器创建 / Create from explicit core solver
    pub fn with_solver(solver: CoreGurobiSolver) -> Self {
        Self {
            inner: CoreColumnGenerationAdapter::new("gurobi", solver),
        }
    }

    /// 获取底层 core 求解器 / Get underlying core solver
    pub fn solver(&self) -> &CoreGurobiSolver {
        self.inner.solver()
    }

    /// 设置求解 gap / Set solve gap
    pub fn with_gap(self, gap: f64) -> Self {
        self.map_config(|config| config.with_gap(gap))
    }

    /// 设置内存限制（MB）/ Set memory limit (MB)
    pub fn with_memory_limit_mb(self, memory_limit_mb: f64) -> Self {
        self.map_config(|config| config.with_memory_limit_mb(memory_limit_mb))
    }

    /// 设置内存限制（GB）/ Set memory limit (GB)
    pub fn with_memory_limit_gb(self, memory_limit_gb: f64) -> Self {
        self.map_config(|config| config.with_memory_limit_gb(memory_limit_gb))
    }

    /// 设置改进判定阈值 / Set improvement threshold
    pub fn with_improve_threshold(self, threshold: f64) -> Self {
        self.map_config(|config| config.with_improve_threshold(threshold))
    }

    /// 设置遥测回调 / Set telemetry callback
    pub fn with_telemetry_callback(self, callback: Option<GurobiTelemetryCallback>) -> Self {
        self.map_config(|config| config.with_telemetry_callback(callback))
    }

    /// 追加遥测回调 / Append telemetry callback
    pub fn add_telemetry_callback(mut self, callback: GurobiTelemetryCallback) -> Self {
        let config = self.inner.solver_mut().config_mut();
        config.telemetry_callback = Some(match config.telemetry_callback.take() {
            Some(existing) => Arc::new(move |status| {
                existing(status)?;
                callback(status)
            }),
            None => callback,
        });
        self
    }

    /// 设置遥测最小上报间隔（秒） / Set minimum telemetry emit interval (seconds)
    pub fn with_telemetry_min_interval(self, seconds: f64) -> Self {
        self.map_config(|config| config.with_telemetry_min_interval(seconds))
    }

    pub fn with_numeric_diagnostics(self, enabled: bool) -> Self {
        self.map_config(|config| config.with_numeric_diagnostics(enabled))
    }

    pub fn with_auto_apply_numeric_profile(self, enabled: bool) -> Self {
        self.map_config(|config| config.with_auto_apply_numeric_profile(enabled))
    }

    pub fn with_numeric_diagnostics_callback(
        self,
        callback: Option<GurobiNumericDiagnosticsCallback>,
    ) -> Self {
        self.map_config(|config| config.with_numeric_diagnostics_callback(callback))
    }

    pub fn add_numeric_diagnostics_callback(
        mut self,
        callback: GurobiNumericDiagnosticsCallback,
    ) -> Self {
        let config = self.inner.solver_mut().config_mut();
        config.numeric_diagnostics_callback =
            Some(match config.numeric_diagnostics_callback.take() {
                Some(existing) => Arc::new(move |diagnostics| {
                    existing(diagnostics)?;
                    callback(diagnostics)
                }),
                None => callback,
            });
        self
    }

    /// 应用数值稳健优先模板 / Apply robustness-first profile
    pub fn with_robust_defaults(self) -> Self {
        self.map_config(GurobiConfig::with_robust_defaults)
    }

    /// 应用性能优先模板 / Apply performance-first profile
    pub fn with_performance_defaults(self) -> Self {
        self.map_config(GurobiConfig::with_performance_defaults)
    }

    /// 应用均衡模板 / Apply balanced profile
    pub fn with_balanced_defaults(self) -> Self {
        self.map_config(GurobiConfig::with_balanced_defaults)
    }

    /// 设置数值稳定性关注级别 / Set numeric focus level
    pub fn with_numeric_focus(self, level: i32) -> Self {
        self.map_config(|config| config.with_numeric_focus(level))
    }

    /// 设置缩放策略 / Set scaling strategy
    pub fn with_scale_flag(self, flag: i32) -> Self {
        self.map_config(|config| config.with_scale_flag(flag))
    }

    /// 设置分阶段回调 / Set staged callback
    pub fn with_stage_callback(self, callback: Option<GurobiStageCallback>) -> Self {
        self.map_config(|config| config.with_stage_callback(callback))
    }

    /// 追加分阶段回调 / Append staged callback
    pub fn add_stage_callback(mut self, callback: GurobiStageCallback) -> Self {
        let config = self.inner.solver_mut().config_mut();
        config.stage_callback = Some(match config.stage_callback.take() {
            Some(existing) => Arc::new(move |status| {
                existing(status)?;
                callback(status)
            }),
            None => callback,
        });
        self
    }

    /// 追加指定阶段回调 / Append staged callback for specific stage
    pub fn add_stage_callback_for(
        mut self,
        stage: GurobiStage,
        callback: GurobiStageCallback,
    ) -> Self {
        let config = self.inner.solver_mut().config_mut();
        config.stage_callback = Some(match config.stage_callback.take() {
            Some(existing) => Arc::new(move |status| {
                existing(status)?;
                if status.stage == stage {
                    callback(status)?;
                }
                Ok(())
            }),
            None => Arc::new(move |status| {
                if status.stage == stage {
                    callback(status)?;
                }
                Ok(())
            }),
        });
        self
    }

    /// 追加建模后回调 / Append callback for `AfterModeling`
    pub fn add_after_modeling_callback(self, callback: GurobiStageCallback) -> Self {
        self.add_stage_callback_for(GurobiStage::AfterModeling, callback)
    }

    /// 追加配置后回调 / Append callback for `Configuration`
    pub fn add_configuration_callback(self, callback: GurobiStageCallback) -> Self {
        self.add_stage_callback_for(GurobiStage::Configuration, callback)
    }

    /// 追加解分析回调 / Append callback for `AnalyzingSolution`
    pub fn add_analyzing_solution_callback(self, callback: GurobiStageCallback) -> Self {
        self.add_stage_callback_for(GurobiStage::AnalyzingSolution, callback)
    }

    /// 追加失败后回调 / Append callback for `AfterFailure`
    pub fn add_after_failure_callback(self, callback: GurobiStageCallback) -> Self {
        self.add_stage_callback_for(GurobiStage::AfterFailure, callback)
    }

    /// 设置原生回调 / Set native callback
    pub fn with_native_callback(self, callback: Option<GurobiNativeCallback>) -> Self {
        self.map_config(|config| config.with_native_callback(callback))
    }

    /// 追加原生回调（覆盖语义） / Append native callback (override semantics)
    pub fn add_native_callback(mut self, callback: GurobiNativeCallback) -> Self {
        self.inner.solver_mut().config_mut().native_callback = Some(callback);
        self
    }

    pub fn with_native_observers(self, observers: Vec<GurobiNativeObserver>) -> Self {
        self.map_config(|config| config.with_native_observers(observers))
    }

    pub fn add_native_observer(mut self, observer: GurobiNativeObserver) -> Self {
        self.inner
            .solver_mut()
            .config_mut()
            .native_observers
            .push(observer);
        self
    }

    /// 设置环境创建回调 / Set environment creation callback
    pub fn with_env_callback(self, callback: Option<GurobiEnvCallback>) -> Self {
        self.map_config(|config| config.with_env_callback(callback))
    }

    /// 追加环境创建回调 / Append environment creation callback
    pub fn add_env_callback(mut self, callback: GurobiEnvCallback) -> Self {
        let config = self.inner.solver_mut().config_mut();
        config.env_callback = Some(match config.env_callback.take() {
            Some(existing) => Arc::new(move |env| {
                existing(env)?;
                callback(env)
            }),
            None => callback,
        });
        self
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl ColumnGenerationSolver for GurobiColumnGenerationSolver {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn solve_milp_with_options(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> ospf_rust_core::error::Result<FeasibleSolution> {
        self.inner.solve_milp_with_options(model, options).await
    }

    async fn solve_lp_with_options(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> ospf_rust_core::error::Result<LPResult> {
        self.inner.solve_lp_with_options(model, options).await
    }
}

#[cfg(not(feature = "async"))]
impl ColumnGenerationSolver for GurobiColumnGenerationSolver {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn solve_milp_with_options(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> ospf_rust_core::error::Result<FeasibleSolution> {
        self.inner.solve_milp_with_options(model, options)
    }

    fn solve_lp_with_options(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        options: FrameworkSolveOptions,
    ) -> ospf_rust_core::error::Result<LPResult> {
        self.inner.solve_lp_with_options(model, options)
    }
}

/// Gurobi 线性 Benders 分解求解器 / Gurobi linear Benders decomposition solver
#[derive(Debug)]
pub struct GurobiLinearBendersDecompositionSolver {
    inner: CoreLinearBendersAdapter<CoreGurobiSolver>,
}

impl Default for GurobiLinearBendersDecompositionSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl GurobiLinearBendersDecompositionSolver {
    fn map_config(mut self, mapper: impl FnOnce(GurobiConfig) -> GurobiConfig) -> Self {
        let current = self.inner.solver().config().clone();
        *self.inner.solver_mut().config_mut() = mapper(current);
        self
    }

    /// 创建默认求解器 / Create default solver
    pub fn new() -> Self {
        Self::with_solver(CoreGurobiSolver::new())
    }

    /// 使用配置创建求解器 / Create solver with configuration
    pub fn with_config(config: GurobiConfig) -> Self {
        Self::with_solver(CoreGurobiSolver::with_config(config))
    }

    /// 使用指定 core 求解器创建 / Create from explicit core solver
    pub fn with_solver(solver: CoreGurobiSolver) -> Self {
        Self {
            inner: CoreLinearBendersAdapter::new("gurobi", solver),
        }
    }

    /// 配置割生成上下文 / Configure cut-generation context
    pub fn with_cut_context(
        self,
        mechanism_model: MechanismModel<f64>,
        objective_variable: Option<VariableId>,
        fixed_variable_ids: Vec<VariableId>,
    ) -> Self {
        Self {
            inner: self.inner.with_cut_context(BendersCutContext {
                mechanism_model,
                objective_variable,
                fixed_variable_ids,
            }),
        }
    }

    /// 获取底层 core 求解器 / Get underlying core solver
    pub fn solver(&self) -> &CoreGurobiSolver {
        self.inner.solver()
    }

    /// 设置求解 gap / Set solve gap
    pub fn with_gap(self, gap: f64) -> Self {
        self.map_config(|config| config.with_gap(gap))
    }

    /// 设置内存限制（MB）/ Set memory limit (MB)
    pub fn with_memory_limit_mb(self, memory_limit_mb: f64) -> Self {
        self.map_config(|config| config.with_memory_limit_mb(memory_limit_mb))
    }

    /// 设置内存限制（GB）/ Set memory limit (GB)
    pub fn with_memory_limit_gb(self, memory_limit_gb: f64) -> Self {
        self.map_config(|config| config.with_memory_limit_gb(memory_limit_gb))
    }

    /// 设置改进判定阈值 / Set improvement threshold
    pub fn with_improve_threshold(self, threshold: f64) -> Self {
        self.map_config(|config| config.with_improve_threshold(threshold))
    }

    /// 设置遥测回调 / Set telemetry callback
    pub fn with_telemetry_callback(self, callback: Option<GurobiTelemetryCallback>) -> Self {
        self.map_config(|config| config.with_telemetry_callback(callback))
    }

    /// 追加遥测回调 / Append telemetry callback
    pub fn add_telemetry_callback(mut self, callback: GurobiTelemetryCallback) -> Self {
        let config = self.inner.solver_mut().config_mut();
        config.telemetry_callback = Some(match config.telemetry_callback.take() {
            Some(existing) => Arc::new(move |status| {
                existing(status)?;
                callback(status)
            }),
            None => callback,
        });
        self
    }

    /// 设置遥测最小上报间隔（秒） / Set minimum telemetry emit interval (seconds)
    pub fn with_telemetry_min_interval(self, seconds: f64) -> Self {
        self.map_config(|config| config.with_telemetry_min_interval(seconds))
    }

    pub fn with_numeric_diagnostics(self, enabled: bool) -> Self {
        self.map_config(|config| config.with_numeric_diagnostics(enabled))
    }

    pub fn with_auto_apply_numeric_profile(self, enabled: bool) -> Self {
        self.map_config(|config| config.with_auto_apply_numeric_profile(enabled))
    }

    pub fn with_numeric_diagnostics_callback(
        self,
        callback: Option<GurobiNumericDiagnosticsCallback>,
    ) -> Self {
        self.map_config(|config| config.with_numeric_diagnostics_callback(callback))
    }

    pub fn add_numeric_diagnostics_callback(
        mut self,
        callback: GurobiNumericDiagnosticsCallback,
    ) -> Self {
        let config = self.inner.solver_mut().config_mut();
        config.numeric_diagnostics_callback =
            Some(match config.numeric_diagnostics_callback.take() {
                Some(existing) => Arc::new(move |diagnostics| {
                    existing(diagnostics)?;
                    callback(diagnostics)
                }),
                None => callback,
            });
        self
    }

    /// 应用数值稳健优先模板 / Apply robustness-first profile
    pub fn with_robust_defaults(self) -> Self {
        self.map_config(GurobiConfig::with_robust_defaults)
    }

    /// 应用性能优先模板 / Apply performance-first profile
    pub fn with_performance_defaults(self) -> Self {
        self.map_config(GurobiConfig::with_performance_defaults)
    }

    /// 应用均衡模板 / Apply balanced profile
    pub fn with_balanced_defaults(self) -> Self {
        self.map_config(GurobiConfig::with_balanced_defaults)
    }

    /// 设置数值稳定性关注级别 / Set numeric focus level
    pub fn with_numeric_focus(self, level: i32) -> Self {
        self.map_config(|config| config.with_numeric_focus(level))
    }

    /// 设置缩放策略 / Set scaling strategy
    pub fn with_scale_flag(self, flag: i32) -> Self {
        self.map_config(|config| config.with_scale_flag(flag))
    }

    /// 设置分阶段回调 / Set staged callback
    pub fn with_stage_callback(self, callback: Option<GurobiStageCallback>) -> Self {
        self.map_config(|config| config.with_stage_callback(callback))
    }

    /// 追加分阶段回调 / Append staged callback
    pub fn add_stage_callback(mut self, callback: GurobiStageCallback) -> Self {
        let config = self.inner.solver_mut().config_mut();
        config.stage_callback = Some(match config.stage_callback.take() {
            Some(existing) => Arc::new(move |status| {
                existing(status)?;
                callback(status)
            }),
            None => callback,
        });
        self
    }

    /// 追加指定阶段回调 / Append staged callback for specific stage
    pub fn add_stage_callback_for(
        mut self,
        stage: GurobiStage,
        callback: GurobiStageCallback,
    ) -> Self {
        let config = self.inner.solver_mut().config_mut();
        config.stage_callback = Some(match config.stage_callback.take() {
            Some(existing) => Arc::new(move |status| {
                existing(status)?;
                if status.stage == stage {
                    callback(status)?;
                }
                Ok(())
            }),
            None => Arc::new(move |status| {
                if status.stage == stage {
                    callback(status)?;
                }
                Ok(())
            }),
        });
        self
    }

    /// 追加建模后回调 / Append callback for `AfterModeling`
    pub fn add_after_modeling_callback(self, callback: GurobiStageCallback) -> Self {
        self.add_stage_callback_for(GurobiStage::AfterModeling, callback)
    }

    /// 追加配置后回调 / Append callback for `Configuration`
    pub fn add_configuration_callback(self, callback: GurobiStageCallback) -> Self {
        self.add_stage_callback_for(GurobiStage::Configuration, callback)
    }

    /// 追加解分析回调 / Append callback for `AnalyzingSolution`
    pub fn add_analyzing_solution_callback(self, callback: GurobiStageCallback) -> Self {
        self.add_stage_callback_for(GurobiStage::AnalyzingSolution, callback)
    }

    /// 追加失败后回调 / Append callback for `AfterFailure`
    pub fn add_after_failure_callback(self, callback: GurobiStageCallback) -> Self {
        self.add_stage_callback_for(GurobiStage::AfterFailure, callback)
    }

    /// 设置原生回调 / Set native callback
    pub fn with_native_callback(self, callback: Option<GurobiNativeCallback>) -> Self {
        self.map_config(|config| config.with_native_callback(callback))
    }

    /// 追加原生回调（覆盖语义） / Append native callback (override semantics)
    pub fn add_native_callback(mut self, callback: GurobiNativeCallback) -> Self {
        self.inner.solver_mut().config_mut().native_callback = Some(callback);
        self
    }

    pub fn with_native_observers(self, observers: Vec<GurobiNativeObserver>) -> Self {
        self.map_config(|config| config.with_native_observers(observers))
    }

    pub fn add_native_observer(mut self, observer: GurobiNativeObserver) -> Self {
        self.inner
            .solver_mut()
            .config_mut()
            .native_observers
            .push(observer);
        self
    }

    /// 设置环境创建回调 / Set environment creation callback
    pub fn with_env_callback(self, callback: Option<GurobiEnvCallback>) -> Self {
        self.map_config(|config| config.with_env_callback(callback))
    }

    /// 追加环境创建回调 / Append environment creation callback
    pub fn add_env_callback(mut self, callback: GurobiEnvCallback) -> Self {
        let config = self.inner.solver_mut().config_mut();
        config.env_callback = Some(match config.env_callback.take() {
            Some(existing) => Arc::new(move |env| {
                existing(env)?;
                callback(env)
            }),
            None => callback,
        });
        self
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl LinearBendersDecompositionSolver for GurobiLinearBendersDecompositionSolver {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn solve_master(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        cuts: &[LinearCut],
    ) -> ospf_rust_core::error::Result<ospf_rust_core::solver::SolverOutput> {
        self.inner.solve_master(model, cuts).await
    }

    async fn solve_sub(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        master_solution: &[f64],
    ) -> ospf_rust_core::error::Result<LinearSubResult> {
        self.inner.solve_sub(model, master_solution).await
    }
}

#[cfg(not(feature = "async"))]
impl LinearBendersDecompositionSolver for GurobiLinearBendersDecompositionSolver {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn solve_master(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        cuts: &[LinearCut],
    ) -> ospf_rust_core::error::Result<ospf_rust_core::solver::SolverOutput> {
        self.inner.solve_master(model, cuts)
    }

    fn solve_sub(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        master_solution: &[f64],
    ) -> ospf_rust_core::error::Result<LinearSubResult> {
        self.inner.solve_sub(model, master_solution)
    }
}

/// Gurobi Benders 分解求解器（线性 + 二次）
/// Gurobi Benders decomposition solver (linear + quadratic)
#[derive(Debug)]
pub struct GurobiBendersDecompositionSolver {
    linear: GurobiLinearBendersDecompositionSolver,
    quadratic: CoreQuadraticBendersAdapter<CoreGurobiSolver>,
}

impl Default for GurobiBendersDecompositionSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl GurobiBendersDecompositionSolver {
    fn map_configs(mut self, mapper: impl Fn(GurobiConfig) -> GurobiConfig) -> Self {
        let linear_current = self.linear.inner.solver().config().clone();
        *self.linear.inner.solver_mut().config_mut() = mapper(linear_current);
        let quadratic_current = self.quadratic.solver().config().clone();
        *self.quadratic.solver_mut().config_mut() = mapper(quadratic_current);
        self
    }

    /// 创建默认求解器 / Create default solver
    pub fn new() -> Self {
        Self {
            linear: GurobiLinearBendersDecompositionSolver::new(),
            quadratic: CoreQuadraticBendersAdapter::new("gurobi", CoreGurobiSolver::new()),
        }
    }

    /// 使用配置创建求解器 / Create solver with configuration
    pub fn with_config(config: GurobiConfig) -> Self {
        Self {
            linear: GurobiLinearBendersDecompositionSolver::with_config(config.clone()),
            quadratic: CoreQuadraticBendersAdapter::new(
                "gurobi",
                CoreGurobiSolver::with_config(config),
            ),
        }
    }

    /// 配置割生成上下文 / Configure cut-generation context
    pub fn with_cut_context(
        self,
        mechanism_model: MechanismModel<f64>,
        objective_variable: Option<VariableId>,
        fixed_variable_ids: Vec<VariableId>,
    ) -> Self {
        let context = BendersCutContext {
            mechanism_model,
            objective_variable,
            fixed_variable_ids,
        };
        Self {
            linear: self.linear.with_cut_context(
                context.mechanism_model.clone(),
                context.objective_variable,
                context.fixed_variable_ids.clone(),
            ),
            quadratic: self.quadratic.with_cut_context(context),
        }
    }

    /// 设置求解 gap / Set solve gap
    pub fn with_gap(self, gap: f64) -> Self {
        self.map_configs(|config| config.with_gap(gap))
    }

    /// 设置内存限制（MB）/ Set memory limit (MB)
    pub fn with_memory_limit_mb(self, memory_limit_mb: f64) -> Self {
        self.map_configs(|config| config.with_memory_limit_mb(memory_limit_mb))
    }

    /// 设置内存限制（GB）/ Set memory limit (GB)
    pub fn with_memory_limit_gb(self, memory_limit_gb: f64) -> Self {
        self.map_configs(|config| config.with_memory_limit_gb(memory_limit_gb))
    }

    /// 设置改进判定阈值 / Set improvement threshold
    pub fn with_improve_threshold(self, threshold: f64) -> Self {
        self.map_configs(|config| config.with_improve_threshold(threshold))
    }

    /// 设置遥测回调 / Set telemetry callback
    pub fn with_telemetry_callback(self, callback: Option<GurobiTelemetryCallback>) -> Self {
        self.map_configs(|config| config.with_telemetry_callback(callback.clone()))
    }

    /// 追加遥测回调 / Append telemetry callback
    pub fn add_telemetry_callback(mut self, callback: GurobiTelemetryCallback) -> Self {
        let linear = self.linear.inner.solver_mut().config_mut();
        linear.telemetry_callback = Some(match linear.telemetry_callback.take() {
            Some(existing) => {
                let callback = callback.clone();
                Arc::new(move |status| {
                    existing(status)?;
                    callback(status)
                })
            }
            None => callback.clone(),
        });
        let quadratic = self.quadratic.solver_mut().config_mut();
        quadratic.telemetry_callback = Some(match quadratic.telemetry_callback.take() {
            Some(existing) => Arc::new(move |status| {
                existing(status)?;
                callback(status)
            }),
            None => callback,
        });
        self
    }

    /// 设置遥测最小上报间隔（秒） / Set minimum telemetry emit interval (seconds)
    pub fn with_telemetry_min_interval(self, seconds: f64) -> Self {
        self.map_configs(|config| config.with_telemetry_min_interval(seconds))
    }

    pub fn with_numeric_diagnostics(self, enabled: bool) -> Self {
        self.map_configs(|config| config.with_numeric_diagnostics(enabled))
    }

    pub fn with_auto_apply_numeric_profile(self, enabled: bool) -> Self {
        self.map_configs(|config| config.with_auto_apply_numeric_profile(enabled))
    }

    pub fn with_numeric_diagnostics_callback(
        self,
        callback: Option<GurobiNumericDiagnosticsCallback>,
    ) -> Self {
        self.map_configs(|config| config.with_numeric_diagnostics_callback(callback.clone()))
    }

    pub fn add_numeric_diagnostics_callback(
        mut self,
        callback: GurobiNumericDiagnosticsCallback,
    ) -> Self {
        let linear = self.linear.inner.solver_mut().config_mut();
        linear.numeric_diagnostics_callback =
            Some(match linear.numeric_diagnostics_callback.take() {
                Some(existing) => {
                    let callback = callback.clone();
                    Arc::new(move |diagnostics| {
                        existing(diagnostics)?;
                        callback(diagnostics)
                    })
                }
                None => callback.clone(),
            });

        let quadratic = self.quadratic.solver_mut().config_mut();
        quadratic.numeric_diagnostics_callback =
            Some(match quadratic.numeric_diagnostics_callback.take() {
                Some(existing) => Arc::new(move |diagnostics| {
                    existing(diagnostics)?;
                    callback(diagnostics)
                }),
                None => callback,
            });
        self
    }

    /// 应用数值稳健优先模板 / Apply robustness-first profile
    pub fn with_robust_defaults(self) -> Self {
        self.map_configs(GurobiConfig::with_robust_defaults)
    }

    /// 应用性能优先模板 / Apply performance-first profile
    pub fn with_performance_defaults(self) -> Self {
        self.map_configs(GurobiConfig::with_performance_defaults)
    }

    /// 应用均衡模板 / Apply balanced profile
    pub fn with_balanced_defaults(self) -> Self {
        self.map_configs(GurobiConfig::with_balanced_defaults)
    }

    /// 设置数值稳定性关注级别 / Set numeric focus level
    pub fn with_numeric_focus(self, level: i32) -> Self {
        self.map_configs(|config| config.with_numeric_focus(level))
    }

    /// 设置缩放策略 / Set scaling strategy
    pub fn with_scale_flag(self, flag: i32) -> Self {
        self.map_configs(|config| config.with_scale_flag(flag))
    }

    /// 设置分阶段回调 / Set staged callback
    pub fn with_stage_callback(self, callback: Option<GurobiStageCallback>) -> Self {
        self.map_configs(|config| config.with_stage_callback(callback.clone()))
    }

    /// 追加分阶段回调 / Append staged callback
    pub fn add_stage_callback(mut self, callback: GurobiStageCallback) -> Self {
        let linear = self.linear.inner.solver_mut().config_mut();
        linear.stage_callback = Some(match linear.stage_callback.take() {
            Some(existing) => {
                let callback = callback.clone();
                Arc::new(move |status| {
                    existing(status)?;
                    callback(status)
                })
            }
            None => callback.clone(),
        });
        let quadratic = self.quadratic.solver_mut().config_mut();
        quadratic.stage_callback = Some(match quadratic.stage_callback.take() {
            Some(existing) => Arc::new(move |status| {
                existing(status)?;
                callback(status)
            }),
            None => callback,
        });
        self
    }

    /// 追加指定阶段回调 / Append staged callback for specific stage
    pub fn add_stage_callback_for(
        mut self,
        stage: GurobiStage,
        callback: GurobiStageCallback,
    ) -> Self {
        let linear = self.linear.inner.solver_mut().config_mut();
        linear.stage_callback = Some(match linear.stage_callback.take() {
            Some(existing) => {
                let callback = callback.clone();
                Arc::new(move |status| {
                    existing(status)?;
                    if status.stage == stage {
                        callback(status)?;
                    }
                    Ok(())
                })
            }
            None => {
                let callback = callback.clone();
                Arc::new(move |status| {
                    if status.stage == stage {
                        callback(status)?;
                    }
                    Ok(())
                })
            }
        });
        let quadratic = self.quadratic.solver_mut().config_mut();
        quadratic.stage_callback = Some(match quadratic.stage_callback.take() {
            Some(existing) => Arc::new(move |status| {
                existing(status)?;
                if status.stage == stage {
                    callback(status)?;
                }
                Ok(())
            }),
            None => Arc::new(move |status| {
                if status.stage == stage {
                    callback(status)?;
                }
                Ok(())
            }),
        });
        self
    }

    /// 追加建模后回调 / Append callback for `AfterModeling`
    pub fn add_after_modeling_callback(self, callback: GurobiStageCallback) -> Self {
        self.add_stage_callback_for(GurobiStage::AfterModeling, callback)
    }

    /// 追加配置后回调 / Append callback for `Configuration`
    pub fn add_configuration_callback(self, callback: GurobiStageCallback) -> Self {
        self.add_stage_callback_for(GurobiStage::Configuration, callback)
    }

    /// 追加解分析回调 / Append callback for `AnalyzingSolution`
    pub fn add_analyzing_solution_callback(self, callback: GurobiStageCallback) -> Self {
        self.add_stage_callback_for(GurobiStage::AnalyzingSolution, callback)
    }

    /// 追加失败后回调 / Append callback for `AfterFailure`
    pub fn add_after_failure_callback(self, callback: GurobiStageCallback) -> Self {
        self.add_stage_callback_for(GurobiStage::AfterFailure, callback)
    }

    /// 设置原生回调 / Set native callback
    pub fn with_native_callback(self, callback: Option<GurobiNativeCallback>) -> Self {
        self.map_configs(|config| config.with_native_callback(callback.clone()))
    }

    /// 追加原生回调（覆盖语义） / Append native callback (override semantics)
    pub fn add_native_callback(mut self, callback: GurobiNativeCallback) -> Self {
        self.linear.inner.solver_mut().config_mut().native_callback = Some(callback.clone());
        self.quadratic.solver_mut().config_mut().native_callback = Some(callback);
        self
    }

    pub fn with_native_observers(self, observers: Vec<GurobiNativeObserver>) -> Self {
        self.map_configs(|config| config.with_native_observers(observers.clone()))
    }

    pub fn add_native_observer(mut self, observer: GurobiNativeObserver) -> Self {
        self.linear
            .inner
            .solver_mut()
            .config_mut()
            .native_observers
            .push(observer.clone());
        self.quadratic
            .solver_mut()
            .config_mut()
            .native_observers
            .push(observer);
        self
    }

    /// 设置环境创建回调 / Set environment creation callback
    pub fn with_env_callback(self, callback: Option<GurobiEnvCallback>) -> Self {
        self.map_configs(|config| config.with_env_callback(callback.clone()))
    }

    /// 追加环境创建回调 / Append environment creation callback
    pub fn add_env_callback(mut self, callback: GurobiEnvCallback) -> Self {
        let linear = self.linear.inner.solver_mut().config_mut();
        linear.env_callback = Some(match linear.env_callback.take() {
            Some(existing) => {
                let callback = callback.clone();
                Arc::new(move |env| {
                    existing(env)?;
                    callback(env)
                })
            }
            None => callback.clone(),
        });
        let quadratic = self.quadratic.solver_mut().config_mut();
        quadratic.env_callback = Some(match quadratic.env_callback.take() {
            Some(existing) => Arc::new(move |env| {
                existing(env)?;
                callback(env)
            }),
            None => callback,
        });
        self
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl LinearBendersDecompositionSolver for GurobiBendersDecompositionSolver {
    fn name(&self) -> &str {
        self.linear.name()
    }

    async fn solve_master(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        cuts: &[LinearCut],
    ) -> ospf_rust_core::error::Result<ospf_rust_core::solver::SolverOutput> {
        self.linear.solve_master(model, cuts).await
    }

    async fn solve_sub(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        master_solution: &[f64],
    ) -> ospf_rust_core::error::Result<LinearSubResult> {
        self.linear.solve_sub(model, master_solution).await
    }
}

#[cfg(not(feature = "async"))]
impl LinearBendersDecompositionSolver for GurobiBendersDecompositionSolver {
    fn name(&self) -> &str {
        self.linear.name()
    }

    fn solve_master(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        cuts: &[LinearCut],
    ) -> ospf_rust_core::error::Result<ospf_rust_core::solver::SolverOutput> {
        self.linear.solve_master(model, cuts)
    }

    fn solve_sub(
        &self,
        model: &ospf_rust_core::model::intermediate::LinearTriadModel,
        master_solution: &[f64],
    ) -> ospf_rust_core::error::Result<LinearSubResult> {
        self.linear.solve_sub(model, master_solution)
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl QuadraticBendersDecompositionSolver for GurobiBendersDecompositionSolver {
    async fn solve_master_quadratic(
        &self,
        model: &ospf_rust_core::model::intermediate::QuadraticTetradModel,
        linear_cuts: &[LinearCut],
        quadratic_cuts: &[QuadraticCut],
    ) -> ospf_rust_core::error::Result<ospf_rust_core::solver::SolverOutput> {
        self.quadratic
            .solve_master_quadratic(model, linear_cuts, quadratic_cuts)
            .await
    }

    async fn solve_sub_quadratic(
        &self,
        model: &ospf_rust_core::model::intermediate::QuadraticTetradModel,
        master_solution: &[f64],
    ) -> ospf_rust_core::error::Result<QuadraticSubResult> {
        self.quadratic
            .solve_sub_quadratic(model, master_solution)
            .await
    }
}

#[cfg(not(feature = "async"))]
impl QuadraticBendersDecompositionSolver for GurobiBendersDecompositionSolver {
    fn solve_master_quadratic(
        &self,
        model: &ospf_rust_core::model::intermediate::QuadraticTetradModel,
        linear_cuts: &[LinearCut],
        quadratic_cuts: &[QuadraticCut],
    ) -> ospf_rust_core::error::Result<ospf_rust_core::solver::SolverOutput> {
        self.quadratic
            .solve_master_quadratic(model, linear_cuts, quadratic_cuts)
    }

    fn solve_sub_quadratic(
        &self,
        model: &ospf_rust_core::model::intermediate::QuadraticTetradModel,
        master_solution: &[f64],
    ) -> ospf_rust_core::error::Result<QuadraticSubResult> {
        self.quadratic.solve_sub_quadratic(model, master_solution)
    }
}

#[cfg(all(
    test,
    any(feature = "gurobi10", feature = "gurobi11", feature = "gurobi12"),
    not(feature = "async")
))]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[test]
    fn gurobi_column_generation_solver_forwards_telemetry_config() {
        let callback: GurobiTelemetryCallback = Arc::new(|_| Ok(()));
        let native_observer: GurobiNativeObserver = Arc::new(|_| Ok(GurobiNativeControl::Continue));
        let diagnostics_callback: GurobiNumericDiagnosticsCallback = Arc::new(|_| Ok(()));
        let stage_callback: GurobiStageCallback = Arc::new(|_| Ok(()));
        let native_callback: GurobiNativeCallback = Arc::new(|_| Ok(()));
        let env_callback: GurobiEnvCallback = Arc::new(|_| Ok(()));
        let solver = GurobiColumnGenerationSolver::new()
            .with_telemetry_callback(Some(callback))
            .with_telemetry_min_interval(0.5)
            .with_numeric_diagnostics(true)
            .with_auto_apply_numeric_profile(true)
            .with_numeric_diagnostics_callback(Some(diagnostics_callback))
            .add_native_observer(native_observer)
            .with_numeric_focus(3)
            .with_scale_flag(2)
            .with_stage_callback(Some(stage_callback))
            .with_native_callback(Some(native_callback))
            .with_env_callback(Some(env_callback));

        assert!(solver.solver().config().telemetry_callback.is_some());
        assert_eq!(solver.solver().config().telemetry_min_interval, Some(0.5));
        assert!(solver.solver().config().enable_numeric_diagnostics);
        assert!(solver.solver().config().auto_apply_numeric_profile);
        assert!(
            solver
                .solver()
                .config()
                .numeric_diagnostics_callback
                .is_some()
        );
        assert_eq!(solver.solver().config().numeric_focus, Some(3));
        assert_eq!(solver.solver().config().scale_flag, Some(2));
        assert!(solver.solver().config().stage_callback.is_some());
        assert!(solver.solver().config().native_callback.is_some());
        assert_eq!(solver.solver().config().native_observers.len(), 1);
        assert!(solver.solver().config().env_callback.is_some());
    }

    #[test]
    fn gurobi_column_generation_solver_forwards_numeric_profiles() {
        let robust = GurobiColumnGenerationSolver::new().with_robust_defaults();
        assert_eq!(robust.solver().config().coefficient_zero_tolerance, 1e-10);
        assert_eq!(robust.solver().config().numeric_focus, Some(3));
        assert_eq!(robust.solver().config().scale_flag, Some(2));

        let performance = GurobiColumnGenerationSolver::new().with_performance_defaults();
        assert_eq!(
            performance.solver().config().coefficient_zero_tolerance,
            1e-13
        );
        assert_eq!(performance.solver().config().numeric_focus, Some(0));
        assert_eq!(performance.solver().config().scale_flag, Some(-1));

        let balanced = GurobiColumnGenerationSolver::new().with_balanced_defaults();
        assert_eq!(balanced.solver().config().coefficient_zero_tolerance, 1e-12);
        assert_eq!(balanced.solver().config().numeric_focus, Some(1));
        assert_eq!(balanced.solver().config().scale_flag, Some(1));
    }

    #[test]
    fn gurobi_column_generation_add_native_callback_overrides_previous_callback() {
        let first: GurobiNativeCallback = Arc::new(|_| Ok(()));
        let second: GurobiNativeCallback = Arc::new(|_| Ok(()));
        let solver = GurobiColumnGenerationSolver::new()
            .add_native_callback(first.clone())
            .add_native_callback(second.clone());
        let registered = solver
            .solver()
            .config()
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
    fn gurobi_linear_benders_solver_forwards_telemetry_config() {
        let callback: GurobiTelemetryCallback = Arc::new(|_| Ok(()));
        let native_observer: GurobiNativeObserver = Arc::new(|_| Ok(GurobiNativeControl::Continue));
        let diagnostics_callback: GurobiNumericDiagnosticsCallback = Arc::new(|_| Ok(()));
        let stage_callback: GurobiStageCallback = Arc::new(|_| Ok(()));
        let native_callback: GurobiNativeCallback = Arc::new(|_| Ok(()));
        let env_callback: GurobiEnvCallback = Arc::new(|_| Ok(()));
        let solver = GurobiLinearBendersDecompositionSolver::new()
            .with_telemetry_callback(Some(callback))
            .with_telemetry_min_interval(0.25)
            .with_numeric_diagnostics(true)
            .with_auto_apply_numeric_profile(true)
            .with_numeric_diagnostics_callback(Some(diagnostics_callback))
            .add_native_observer(native_observer)
            .with_numeric_focus(2)
            .with_scale_flag(1)
            .with_stage_callback(Some(stage_callback))
            .with_native_callback(Some(native_callback))
            .with_env_callback(Some(env_callback));

        assert!(solver.solver().config().telemetry_callback.is_some());
        assert_eq!(solver.solver().config().telemetry_min_interval, Some(0.25));
        assert!(solver.solver().config().enable_numeric_diagnostics);
        assert!(solver.solver().config().auto_apply_numeric_profile);
        assert!(
            solver
                .solver()
                .config()
                .numeric_diagnostics_callback
                .is_some()
        );
        assert_eq!(solver.solver().config().numeric_focus, Some(2));
        assert_eq!(solver.solver().config().scale_flag, Some(1));
        assert!(solver.solver().config().stage_callback.is_some());
        assert!(solver.solver().config().native_callback.is_some());
        assert_eq!(solver.solver().config().native_observers.len(), 1);
        assert!(solver.solver().config().env_callback.is_some());
    }

    #[test]
    fn gurobi_linear_benders_solver_forwards_numeric_profiles() {
        let robust = GurobiLinearBendersDecompositionSolver::new().with_robust_defaults();
        assert_eq!(robust.solver().config().coefficient_zero_tolerance, 1e-10);
        assert_eq!(robust.solver().config().numeric_focus, Some(3));
        assert_eq!(robust.solver().config().scale_flag, Some(2));

        let performance = GurobiLinearBendersDecompositionSolver::new().with_performance_defaults();
        assert_eq!(
            performance.solver().config().coefficient_zero_tolerance,
            1e-13
        );
        assert_eq!(performance.solver().config().numeric_focus, Some(0));
        assert_eq!(performance.solver().config().scale_flag, Some(-1));

        let balanced = GurobiLinearBendersDecompositionSolver::new().with_balanced_defaults();
        assert_eq!(balanced.solver().config().coefficient_zero_tolerance, 1e-12);
        assert_eq!(balanced.solver().config().numeric_focus, Some(1));
        assert_eq!(balanced.solver().config().scale_flag, Some(1));
    }

    #[test]
    fn gurobi_benders_solver_forwards_telemetry_config_to_both_paths() {
        let callback: GurobiTelemetryCallback = Arc::new(|_| Ok(()));
        let native_observer: GurobiNativeObserver = Arc::new(|_| Ok(GurobiNativeControl::Continue));
        let diagnostics_callback: GurobiNumericDiagnosticsCallback = Arc::new(|_| Ok(()));
        let stage_callback: GurobiStageCallback = Arc::new(|_| Ok(()));
        let native_callback: GurobiNativeCallback = Arc::new(|_| Ok(()));
        let env_callback: GurobiEnvCallback = Arc::new(|_| Ok(()));
        let solver = GurobiBendersDecompositionSolver::new()
            .with_telemetry_callback(Some(callback))
            .with_telemetry_min_interval(1.0)
            .with_numeric_diagnostics(true)
            .with_auto_apply_numeric_profile(true)
            .with_numeric_diagnostics_callback(Some(diagnostics_callback))
            .add_native_observer(native_observer)
            .with_numeric_focus(1)
            .with_scale_flag(3)
            .with_stage_callback(Some(stage_callback))
            .with_native_callback(Some(native_callback))
            .with_env_callback(Some(env_callback));

        assert!(
            solver
                .linear
                .inner
                .solver()
                .config()
                .telemetry_callback
                .is_some()
        );
        assert_eq!(
            solver.linear.inner.solver().config().telemetry_min_interval,
            Some(1.0)
        );
        assert_eq!(solver.linear.inner.solver().config().numeric_focus, Some(1));
        assert_eq!(solver.linear.inner.solver().config().scale_flag, Some(3));
        assert!(
            solver
                .linear
                .inner
                .solver()
                .config()
                .enable_numeric_diagnostics
        );
        assert!(
            solver
                .linear
                .inner
                .solver()
                .config()
                .auto_apply_numeric_profile
        );
        assert!(
            solver
                .linear
                .inner
                .solver()
                .config()
                .numeric_diagnostics_callback
                .is_some()
        );
        assert!(
            solver
                .linear
                .inner
                .solver()
                .config()
                .stage_callback
                .is_some()
        );
        assert!(
            solver
                .linear
                .inner
                .solver()
                .config()
                .native_callback
                .is_some()
        );
        assert_eq!(
            solver.linear.inner.solver().config().native_observers.len(),
            1
        );
        assert!(solver.linear.inner.solver().config().env_callback.is_some());
        assert!(
            solver
                .quadratic
                .solver()
                .config()
                .telemetry_callback
                .is_some()
        );
        assert_eq!(
            solver.quadratic.solver().config().telemetry_min_interval,
            Some(1.0)
        );
        assert_eq!(solver.quadratic.solver().config().numeric_focus, Some(1));
        assert_eq!(solver.quadratic.solver().config().scale_flag, Some(3));
        assert!(
            solver
                .quadratic
                .solver()
                .config()
                .enable_numeric_diagnostics
        );
        assert!(
            solver
                .quadratic
                .solver()
                .config()
                .auto_apply_numeric_profile
        );
        assert!(
            solver
                .quadratic
                .solver()
                .config()
                .numeric_diagnostics_callback
                .is_some()
        );
        assert!(solver.quadratic.solver().config().stage_callback.is_some());
        assert!(solver.quadratic.solver().config().native_callback.is_some());
        assert_eq!(solver.quadratic.solver().config().native_observers.len(), 1);
        assert!(solver.quadratic.solver().config().env_callback.is_some());
    }

    #[test]
    fn gurobi_benders_solver_forwards_numeric_profiles_to_both_paths() {
        let robust = GurobiBendersDecompositionSolver::new().with_robust_defaults();
        assert_eq!(
            robust
                .linear
                .inner
                .solver()
                .config()
                .coefficient_zero_tolerance,
            1e-10
        );
        assert_eq!(robust.linear.inner.solver().config().numeric_focus, Some(3));
        assert_eq!(robust.linear.inner.solver().config().scale_flag, Some(2));
        assert_eq!(
            robust
                .quadratic
                .solver()
                .config()
                .coefficient_zero_tolerance,
            1e-10
        );
        assert_eq!(robust.quadratic.solver().config().numeric_focus, Some(3));
        assert_eq!(robust.quadratic.solver().config().scale_flag, Some(2));

        let performance = GurobiBendersDecompositionSolver::new().with_performance_defaults();
        assert_eq!(
            performance
                .linear
                .inner
                .solver()
                .config()
                .coefficient_zero_tolerance,
            1e-13
        );
        assert_eq!(
            performance.linear.inner.solver().config().numeric_focus,
            Some(0)
        );
        assert_eq!(
            performance.linear.inner.solver().config().scale_flag,
            Some(-1)
        );
        assert_eq!(
            performance
                .quadratic
                .solver()
                .config()
                .coefficient_zero_tolerance,
            1e-13
        );
        assert_eq!(
            performance.quadratic.solver().config().numeric_focus,
            Some(0)
        );
        assert_eq!(performance.quadratic.solver().config().scale_flag, Some(-1));

        let balanced = GurobiBendersDecompositionSolver::new().with_balanced_defaults();
        assert_eq!(
            balanced
                .linear
                .inner
                .solver()
                .config()
                .coefficient_zero_tolerance,
            1e-12
        );
        assert_eq!(
            balanced.linear.inner.solver().config().numeric_focus,
            Some(1)
        );
        assert_eq!(balanced.linear.inner.solver().config().scale_flag, Some(1));
        assert_eq!(
            balanced
                .quadratic
                .solver()
                .config()
                .coefficient_zero_tolerance,
            1e-12
        );
        assert_eq!(balanced.quadratic.solver().config().numeric_focus, Some(1));
        assert_eq!(balanced.quadratic.solver().config().scale_flag, Some(1));
    }

    #[test]
    fn gurobi_column_generation_solver_add_stage_callback_aggregates_handlers() {
        let counter = Arc::new(AtomicUsize::new(0));
        let first_counter = counter.clone();
        let second_counter = counter.clone();
        let first: GurobiStageCallback = Arc::new(move |_| {
            first_counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        let second: GurobiStageCallback = Arc::new(move |_| {
            second_counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });

        let solver = GurobiColumnGenerationSolver::new()
            .add_stage_callback(first)
            .add_stage_callback(second);
        let callback = solver
            .solver()
            .config()
            .stage_callback
            .as_ref()
            .expect("stage callback should be registered");
        let status = ospf_rust_core::solver::solvers::gurobi::GurobiStageStatus {
            stage: ospf_rust_core::solver::solvers::gurobi::GurobiStage::AfterModeling,
            solver_status: None,
            solve_time: std::time::Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        };
        callback(&status).expect("stage callback chain should succeed");
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn gurobi_column_generation_solver_add_stage_callback_for_filters_stage() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_ref = counter.clone();
        let cb: GurobiStageCallback = Arc::new(move |_| {
            counter_ref.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });

        let solver = GurobiColumnGenerationSolver::new()
            .add_stage_callback_for(GurobiStage::AfterFailure, cb);
        let callback = solver
            .solver()
            .config()
            .stage_callback
            .as_ref()
            .expect("stage callback should be registered");

        let status_not_match = ospf_rust_core::solver::solvers::gurobi::GurobiStageStatus {
            stage: ospf_rust_core::solver::solvers::gurobi::GurobiStage::AfterModeling,
            solver_status: None,
            solve_time: std::time::Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        };
        callback(&status_not_match).expect("stage callback should succeed for non-match stage");
        assert_eq!(counter.load(Ordering::SeqCst), 0);

        let status_match = ospf_rust_core::solver::solvers::gurobi::GurobiStageStatus {
            stage: ospf_rust_core::solver::solvers::gurobi::GurobiStage::AfterFailure,
            solver_status: None,
            solve_time: std::time::Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        };
        callback(&status_match).expect("stage callback should succeed for match stage");
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn gurobi_benders_solver_add_stage_callback_for_filters_stage_on_both_paths() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_ref = counter.clone();
        let cb: GurobiStageCallback = Arc::new(move |_| {
            counter_ref.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });

        let solver = GurobiBendersDecompositionSolver::new()
            .add_stage_callback_for(GurobiStage::AfterFailure, cb);
        let linear_callback = solver
            .linear
            .inner
            .solver()
            .config()
            .stage_callback
            .as_ref()
            .expect("linear stage callback should be registered");
        let quadratic_callback = solver
            .quadratic
            .solver()
            .config()
            .stage_callback
            .as_ref()
            .expect("quadratic stage callback should be registered");

        let status_not_match = ospf_rust_core::solver::solvers::gurobi::GurobiStageStatus {
            stage: ospf_rust_core::solver::solvers::gurobi::GurobiStage::AfterModeling,
            solver_status: None,
            solve_time: std::time::Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        };
        linear_callback(&status_not_match).expect("linear callback should ignore non-match stage");
        quadratic_callback(&status_not_match)
            .expect("quadratic callback should ignore non-match stage");
        assert_eq!(counter.load(Ordering::SeqCst), 0);

        let status_match = ospf_rust_core::solver::solvers::gurobi::GurobiStageStatus {
            stage: ospf_rust_core::solver::solvers::gurobi::GurobiStage::AfterFailure,
            solver_status: None,
            solve_time: std::time::Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        };
        linear_callback(&status_match).expect("linear callback should accept match stage");
        quadratic_callback(&status_match).expect("quadratic callback should accept match stage");
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn gurobi_column_generation_solver_named_stage_helpers_can_chain() {
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

        let solver = GurobiColumnGenerationSolver::new()
            .add_after_modeling_callback(on_modeling)
            .add_configuration_callback(on_configuration)
            .add_analyzing_solution_callback(on_analyzing)
            .add_after_failure_callback(on_failure);
        let callback = solver
            .solver()
            .config()
            .stage_callback
            .as_ref()
            .expect("stage callback should be registered");

        let configuration_status = ospf_rust_core::solver::solvers::gurobi::GurobiStageStatus {
            stage: ospf_rust_core::solver::solvers::gurobi::GurobiStage::Configuration,
            solver_status: None,
            solve_time: std::time::Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        };
        callback(&configuration_status).expect("unrelated stage should be ignored");
        assert_eq!(modeling_counter.load(Ordering::SeqCst), 0);
        assert_eq!(configuration_counter.load(Ordering::SeqCst), 1);
        assert_eq!(analyzing_counter.load(Ordering::SeqCst), 0);
        assert_eq!(failure_counter.load(Ordering::SeqCst), 0);

        let modeling_status = ospf_rust_core::solver::solvers::gurobi::GurobiStageStatus {
            stage: ospf_rust_core::solver::solvers::gurobi::GurobiStage::AfterModeling,
            solver_status: None,
            solve_time: std::time::Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        };
        callback(&modeling_status).expect("after-modeling stage should run modeling callback");
        assert_eq!(modeling_counter.load(Ordering::SeqCst), 1);
        assert_eq!(configuration_counter.load(Ordering::SeqCst), 1);
        assert_eq!(analyzing_counter.load(Ordering::SeqCst), 0);
        assert_eq!(failure_counter.load(Ordering::SeqCst), 0);

        let analyzing_status = ospf_rust_core::solver::solvers::gurobi::GurobiStageStatus {
            stage: ospf_rust_core::solver::solvers::gurobi::GurobiStage::AnalyzingSolution,
            solver_status: None,
            solve_time: std::time::Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        };
        callback(&analyzing_status).expect("analyzing stage should run analyzing callback");
        assert_eq!(modeling_counter.load(Ordering::SeqCst), 1);
        assert_eq!(configuration_counter.load(Ordering::SeqCst), 1);
        assert_eq!(analyzing_counter.load(Ordering::SeqCst), 1);
        assert_eq!(failure_counter.load(Ordering::SeqCst), 0);

        let failure_status = ospf_rust_core::solver::solvers::gurobi::GurobiStageStatus {
            stage: ospf_rust_core::solver::solvers::gurobi::GurobiStage::AfterFailure,
            solver_status: None,
            solve_time: std::time::Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        };
        callback(&failure_status).expect("after-failure stage should run failure callback");
        assert_eq!(modeling_counter.load(Ordering::SeqCst), 1);
        assert_eq!(configuration_counter.load(Ordering::SeqCst), 1);
        assert_eq!(analyzing_counter.load(Ordering::SeqCst), 1);
        assert_eq!(failure_counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn gurobi_linear_benders_solver_named_stage_helpers_can_chain() {
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

        let solver = GurobiLinearBendersDecompositionSolver::new()
            .add_after_modeling_callback(on_modeling)
            .add_after_failure_callback(on_failure);
        let callback = solver
            .solver()
            .config()
            .stage_callback
            .as_ref()
            .expect("stage callback should be registered");

        let configuration_status = ospf_rust_core::solver::solvers::gurobi::GurobiStageStatus {
            stage: ospf_rust_core::solver::solvers::gurobi::GurobiStage::Configuration,
            solver_status: None,
            solve_time: std::time::Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        };
        callback(&configuration_status).expect("unrelated stage should be ignored");
        assert_eq!(modeling_counter.load(Ordering::SeqCst), 0);
        assert_eq!(failure_counter.load(Ordering::SeqCst), 0);

        let modeling_status = ospf_rust_core::solver::solvers::gurobi::GurobiStageStatus {
            stage: ospf_rust_core::solver::solvers::gurobi::GurobiStage::AfterModeling,
            solver_status: None,
            solve_time: std::time::Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        };
        callback(&modeling_status).expect("after-modeling stage should run modeling callback");
        assert_eq!(modeling_counter.load(Ordering::SeqCst), 1);
        assert_eq!(failure_counter.load(Ordering::SeqCst), 0);

        let failure_status = ospf_rust_core::solver::solvers::gurobi::GurobiStageStatus {
            stage: ospf_rust_core::solver::solvers::gurobi::GurobiStage::AfterFailure,
            solver_status: None,
            solve_time: std::time::Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        };
        callback(&failure_status).expect("after-failure stage should run failure callback");
        assert_eq!(modeling_counter.load(Ordering::SeqCst), 1);
        assert_eq!(failure_counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn gurobi_benders_solver_named_stage_helpers_filter_on_both_paths() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_ref = counter.clone();
        let callback: GurobiStageCallback = Arc::new(move |_| {
            counter_ref.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });

        let solver = GurobiBendersDecompositionSolver::new().add_after_failure_callback(callback);
        let linear_callback = solver
            .linear
            .inner
            .solver()
            .config()
            .stage_callback
            .as_ref()
            .expect("linear stage callback should be registered");
        let quadratic_callback = solver
            .quadratic
            .solver()
            .config()
            .stage_callback
            .as_ref()
            .expect("quadratic stage callback should be registered");

        let status_not_match = ospf_rust_core::solver::solvers::gurobi::GurobiStageStatus {
            stage: ospf_rust_core::solver::solvers::gurobi::GurobiStage::AfterModeling,
            solver_status: None,
            solve_time: std::time::Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        };
        linear_callback(&status_not_match).expect("linear callback should ignore non-match stage");
        quadratic_callback(&status_not_match)
            .expect("quadratic callback should ignore non-match stage");
        assert_eq!(counter.load(Ordering::SeqCst), 0);

        let status_match = ospf_rust_core::solver::solvers::gurobi::GurobiStageStatus {
            stage: ospf_rust_core::solver::solvers::gurobi::GurobiStage::AfterFailure,
            solver_status: None,
            solve_time: std::time::Duration::from_secs(0),
            objective_value: None,
            best_bound: None,
            mip_gap: None,
            iterations: None,
            node_count: None,
        };
        linear_callback(&status_match).expect("linear callback should accept match stage");
        quadratic_callback(&status_match).expect("quadratic callback should accept match stage");
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
}
