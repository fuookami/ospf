//! SCIP 扩展求解器接口实现
//! SCIP extension solver interfaces
//!
//! 提供 framework 层对 SCIP 的列生成与 Benders 分解适配器。
//! Provides framework-level column-generation and Benders adapters for SCIP.

use ospf_rust_core::model::mechanism::MechanismModel;
use ospf_rust_core::solver::solvers::SCIPSolver as CoreScipSolver;
use ospf_rust_core::solver::solvers::scip::{
    PresolvingMode, SCIPConfig, SCIPNativeCallback, SCIPNativeObserver, SCIPSnapshotObserver,
    SCIPStage, SCIPStageCallback, SCIPTelemetryCallback,
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

/// SCIP 列生成求解器 / SCIP column generation solver
#[derive(Debug)]
pub struct ScipColumnGenerationSolver {
    /// 内部列生成适配器 / Inner column-generation adapter wrapping the core SCIP solver
    inner: CoreColumnGenerationAdapter<CoreScipSolver>,
}

impl Default for ScipColumnGenerationSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ScipColumnGenerationSolver {
    fn map_config(mut self, mapper: impl FnOnce(SCIPConfig) -> SCIPConfig) -> Self {
        let current = self.inner.solver().config().clone();
        *self.inner.solver_mut().config_mut() = mapper(current);
        self
    }

    /// 创建默认求解器 / Create default solver
    pub fn new() -> Self {
        Self::with_solver(CoreScipSolver::new())
    }

    /// 使用配置创建求解器 / Create solver with configuration
    pub fn with_config(config: SCIPConfig) -> Self {
        Self::with_solver(CoreScipSolver::with_config(config))
    }

    /// 使用指定 core 求解器创建 / Create from explicit core solver
    pub fn with_solver(solver: CoreScipSolver) -> Self {
        Self {
            inner: CoreColumnGenerationAdapter::new("scip", solver),
        }
    }

    /// 获取底层 core 求解器 / Get underlying core solver
    pub fn solver(&self) -> &CoreScipSolver {
        self.inner.solver()
    }

    /// 应用推荐 LP/子问题配置 / Apply recommended LP/subproblem settings.
    pub fn with_lp_subproblem_defaults(self) -> Self {
        self.map_config(|config| config.with_lp_subproblem_defaults())
    }

    /// 设置求解 gap / Set solve gap
    pub fn with_gap(self, gap: f64) -> Self {
        self.map_config(|config| config.with_gap(gap))
    }

    /// 设置节点数限制 / Set node limit
    pub fn with_node_limit(self, limit: i64) -> Self {
        self.map_config(|config| config.with_node_limit(limit))
    }

    /// 设置内存限制（MB，旧接口）/ Set memory limit in MB (legacy alias)
    pub fn with_mem_limit(self, limit_mb: f64) -> Self {
        self.map_config(|config| config.with_mem_limit(limit_mb))
    }

    /// 设置内存限制（MB）/ Set memory limit (MB)
    pub fn with_memory_limit_mb(self, memory_limit_mb: f64) -> Self {
        self.map_config(|config| config.with_memory_limit_mb(memory_limit_mb))
    }

    /// 设置内存限制（GB）/ Set memory limit (GB)
    pub fn with_memory_limit_gb(self, memory_limit_gb: f64) -> Self {
        self.map_config(|config| config.with_memory_limit_gb(memory_limit_gb))
    }

    /// 设置控制台输出频率 / Set console display frequency
    pub fn with_display_freq(self, freq: i32) -> Self {
        self.map_config(|config| config.with_display_freq(freq))
    }

    /// 设置启发式优先级 / Set heuristics priority
    pub fn with_heuristics_priority(self, priority: i32) -> Self {
        self.map_config(|config| config.with_heuristics_priority(priority))
    }

    /// 设置无改进时间限制（秒）/ Set no-improvement time limit in seconds
    pub fn with_no_improvement_time_limit(self, seconds: f64) -> Self {
        self.map_config(|config| config.with_no_improvement_time_limit(seconds))
    }

    /// 设置改进容差 / Set improvement tolerance
    pub fn with_improvement_tolerance(self, tolerance: f64) -> Self {
        self.map_config(|config| config.with_improvement_tolerance(tolerance))
    }

    /// 设置改进判定阈值 / Set improvement threshold
    pub fn with_improve_threshold(self, threshold: f64) -> Self {
        self.map_config(|config| config.with_improve_threshold(threshold))
    }

    /// 设置遥测最小间隔（秒）/ Set telemetry minimum interval in seconds
    pub fn with_telemetry_min_interval(self, seconds: f64) -> Self {
        self.map_config(|config| config.with_telemetry_min_interval(seconds))
    }

    /// 设置阶段回调（替换已有） / Set stage callback (replaces existing)
    pub fn with_stage_callback(self, callback: Option<SCIPStageCallback>) -> Self {
        self.map_config(|config| config.with_stage_callback(callback))
    }

    /// 添加阶段回调 / Add stage callback
    pub fn add_stage_callback(self, callback: SCIPStageCallback) -> Self {
        self.map_config(|config| config.add_stage_callback(callback))
    }

    /// 为指定阶段添加回调 / Add callback for a specific stage
    pub fn add_stage_callback_for(self, stage: SCIPStage, callback: SCIPStageCallback) -> Self {
        self.map_config(|config| config.add_stage_callback_for(stage, callback))
    }

    /// 添加建模完成后回调 / Add callback invoked after modeling
    pub fn add_after_modeling_callback(self, callback: SCIPStageCallback) -> Self {
        self.map_config(|config| config.add_after_modeling_callback(callback))
    }

    /// 添加配置阶段回调 / Add callback invoked at configuration stage
    pub fn add_configuration_callback(self, callback: SCIPStageCallback) -> Self {
        self.map_config(|config| config.add_configuration_callback(callback))
    }

    /// 添加解分析回调 / Add callback invoked when analyzing solution
    pub fn add_analyzing_solution_callback(self, callback: SCIPStageCallback) -> Self {
        self.map_config(|config| config.add_analyzing_solution_callback(callback))
    }

    /// 添加求解失败后回调 / Add callback invoked after solve failure
    pub fn add_after_failure_callback(self, callback: SCIPStageCallback) -> Self {
        self.map_config(|config| config.add_after_failure_callback(callback))
    }

    /// 设置遥测回调（替换已有） / Set telemetry callback (replaces existing)
    pub fn with_telemetry_callback(self, callback: Option<SCIPTelemetryCallback>) -> Self {
        self.map_config(|config| config.with_telemetry_callback(callback))
    }

    /// 添加遥测回调 / Add telemetry callback
    pub fn add_telemetry_callback(self, callback: SCIPTelemetryCallback) -> Self {
        self.map_config(|config| config.add_telemetry_callback(callback))
    }

    /// 设置快照观察者列表（替换已有） / Set snapshot observers (replaces existing)
    pub fn with_snapshot_observers(self, observers: Vec<SCIPSnapshotObserver>) -> Self {
        self.map_config(|config| config.with_snapshot_observers(observers))
    }

    /// 添加快照观察者 / Add snapshot observer
    pub fn add_snapshot_observer(self, observer: SCIPSnapshotObserver) -> Self {
        self.map_config(|config| config.add_snapshot_observer(observer))
    }

    /// 设置原生回调（替换已有） / Set native callback (replaces existing)
    pub fn with_native_callback(self, callback: Option<SCIPNativeCallback>) -> Self {
        self.map_config(|config| config.with_native_callback(callback))
    }

    /// 添加原生回调（替换已有） / Add native callback (replaces previous)
    pub fn add_native_callback(mut self, callback: SCIPNativeCallback) -> Self {
        self.inner.solver_mut().config_mut().native_callback = Some(callback);
        self
    }

    /// 设置原生观察者列表（替换已有） / Set native observers (replaces existing)
    pub fn with_native_observers(self, observers: Vec<SCIPNativeObserver>) -> Self {
        self.map_config(|config| config.with_native_observers(observers))
    }

    /// 添加原生观察者 / Add native observer
    pub fn add_native_observer(mut self, observer: SCIPNativeObserver) -> Self {
        self.inner
            .solver_mut()
            .config_mut()
            .native_observers
            .push(observer);
        self
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl ColumnGenerationSolver for ScipColumnGenerationSolver {
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
impl ColumnGenerationSolver for ScipColumnGenerationSolver {
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

/// SCIP 线性 Benders 分解求解器 / SCIP linear Benders decomposition solver
#[derive(Debug)]
pub struct ScipLinearBendersDecompositionSolver {
    /// 内部线性 Benders 适配器 / Inner linear Benders adapter wrapping the core SCIP solver
    inner: CoreLinearBendersAdapter<CoreScipSolver>,
}

impl Default for ScipLinearBendersDecompositionSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ScipLinearBendersDecompositionSolver {
    fn map_config(mut self, mapper: impl FnOnce(SCIPConfig) -> SCIPConfig) -> Self {
        let current = self.inner.solver().config().clone();
        *self.inner.solver_mut().config_mut() = mapper(current);
        self
    }

    /// 创建默认求解器 / Create default solver
    pub fn new() -> Self {
        Self::with_solver(CoreScipSolver::new())
    }

    /// 使用配置创建求解器 / Create solver with configuration
    pub fn with_config(config: SCIPConfig) -> Self {
        Self::with_solver(CoreScipSolver::with_config(config))
    }

    /// 使用指定 core 求解器创建 / Create from explicit core solver
    pub fn with_solver(solver: CoreScipSolver) -> Self {
        Self {
            inner: CoreLinearBendersAdapter::new("scip", solver),
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
    pub fn solver(&self) -> &CoreScipSolver {
        self.inner.solver()
    }

    /// 应用推荐 LP/子问题配置 / Apply recommended LP/subproblem settings.
    pub fn with_lp_subproblem_defaults(self) -> Self {
        self.map_config(|config| config.with_lp_subproblem_defaults())
    }

    /// 设置求解 gap / Set solve gap
    pub fn with_gap(self, gap: f64) -> Self {
        self.map_config(|config| config.with_gap(gap))
    }

    /// 设置节点数限制 / Set node limit
    pub fn with_node_limit(self, limit: i64) -> Self {
        self.map_config(|config| config.with_node_limit(limit))
    }

    /// 设置内存限制（MB，旧接口）/ Set memory limit in MB (legacy alias)
    pub fn with_mem_limit(self, limit_mb: f64) -> Self {
        self.map_config(|config| config.with_mem_limit(limit_mb))
    }

    /// 设置内存限制（MB）/ Set memory limit (MB)
    pub fn with_memory_limit_mb(self, memory_limit_mb: f64) -> Self {
        self.map_config(|config| config.with_memory_limit_mb(memory_limit_mb))
    }

    /// 设置内存限制（GB）/ Set memory limit (GB)
    pub fn with_memory_limit_gb(self, memory_limit_gb: f64) -> Self {
        self.map_config(|config| config.with_memory_limit_gb(memory_limit_gb))
    }

    /// 设置控制台输出频率 / Set console display frequency
    pub fn with_display_freq(self, freq: i32) -> Self {
        self.map_config(|config| config.with_display_freq(freq))
    }

    /// 设置启发式优先级 / Set heuristics priority
    pub fn with_heuristics_priority(self, priority: i32) -> Self {
        self.map_config(|config| config.with_heuristics_priority(priority))
    }

    /// 设置无改进时间限制（秒）/ Set no-improvement time limit in seconds
    pub fn with_no_improvement_time_limit(self, seconds: f64) -> Self {
        self.map_config(|config| config.with_no_improvement_time_limit(seconds))
    }

    /// 设置改进容差 / Set improvement tolerance
    pub fn with_improvement_tolerance(self, tolerance: f64) -> Self {
        self.map_config(|config| config.with_improvement_tolerance(tolerance))
    }

    /// 设置改进判定阈值 / Set improvement threshold
    pub fn with_improve_threshold(self, threshold: f64) -> Self {
        self.map_config(|config| config.with_improve_threshold(threshold))
    }

    /// 设置遥测最小间隔（秒）/ Set telemetry minimum interval in seconds
    pub fn with_telemetry_min_interval(self, seconds: f64) -> Self {
        self.map_config(|config| config.with_telemetry_min_interval(seconds))
    }

    /// 设置阶段回调（替换已有） / Set stage callback (replaces existing)
    pub fn with_stage_callback(self, callback: Option<SCIPStageCallback>) -> Self {
        self.map_config(|config| config.with_stage_callback(callback))
    }

    /// 添加阶段回调 / Add stage callback
    pub fn add_stage_callback(self, callback: SCIPStageCallback) -> Self {
        self.map_config(|config| config.add_stage_callback(callback))
    }

    /// 为指定阶段添加回调 / Add callback for a specific stage
    pub fn add_stage_callback_for(self, stage: SCIPStage, callback: SCIPStageCallback) -> Self {
        self.map_config(|config| config.add_stage_callback_for(stage, callback))
    }

    /// 添加建模完成后回调 / Add callback invoked after modeling
    pub fn add_after_modeling_callback(self, callback: SCIPStageCallback) -> Self {
        self.map_config(|config| config.add_after_modeling_callback(callback))
    }

    /// 添加配置阶段回调 / Add callback invoked at configuration stage
    pub fn add_configuration_callback(self, callback: SCIPStageCallback) -> Self {
        self.map_config(|config| config.add_configuration_callback(callback))
    }

    /// 添加解分析回调 / Add callback invoked when analyzing solution
    pub fn add_analyzing_solution_callback(self, callback: SCIPStageCallback) -> Self {
        self.map_config(|config| config.add_analyzing_solution_callback(callback))
    }

    /// 添加求解失败后回调 / Add callback invoked after solve failure
    pub fn add_after_failure_callback(self, callback: SCIPStageCallback) -> Self {
        self.map_config(|config| config.add_after_failure_callback(callback))
    }

    /// 设置遥测回调（替换已有） / Set telemetry callback (replaces existing)
    pub fn with_telemetry_callback(self, callback: Option<SCIPTelemetryCallback>) -> Self {
        self.map_config(|config| config.with_telemetry_callback(callback))
    }

    /// 添加遥测回调 / Add telemetry callback
    pub fn add_telemetry_callback(self, callback: SCIPTelemetryCallback) -> Self {
        self.map_config(|config| config.add_telemetry_callback(callback))
    }

    /// 设置快照观察者列表（替换已有） / Set snapshot observers (replaces existing)
    pub fn with_snapshot_observers(self, observers: Vec<SCIPSnapshotObserver>) -> Self {
        self.map_config(|config| config.with_snapshot_observers(observers))
    }

    /// 添加快照观察者 / Add snapshot observer
    pub fn add_snapshot_observer(self, observer: SCIPSnapshotObserver) -> Self {
        self.map_config(|config| config.add_snapshot_observer(observer))
    }

    /// 设置原生回调（替换已有） / Set native callback (replaces existing)
    pub fn with_native_callback(self, callback: Option<SCIPNativeCallback>) -> Self {
        self.map_config(|config| config.with_native_callback(callback))
    }

    /// 添加原生回调（替换已有） / Add native callback (replaces previous)
    pub fn add_native_callback(mut self, callback: SCIPNativeCallback) -> Self {
        self.inner.solver_mut().config_mut().native_callback = Some(callback);
        self
    }

    /// 设置原生观察者列表（替换已有） / Set native observers (replaces existing)
    pub fn with_native_observers(self, observers: Vec<SCIPNativeObserver>) -> Self {
        self.map_config(|config| config.with_native_observers(observers))
    }

    /// 添加原生观察者 / Add native observer
    pub fn add_native_observer(mut self, observer: SCIPNativeObserver) -> Self {
        self.inner
            .solver_mut()
            .config_mut()
            .native_observers
            .push(observer);
        self
    }
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl LinearBendersDecompositionSolver for ScipLinearBendersDecompositionSolver {
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
impl LinearBendersDecompositionSolver for ScipLinearBendersDecompositionSolver {
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

/// SCIP Benders 分解求解器（线性 + 二次）/
/// SCIP Benders decomposition solver (linear + quadratic)
#[derive(Debug)]
pub struct ScipQuadraticBendersDecompositionSolver {
    /// 线性 Benders 求解器 / Linear Benders solver
    linear: ScipLinearBendersDecompositionSolver,
    /// 二次 Benders 适配器 / Quadratic Benders adapter
    quadratic: CoreQuadraticBendersAdapter<CoreScipSolver>,
}

impl Default for ScipQuadraticBendersDecompositionSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ScipQuadraticBendersDecompositionSolver {
    fn map_configs(mut self, mapper: impl Fn(SCIPConfig) -> SCIPConfig) -> Self {
        let linear_current = self.linear.inner.solver().config().clone();
        let quadratic_current = self.quadratic.solver().config().clone();
        *self.linear.inner.solver_mut().config_mut() = mapper(linear_current);
        *self.quadratic.solver_mut().config_mut() = mapper(quadratic_current);
        self
    }

    /// 创建默认求解器 / Create default solver
    pub fn new() -> Self {
        Self {
            linear: ScipLinearBendersDecompositionSolver::new(),
            quadratic: CoreQuadraticBendersAdapter::new("scip", CoreScipSolver::new()),
        }
    }

    /// 使用配置创建求解器 / Create solver with configuration
    pub fn with_config(config: SCIPConfig) -> Self {
        Self {
            linear: ScipLinearBendersDecompositionSolver::with_config(config.clone()),
            quadratic: CoreQuadraticBendersAdapter::new(
                "scip",
                CoreScipSolver::with_config(config),
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

    /// 设置节点数限制 / Set node limit
    pub fn with_node_limit(self, limit: i64) -> Self {
        self.map_configs(|config| config.with_node_limit(limit))
    }

    /// 应用推荐 LP/子问题配置 / Apply recommended LP/subproblem settings.
    pub fn with_lp_subproblem_defaults(self) -> Self {
        self.map_configs(|config| config.with_lp_subproblem_defaults())
    }

    /// 设置求解 gap / Set solve gap
    pub fn with_gap(self, gap: f64) -> Self {
        self.map_configs(|config| config.with_gap(gap))
    }

    /// 设置内存限制（MB，旧接口）/ Set memory limit in MB (legacy alias)
    pub fn with_mem_limit(self, limit_mb: f64) -> Self {
        self.map_configs(|config| config.with_mem_limit(limit_mb))
    }

    /// 设置内存限制（MB）/ Set memory limit (MB)
    pub fn with_memory_limit_mb(self, memory_limit_mb: f64) -> Self {
        self.map_configs(|config| config.with_memory_limit_mb(memory_limit_mb))
    }

    /// 设置内存限制（GB）/ Set memory limit (GB)
    pub fn with_memory_limit_gb(self, memory_limit_gb: f64) -> Self {
        self.map_configs(|config| config.with_memory_limit_gb(memory_limit_gb))
    }

    /// 设置控制台输出频率 / Set console display frequency
    pub fn with_display_freq(self, freq: i32) -> Self {
        self.map_configs(|config| config.with_display_freq(freq))
    }

    /// 设置启发式优先级 / Set heuristics priority
    pub fn with_heuristics_priority(self, priority: i32) -> Self {
        self.map_configs(|config| config.with_heuristics_priority(priority))
    }

    /// 设置无改进时间限制（秒）/ Set no-improvement time limit in seconds
    pub fn with_no_improvement_time_limit(self, seconds: f64) -> Self {
        self.map_configs(|config| config.with_no_improvement_time_limit(seconds))
    }

    /// 设置改进容差 / Set improvement tolerance
    pub fn with_improvement_tolerance(self, tolerance: f64) -> Self {
        self.map_configs(|config| config.with_improvement_tolerance(tolerance))
    }

    /// 设置改进判定阈值 / Set improvement threshold
    pub fn with_improve_threshold(self, threshold: f64) -> Self {
        self.map_configs(|config| config.with_improve_threshold(threshold))
    }

    /// 设置遥测最小间隔（秒）/ Set telemetry minimum interval in seconds
    pub fn with_telemetry_min_interval(self, seconds: f64) -> Self {
        self.map_configs(|config| config.with_telemetry_min_interval(seconds))
    }

    /// 设置阶段回调（替换已有） / Set stage callback (replaces existing)
    pub fn with_stage_callback(self, callback: Option<SCIPStageCallback>) -> Self {
        self.map_configs(|config| config.with_stage_callback(callback.clone()))
    }

    /// 添加阶段回调 / Add stage callback
    pub fn add_stage_callback(self, callback: SCIPStageCallback) -> Self {
        self.map_configs(|config| config.add_stage_callback(callback.clone()))
    }

    /// 为指定阶段添加回调 / Add callback for a specific stage
    pub fn add_stage_callback_for(self, stage: SCIPStage, callback: SCIPStageCallback) -> Self {
        self.map_configs(|config| config.add_stage_callback_for(stage, callback.clone()))
    }

    /// 添加建模完成后回调 / Add callback invoked after modeling
    pub fn add_after_modeling_callback(self, callback: SCIPStageCallback) -> Self {
        self.map_configs(|config| config.add_after_modeling_callback(callback.clone()))
    }

    /// 添加配置阶段回调 / Add callback invoked at configuration stage
    pub fn add_configuration_callback(self, callback: SCIPStageCallback) -> Self {
        self.map_configs(|config| config.add_configuration_callback(callback.clone()))
    }

    /// 添加解分析回调 / Add callback invoked when analyzing solution
    pub fn add_analyzing_solution_callback(self, callback: SCIPStageCallback) -> Self {
        self.map_configs(|config| config.add_analyzing_solution_callback(callback.clone()))
    }

    /// 添加求解失败后回调 / Add callback invoked after solve failure
    pub fn add_after_failure_callback(self, callback: SCIPStageCallback) -> Self {
        self.map_configs(|config| config.add_after_failure_callback(callback.clone()))
    }

    /// 设置遥测回调（替换已有） / Set telemetry callback (replaces existing)
    pub fn with_telemetry_callback(self, callback: Option<SCIPTelemetryCallback>) -> Self {
        self.map_configs(|config| config.with_telemetry_callback(callback.clone()))
    }

    /// 添加遥测回调 / Add telemetry callback
    pub fn add_telemetry_callback(self, callback: SCIPTelemetryCallback) -> Self {
        self.map_configs(|config| config.add_telemetry_callback(callback.clone()))
    }

    /// 设置快照观察者列表（替换已有） / Set snapshot observers (replaces existing)
    pub fn with_snapshot_observers(self, observers: Vec<SCIPSnapshotObserver>) -> Self {
        self.map_configs(|config| config.with_snapshot_observers(observers.clone()))
    }

    /// 添加快照观察者 / Add snapshot observer
    pub fn add_snapshot_observer(self, observer: SCIPSnapshotObserver) -> Self {
        self.map_configs(|config| config.add_snapshot_observer(observer.clone()))
    }

    /// 设置原生回调（替换已有） / Set native callback (replaces existing)
    pub fn with_native_callback(self, callback: Option<SCIPNativeCallback>) -> Self {
        self.map_configs(|config| config.with_native_callback(callback.clone()))
    }

    /// 添加原生回调（替换已有，同时应用于线性与二次路径） / Add native callback (replaces previous, applied to both linear and quadratic paths)
    pub fn add_native_callback(mut self, callback: SCIPNativeCallback) -> Self {
        self.linear.inner.solver_mut().config_mut().native_callback = Some(callback.clone());
        self.quadratic.solver_mut().config_mut().native_callback = Some(callback);
        self
    }

    /// 设置原生观察者列表（替换已有） / Set native observers (replaces existing)
    pub fn with_native_observers(self, observers: Vec<SCIPNativeObserver>) -> Self {
        self.map_configs(|config| config.with_native_observers(observers.clone()))
    }

    /// 添加原生观察者（同时应用于线性与二次路径） / Add native observer (applied to both linear and quadratic paths)
    pub fn add_native_observer(mut self, observer: SCIPNativeObserver) -> Self {
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
}

#[cfg(feature = "async")]
#[async_trait::async_trait]
impl LinearBendersDecompositionSolver for ScipQuadraticBendersDecompositionSolver {
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
impl LinearBendersDecompositionSolver for ScipQuadraticBendersDecompositionSolver {
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
impl QuadraticBendersDecompositionSolver for ScipQuadraticBendersDecompositionSolver {
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
impl QuadraticBendersDecompositionSolver for ScipQuadraticBendersDecompositionSolver {
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

#[cfg(all(test, feature = "scip", not(feature = "async")))]
mod tests {
    use super::*;
    use ospf_rust_core::model::intermediate::{
        BasicQuadraticTetradModel, QuadraticTetradModel, SparseMatrix, SparseVector,
    };
    use ospf_rust_core::model::{
        BasicMechanismModel, Linear, LinearConstraint, LinearInequality, LinearMonomial,
        MechanismModel, ObjectiveCategory,
    };
    use ospf_rust_core::solver::solvers::scip::{
        SCIPNativeControl, SCIPNativeSnapshot, SCIPSnapshotControl, SCIPStageStatus,
        SCIPTelemetryStatus,
    };
    use ospf_rust_core::token::Token;
    use ospf_rust_core::variable::ContinuousVariableItem;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn build_cut_context() -> (MechanismModel<f64>, Option<VariableId>, Vec<VariableId>) {
        let x = ContinuousVariableItem::auto("x");
        let y = ContinuousVariableItem::auto("y");
        let theta = ContinuousVariableItem::auto("theta");
        let x_id = x.id();
        let theta_id = theta.id();

        let mut basic = BasicMechanismModel::new("scip_quadratic_cut_context");
        basic.add_token(Token::from_generic(x, 0));
        basic.add_token(Token::from_generic(y, 1));
        basic.add_token(Token::from_generic(theta, 2));
        basic.add_constraint(LinearConstraint::new(
            LinearInequality::less_equal(
                Linear::new(
                    vec![LinearMonomial::new(1.0, 0), LinearMonomial::new(-1.0, 1)],
                    0.0,
                ),
                0.0,
            ),
            "link_xy",
        ));

        (
            MechanismModel::from_basic(basic),
            Some(theta_id),
            vec![x_id],
        )
    }

    fn build_true_quadratic_subproblem(
        rhs: f64,
        fixed_variable_id: VariableId,
    ) -> QuadraticTetradModel {
        let x = ContinuousVariableItem::create(fixed_variable_id, "sub_x");
        let mut basic = BasicQuadraticTetradModel::new("scip_quadratic_subproblem");
        basic.linear.add_variable(Token::from_generic(x, 0));

        let mut row = SparseVector::new();
        row.add(0, 1.0);
        basic.linear.add_constraint(row, rhs);

        let mut model = QuadraticTetradModel::from_basic(basic);
        let mut q = SparseMatrix::new();
        let mut q_row = SparseVector::new();
        q_row.add(0, 1.0);
        q.add_row(q_row);
        model.set_objective(vec![0.0], q, ObjectiveCategory::Minimum);
        model
    }

    #[test]
    fn scip_quadratic_benders_subproblem_feasible_uses_solver_dual_chain() {
        let (mechanism_model, objective_variable, fixed_variable_ids) = build_cut_context();
        let model = build_true_quadratic_subproblem(1.0, fixed_variable_ids[0]);
        let solver = ScipQuadraticBendersDecompositionSolver::with_config(
            SCIPConfig::new().with_output(false),
        )
        .with_cut_context(mechanism_model, objective_variable, fixed_variable_ids);

        let result = solver
            .solve_sub_quadratic(&model, &[0.0])
            .expect("SCIP quadratic feasible subproblem should succeed");
        match result {
            QuadraticSubResult::Feasible(feasible) => {
                assert_eq!(feasible.linear.dual_solution.constraints.len(), 1);
            }
            _ => panic!("expected feasible quadratic sub-result"),
        }
    }

    #[test]
    fn scip_quadratic_benders_subproblem_infeasible_uses_solver_farkas_chain() {
        let (mechanism_model, objective_variable, fixed_variable_ids) = build_cut_context();
        let model = build_true_quadratic_subproblem(-1.0, fixed_variable_ids[0]);
        let solver = ScipQuadraticBendersDecompositionSolver::with_config(
            SCIPConfig::new().with_output(false),
        )
        .with_cut_context(mechanism_model, objective_variable, fixed_variable_ids);

        let result = solver
            .solve_sub_quadratic(&model, &[0.0])
            .expect("SCIP quadratic infeasible subproblem should succeed");
        match result {
            QuadraticSubResult::Infeasible(infeasible) => {
                assert_eq!(infeasible.linear.farkas_dual_solution.constraints.len(), 1);
            }
            _ => panic!("expected infeasible quadratic sub-result"),
        }
    }

    #[test]
    fn scip_column_generation_solver_forwards_stage_and_telemetry_config() {
        let stage_counter = Arc::new(AtomicUsize::new(0));
        let telemetry_counter = Arc::new(AtomicUsize::new(0));
        let observer_counter = Arc::new(AtomicUsize::new(0));

        let stage_counter_ref = stage_counter.clone();
        let stage_callback: SCIPStageCallback = Arc::new(move |_status: &SCIPStageStatus| {
            stage_counter_ref.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        let telemetry_counter_ref = telemetry_counter.clone();
        let telemetry_callback: SCIPTelemetryCallback =
            Arc::new(move |_status: &SCIPTelemetryStatus| {
                telemetry_counter_ref.fetch_add(1, Ordering::SeqCst);
                Ok(())
            });
        let observer_counter_ref = observer_counter.clone();
        let observer: SCIPSnapshotObserver = Arc::new(move |_status: &SCIPTelemetryStatus| {
            observer_counter_ref.fetch_add(1, Ordering::SeqCst);
            Ok(SCIPSnapshotControl::Continue)
        });
        let native_observer: SCIPNativeObserver =
            Arc::new(move |_status: &SCIPNativeSnapshot| Ok(SCIPNativeControl::Continue));
        let native_callback: SCIPNativeCallback =
            Arc::new(move |_status: &SCIPNativeSnapshot| Ok(SCIPNativeControl::Continue));

        let solver = ScipColumnGenerationSolver::new()
            .with_node_limit(100)
            .with_mem_limit(512.0)
            .with_display_freq(5)
            .with_heuristics_priority(2)
            .with_no_improvement_time_limit(30.0)
            .with_improvement_tolerance(1e-6)
            .with_telemetry_min_interval(0.5)
            .add_after_modeling_callback(stage_callback)
            .with_telemetry_callback(Some(telemetry_callback))
            .add_snapshot_observer(observer)
            .add_native_observer(native_observer)
            .with_native_callback(Some(native_callback));

        let config = solver.solver().config();
        assert_eq!(config.node_limit, Some(100));
        assert_eq!(config.mem_limit, Some(512.0));
        assert_eq!(config.display_freq, Some(5));
        assert_eq!(config.heuristics_priority, Some(2));
        assert_eq!(config.no_improvement_time_limit, Some(30.0));
        assert_eq!(config.improvement_tolerance, Some(1e-6));
        assert_eq!(config.telemetry_min_interval, Some(0.5));
        assert!(config.stage_callback.is_some());
        assert!(config.telemetry_callback.is_some());
        assert_eq!(config.snapshot_observers.len(), 1);
        assert!(config.native_callback.is_some());
        assert_eq!(config.native_observers.len(), 1);
    }

    #[test]
    fn scip_column_generation_add_native_callback_overrides_previous_callback() {
        let first: SCIPNativeCallback = Arc::new(|_| Ok(SCIPNativeControl::Continue));
        let second: SCIPNativeCallback = Arc::new(|_| Ok(SCIPNativeControl::Continue));
        let solver = ScipColumnGenerationSolver::new()
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
            "latest native callback should override previous callback"
        );
    }

    #[test]
    fn scip_column_generation_add_native_observer_appends_observers() {
        let first: SCIPNativeObserver = Arc::new(|_| Ok(SCIPNativeControl::Continue));
        let second: SCIPNativeObserver = Arc::new(|_| Ok(SCIPNativeControl::Continue));
        let solver = ScipColumnGenerationSolver::new()
            .add_native_observer(first.clone())
            .add_native_observer(second.clone());
        let observers = &solver.solver().config().native_observers;
        assert_eq!(observers.len(), 2);
        assert!(Arc::ptr_eq(&observers[0], &first));
        assert!(Arc::ptr_eq(&observers[1], &second));
    }

    #[test]
    fn scip_lp_subproblem_defaults_are_available_on_framework_wrappers() {
        let column_generation = ScipColumnGenerationSolver::new().with_lp_subproblem_defaults();
        let column_config = column_generation.solver().config();
        assert_eq!(column_config.threads, Some(1));
        assert_eq!(column_config.presolving, Some(PresolvingMode::Off));
        assert_eq!(column_config.heuristics_priority, Some(0));

        let linear_benders =
            ScipLinearBendersDecompositionSolver::new().with_lp_subproblem_defaults();
        let linear_config = linear_benders.solver().config();
        assert_eq!(linear_config.threads, Some(1));
        assert_eq!(linear_config.presolving, Some(PresolvingMode::Off));
        assert_eq!(linear_config.heuristics_priority, Some(0));

        let quadratic_benders =
            ScipQuadraticBendersDecompositionSolver::new().with_lp_subproblem_defaults();
        let linear_path_config = quadratic_benders.linear.solver().config();
        let quadratic_path_config = quadratic_benders.quadratic.solver().config();
        assert_eq!(linear_path_config.threads, Some(1));
        assert_eq!(quadratic_path_config.threads, Some(1));
        assert_eq!(linear_path_config.presolving, Some(PresolvingMode::Off));
        assert_eq!(quadratic_path_config.presolving, Some(PresolvingMode::Off));
        assert_eq!(linear_path_config.heuristics_priority, Some(0));
        assert_eq!(quadratic_path_config.heuristics_priority, Some(0));
    }

    #[test]
    fn scip_ergonomic_config_aliases_are_available_on_framework_wrappers() {
        let column_generation = ScipColumnGenerationSolver::new()
            .with_gap(0.02)
            .with_memory_limit_gb(1.5)
            .with_improve_threshold(1e-5);
        let column_config = column_generation.solver().config();
        assert_eq!(column_config.mip_gap, Some(0.02));
        assert_eq!(column_config.mem_limit, Some(1536.0));
        assert_eq!(column_config.improvement_tolerance, Some(1e-5));

        let linear_benders = ScipLinearBendersDecompositionSolver::new()
            .with_gap(0.03)
            .with_memory_limit_mb(640.0)
            .with_improve_threshold(1e-6);
        let linear_config = linear_benders.solver().config();
        assert_eq!(linear_config.mip_gap, Some(0.03));
        assert_eq!(linear_config.mem_limit, Some(640.0));
        assert_eq!(linear_config.improvement_tolerance, Some(1e-6));

        let quadratic_benders = ScipQuadraticBendersDecompositionSolver::new()
            .with_gap(0.04)
            .with_memory_limit_gb(2.0)
            .with_improve_threshold(1e-7);
        let linear_path_config = quadratic_benders.linear.solver().config();
        let quadratic_path_config = quadratic_benders.quadratic.solver().config();
        assert_eq!(linear_path_config.mip_gap, Some(0.04));
        assert_eq!(quadratic_path_config.mip_gap, Some(0.04));
        assert_eq!(linear_path_config.mem_limit, Some(2048.0));
        assert_eq!(quadratic_path_config.mem_limit, Some(2048.0));
        assert_eq!(linear_path_config.improvement_tolerance, Some(1e-7));
        assert_eq!(quadratic_path_config.improvement_tolerance, Some(1e-7));
    }

    #[test]
    fn scip_linear_benders_solver_forwards_stage_and_telemetry_config() {
        let stage_callback: SCIPStageCallback = Arc::new(|_status: &SCIPStageStatus| Ok(()));
        let telemetry_callback: SCIPTelemetryCallback =
            Arc::new(|_status: &SCIPTelemetryStatus| Ok(()));
        let observer: SCIPSnapshotObserver =
            Arc::new(|_status: &SCIPTelemetryStatus| Ok(SCIPSnapshotControl::Continue));
        let native_observer: SCIPNativeObserver =
            Arc::new(|_status: &SCIPNativeSnapshot| Ok(SCIPNativeControl::Continue));
        let native_callback: SCIPNativeCallback =
            Arc::new(|_status: &SCIPNativeSnapshot| Ok(SCIPNativeControl::Continue));

        let solver = ScipLinearBendersDecompositionSolver::new()
            .with_node_limit(66)
            .with_mem_limit(640.0)
            .with_display_freq(6)
            .with_heuristics_priority(3)
            .with_no_improvement_time_limit(22.0)
            .with_improvement_tolerance(1e-7)
            .with_telemetry_min_interval(0.3)
            .add_configuration_callback(stage_callback)
            .with_telemetry_callback(Some(telemetry_callback))
            .add_snapshot_observer(observer)
            .add_native_observer(native_observer)
            .with_native_callback(Some(native_callback));

        let config = solver.solver().config();
        assert_eq!(config.node_limit, Some(66));
        assert_eq!(config.mem_limit, Some(640.0));
        assert_eq!(config.display_freq, Some(6));
        assert_eq!(config.heuristics_priority, Some(3));
        assert_eq!(config.no_improvement_time_limit, Some(22.0));
        assert_eq!(config.improvement_tolerance, Some(1e-7));
        assert_eq!(config.telemetry_min_interval, Some(0.3));
        assert!(config.stage_callback.is_some());
        assert!(config.telemetry_callback.is_some());
        assert_eq!(config.snapshot_observers.len(), 1);
        assert!(config.native_callback.is_some());
        assert_eq!(config.native_observers.len(), 1);
    }

    #[test]
    fn scip_quadratic_benders_solver_forwards_stage_and_telemetry_to_both_paths() {
        let stage_callback: SCIPStageCallback = Arc::new(|_status: &SCIPStageStatus| Ok(()));
        let telemetry_callback: SCIPTelemetryCallback =
            Arc::new(|_status: &SCIPTelemetryStatus| Ok(()));
        let observer: SCIPSnapshotObserver =
            Arc::new(|_status: &SCIPTelemetryStatus| Ok(SCIPSnapshotControl::Continue));
        let native_observer: SCIPNativeObserver =
            Arc::new(|_status: &SCIPNativeSnapshot| Ok(SCIPNativeControl::Continue));
        let native_callback: SCIPNativeCallback =
            Arc::new(|_status: &SCIPNativeSnapshot| Ok(SCIPNativeControl::Continue));

        let solver = ScipQuadraticBendersDecompositionSolver::new()
            .with_node_limit(77)
            .with_mem_limit(256.0)
            .with_display_freq(3)
            .with_heuristics_priority(1)
            .with_no_improvement_time_limit(12.0)
            .with_improvement_tolerance(1e-5)
            .with_telemetry_min_interval(1.0)
            .add_after_failure_callback(stage_callback)
            .with_telemetry_callback(Some(telemetry_callback))
            .add_snapshot_observer(observer)
            .add_native_observer(native_observer)
            .with_native_callback(Some(native_callback));

        let linear_config = solver.linear.solver().config();
        let quadratic_config = solver.quadratic.solver().config();

        assert_eq!(linear_config.node_limit, Some(77));
        assert_eq!(quadratic_config.node_limit, Some(77));
        assert_eq!(linear_config.mem_limit, Some(256.0));
        assert_eq!(quadratic_config.mem_limit, Some(256.0));
        assert_eq!(linear_config.display_freq, Some(3));
        assert_eq!(quadratic_config.display_freq, Some(3));
        assert_eq!(linear_config.heuristics_priority, Some(1));
        assert_eq!(quadratic_config.heuristics_priority, Some(1));
        assert_eq!(linear_config.no_improvement_time_limit, Some(12.0));
        assert_eq!(quadratic_config.no_improvement_time_limit, Some(12.0));
        assert_eq!(linear_config.improvement_tolerance, Some(1e-5));
        assert_eq!(quadratic_config.improvement_tolerance, Some(1e-5));
        assert_eq!(linear_config.telemetry_min_interval, Some(1.0));
        assert_eq!(quadratic_config.telemetry_min_interval, Some(1.0));
        assert!(linear_config.stage_callback.is_some());
        assert!(quadratic_config.stage_callback.is_some());
        assert!(linear_config.telemetry_callback.is_some());
        assert!(quadratic_config.telemetry_callback.is_some());
        assert_eq!(linear_config.snapshot_observers.len(), 1);
        assert_eq!(quadratic_config.snapshot_observers.len(), 1);
        assert!(linear_config.native_callback.is_some());
        assert!(quadratic_config.native_callback.is_some());
        assert_eq!(linear_config.native_observers.len(), 1);
        assert_eq!(quadratic_config.native_observers.len(), 1);
    }
}

#[cfg(all(test, feature = "scip", feature = "async"))]
mod async_tests {
    use super::*;
    use ospf_rust_core::model::intermediate::{
        BasicQuadraticTetradModel, QuadraticTetradModel, SparseMatrix, SparseVector,
    };
    use ospf_rust_core::model::{
        BasicMechanismModel, Linear, LinearConstraint, LinearInequality, LinearMonomial,
        MechanismModel, ObjectiveCategory,
    };
    use ospf_rust_core::token::Token;
    use ospf_rust_core::variable::ContinuousVariableItem;

    fn build_cut_context() -> (MechanismModel<f64>, Option<VariableId>, Vec<VariableId>) {
        let x = ContinuousVariableItem::auto("x");
        let y = ContinuousVariableItem::auto("y");
        let theta = ContinuousVariableItem::auto("theta");
        let x_id = x.id();
        let theta_id = theta.id();

        let mut basic = BasicMechanismModel::new("scip_quadratic_cut_context_async");
        basic.add_token(Token::from_generic(x, 0));
        basic.add_token(Token::from_generic(y, 1));
        basic.add_token(Token::from_generic(theta, 2));
        basic.add_constraint(LinearConstraint::new(
            LinearInequality::less_equal(
                Linear::new(
                    vec![LinearMonomial::new(1.0, 0), LinearMonomial::new(-1.0, 1)],
                    0.0,
                ),
                0.0,
            ),
            "link_xy",
        ));

        (
            MechanismModel::from_basic(basic),
            Some(theta_id),
            vec![x_id],
        )
    }

    fn build_true_quadratic_subproblem(
        rhs: f64,
        fixed_variable_id: VariableId,
    ) -> QuadraticTetradModel {
        let x = ContinuousVariableItem::create(fixed_variable_id, "sub_x");
        let mut basic = BasicQuadraticTetradModel::new("scip_quadratic_subproblem_async");
        basic.linear.add_variable(Token::from_generic(x, 0));

        let mut row = SparseVector::new();
        row.add(0, 1.0);
        basic.linear.add_constraint(row, rhs);

        let mut model = QuadraticTetradModel::from_basic(basic);
        let mut q = SparseMatrix::new();
        let mut q_row = SparseVector::new();
        q_row.add(0, 1.0);
        q.add_row(q_row);
        model.set_objective(vec![0.0], q, ObjectiveCategory::Minimum);
        model
    }

    #[tokio::test]
    async fn scip_quadratic_benders_subproblem_feasible_uses_solver_dual_chain_async() {
        let (mechanism_model, objective_variable, fixed_variable_ids) = build_cut_context();
        let model = build_true_quadratic_subproblem(1.0, fixed_variable_ids[0]);
        let solver = ScipQuadraticBendersDecompositionSolver::with_config(
            SCIPConfig::new().with_output(false),
        )
        .with_cut_context(mechanism_model, objective_variable, fixed_variable_ids);

        let result = solver
            .solve_sub_quadratic(&model, &[0.0])
            .await
            .expect("SCIP quadratic feasible subproblem should succeed in async mode");
        match result {
            QuadraticSubResult::Feasible(feasible) => {
                assert_eq!(feasible.linear.dual_solution.constraints.len(), 1);
            }
            _ => panic!("expected feasible quadratic sub-result"),
        }
    }

    #[tokio::test]
    async fn scip_quadratic_benders_subproblem_infeasible_uses_solver_farkas_chain_async() {
        let (mechanism_model, objective_variable, fixed_variable_ids) = build_cut_context();
        let model = build_true_quadratic_subproblem(-1.0, fixed_variable_ids[0]);
        let solver = ScipQuadraticBendersDecompositionSolver::with_config(
            SCIPConfig::new().with_output(false),
        )
        .with_cut_context(mechanism_model, objective_variable, fixed_variable_ids);

        let result = solver
            .solve_sub_quadratic(&model, &[0.0])
            .await
            .expect("SCIP quadratic infeasible subproblem should succeed in async mode");
        match result {
            QuadraticSubResult::Infeasible(infeasible) => {
                assert_eq!(infeasible.linear.farkas_dual_solution.constraints.len(), 1);
            }
            _ => panic!("expected infeasible quadratic sub-result"),
        }
    }
}
