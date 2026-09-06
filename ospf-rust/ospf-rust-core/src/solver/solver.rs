//! 求解器 Trait 定义
//! Solver Trait Definitions

use super::{
    ProgressValue, SolveHandle, SolveProgressReporter, SolveProgressSnapshot, SolveReport,
    SolveStage, SolveWarning, SolverOutput, SolverProvenance, SolvingStatus,
    attach_linear_model_mapping, attach_quadratic_model_mapping, cancelled_solve_report,
    configuration_fingerprint, linear_model_fingerprint, quadratic_model_fingerprint,
    solver_output_to_report_with_cancellation, solver_output_to_report_with_provenance,
    solver_provenance_fingerprint,
};
use crate::error::{CoreError, Result, SolverError};
use crate::model::intermediate::{LinearTriadModel, QuadraticTetradModel};

pub(crate) fn emit_progress_start(
    reporter: Option<&SolveProgressReporter>,
    solver_name: &str,
) -> Result<()> {
    let Some(reporter) = reporter else {
        return Ok(());
    };
    let snapshot = SolveProgressSnapshot::new(
        solver_name,
        SolveStage::Solving,
        vec![
            "solver".to_owned(),
            SolveStage::Solving.stable_name().to_owned(),
        ],
        ProgressValue::known(0.0)?,
        ProgressValue::known(0.0)?,
        std::time::Duration::ZERO,
        None,
        None,
        None,
        false,
    )?;
    reporter(&snapshot)
}

pub(crate) fn validate_per_solve_limits(options: &super::SolveOptions<'_>) -> Result<()> {
    if options.time_limit.is_some_and(|limit| limit.is_zero()) {
        return Err(CoreError::Solver(SolverError::InvalidInput(
            "per-solve time limit must be positive".to_owned(),
        )));
    }
    if options.node_limit.is_some_and(|limit| limit == 0)
        || options.solution_limit.is_some_and(|limit| limit == 0)
    {
        return Err(CoreError::Solver(SolverError::InvalidInput(
            "per-solve node and solution limits must be positive".to_owned(),
        )));
    }
    Ok(())
}

pub(crate) fn emit_progress_terminal(
    reporter: Option<&SolveProgressReporter>,
    solver_name: &str,
    report: &SolveReport<f64>,
) -> Result<()> {
    let Some(reporter) = reporter else {
        return Ok(());
    };
    let objective = report
        .solution
        .as_ref()
        .and_then(|solution| solution.objective_value.or(solution.objective));
    let snapshot = SolveProgressSnapshot::new(
        solver_name,
        SolveStage::Completed,
        vec![
            "solver".to_owned(),
            SolveStage::Completed.stable_name().to_owned(),
        ],
        ProgressValue::known(100.0)?,
        ProgressValue::known(100.0)?,
        report.statistics.solve_time,
        objective,
        report.statistics.best_bound_value,
        report.statistics.relative_gap,
        true,
    )?;
    reporter(&snapshot)
}

fn compatibility_linear_report(
    solver_name: &str,
    model: &LinearTriadModel,
    output: SolverOutput,
    cancellation_handle: Option<&SolveHandle>,
) -> Result<SolveReport<f64>> {
    let legacy_status = format!("{:?}", output.status);
    let provenance = SolverProvenance {
        solver_id: solver_name.to_owned(),
        backend_name: solver_name.to_owned(),
        ..SolverProvenance::default()
    };
    if let Some(handle) = cancellation_handle {
        handle.mark_completed();
    }
    let mut report = match cancellation_handle {
        Some(handle) => {
            solver_output_to_report_with_cancellation(output, provenance.clone(), handle)?
        }
        None => solver_output_to_report_with_provenance(output, provenance.clone())?,
    };
    report.warnings.push(SolveWarning::new(
        "LegacyStatusMapping",
        "report was projected from the compatibility SolverOutput facade",
    ));
    report
        .diagnostics
        .extensions
        .insert("legacy.solverStatus".to_owned(), legacy_status);
    report.fingerprints.model = Some(linear_model_fingerprint(model)?);
    report.fingerprints.configuration = Some(configuration_fingerprint(
        &provenance.effective_configuration,
    ));
    report.fingerprints.solver = Some(solver_provenance_fingerprint(&provenance));
    attach_linear_model_mapping(report, model)
}

fn compatibility_quadratic_report(
    solver_name: &str,
    model: &QuadraticTetradModel,
    output: SolverOutput,
    cancellation_handle: Option<&SolveHandle>,
) -> Result<SolveReport<f64>> {
    let legacy_status = format!("{:?}", output.status);
    let provenance = SolverProvenance {
        solver_id: solver_name.to_owned(),
        backend_name: solver_name.to_owned(),
        ..SolverProvenance::default()
    };
    if let Some(handle) = cancellation_handle {
        handle.mark_completed();
    }
    let mut report = match cancellation_handle {
        Some(handle) => {
            solver_output_to_report_with_cancellation(output, provenance.clone(), handle)?
        }
        None => solver_output_to_report_with_provenance(output, provenance.clone())?,
    };
    report.warnings.push(SolveWarning::new(
        "LegacyStatusMapping",
        "report was projected from the compatibility SolverOutput facade",
    ));
    report
        .diagnostics
        .extensions
        .insert("legacy.solverStatus".to_owned(), legacy_status);
    report.fingerprints.model = Some(quadratic_model_fingerprint(model)?);
    report.fingerprints.configuration = Some(configuration_fingerprint(
        &provenance.effective_configuration,
    ));
    report.fingerprints.solver = Some(solver_provenance_fingerprint(&provenance));
    attach_quadratic_model_mapping(report, model)
}

/// 运行时能力级别 / Runtime capability level.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilitySupport {
    /// 已由当前运行时确认支持 / Confirmed supported by the current runtime.
    Supported,
    /// 依赖许可证、插件或外部环境 / Depends on a license, plugin, or external environment.
    Conditional,
    /// 当前运行时明确不支持 / Explicitly unsupported by the current runtime.
    Unsupported,
}

/// 求解器能力集合 / Solver capability set.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SolverCapabilities {
    /// 按稳定能力名保存的能力级别 / Capability levels keyed by stable names.
    pub levels: std::collections::BTreeMap<String, CapabilitySupport>,
}

impl SolverCapabilities {
    /// 从旧能力枚举创建运行时能力集合 / Create runtime capabilities from legacy capability enums.
    pub fn from_legacy(capabilities: &[SolverCapability]) -> Self {
        let mut levels = std::collections::BTreeMap::new();
        for capability in capabilities {
            levels.insert(
                capability.stable_name().to_owned(),
                CapabilitySupport::Conditional,
            );
        }
        Self { levels }
    }

    /// 设置一个能力级别 / Set one capability level.
    pub fn with(mut self, capability: impl Into<String>, support: CapabilitySupport) -> Self {
        self.levels.insert(capability.into(), support);
        self
    }

    /// 查询能力级别 / Get one capability level.
    pub fn support(&self, capability: &str) -> CapabilitySupport {
        self.levels
            .get(capability)
            .copied()
            .unwrap_or(CapabilitySupport::Unsupported)
    }
}

/// 求解器运行时描述 / Runtime solver descriptor.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolverDescriptor {
    /// 稳定求解器 ID / Stable solver ID.
    pub solver_id: String,
    /// 展示名称 / Display name.
    pub display_name: String,
    /// backend 名称 / Backend name.
    pub backend_name: String,
    /// backend 版本 / Backend version.
    pub backend_version: Option<String>,
    /// 当前运行时是否可用 / Whether the current runtime is available.
    pub runtime_available: Option<bool>,
    /// 能力集合 / Capability set.
    pub capabilities: SolverCapabilities,
    /// 不能重放或需外部条件的原因 / Reasons for non-replayability or external prerequisites.
    pub warnings: Vec<String>,
}

impl SolverDescriptor {
    /// 创建未知运行时描述 / Create a descriptor with unknown runtime availability.
    pub fn unknown(name: &str, capabilities: &[SolverCapability]) -> Self {
        Self {
            solver_id: name.to_owned(),
            display_name: name.to_owned(),
            backend_name: name.to_owned(),
            backend_version: None,
            runtime_available: None,
            capabilities: SolverCapabilities::from_legacy(capabilities),
            warnings: vec!["runtime availability requires a backend probe".to_owned()],
        }
    }
}

/// 求解器能力 / Solver Capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverCapability {
    /// 线性规划 / Linear Programming
    Linear,
    /// 混合整数线性规划 / Mixed Integer Linear Programming
    Mip,
    /// 二次规划 / Quadratic Programming
    Quadratic,
    /// 混合整数二次规划 / Mixed Integer Quadratic Programming
    MIQP,
    /// 二阶锥规划 / Second Order Cone Programming
    SOCP,
    /// 原生 indicator 约束支持 / Native indicator-constraint support
    NativeIndicator,
    /// 原生 SOS1 约束支持 / Native SOS1-constraint support
    NativeSOS1,
    /// 约束规划模型 / Constraint-programming model
    ConstraintProgramming,
}

impl SolverCapability {
    fn stable_name(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::Mip => "mip",
            Self::Quadratic => "quadratic",
            Self::MIQP => "miqp",
            Self::SOCP => "socp",
            Self::NativeIndicator => "native_indicator",
            Self::NativeSOS1 => "native_sos1",
            Self::ConstraintProgramming => "constraint_programming",
        }
    }
}

/// 求解器通用信息 Trait / Solver common info trait
pub trait SolverInfo: Send + Sync {
    /// 获取求解器名称 / Get solver name
    fn name(&self) -> &str;

    /// 获取求解器能力 / Get solver capabilities
    fn capabilities(&self) -> Vec<SolverCapability>;

    /// 获取运行时描述 / Get the runtime descriptor.
    fn descriptor(&self) -> SolverDescriptor {
        SolverDescriptor::unknown(self.name(), &self.capabilities())
    }

    /// 检查是否支持某种能力 / Check if capability is supported
    fn supports(&self, capability: SolverCapability) -> bool {
        self.capabilities().contains(&capability)
    }
}

/// 线性模型求解 trait / Linear-model solver trait
pub trait LinearSolver: SolverInfo {
    /// 求解线性模型 / Solve linear model
    fn solve_linear(&self, model: &LinearTriadModel) -> Result<SolverOutput>;

    /// 以统一报告求解线性模型 / Solve a linear model as a unified report.
    ///
    /// 默认实现通过兼容 `SolverOutput` 投影，并保留迁移 warning / The default implementation projects the compatibility `SolverOutput` and records a migration warning.
    fn solve_linear_report(&self, model: &LinearTriadModel) -> Result<SolveReport<f64>> {
        compatibility_linear_report(self.name(), model, self.solve_linear(model)?, None)
    }

    /// 求解线性模型并尝试返回解池（可选）/ Solve linear model and optionally return solution pool
    ///
    /// 默认实现返回 `None`，表示求解器未提供原生多解支持 / Default returns `None`, meaning no native multi-solution support.
    fn solve_linear_with_solution_pool(
        &self,
        _model: &LinearTriadModel,
        _solution_amount: usize,
    ) -> Result<Option<(SolverOutput, Vec<Vec<f64>>)>> {
        Ok(None)
    }

    /// 使用统一参数求解线性模型并尝试返回解池（可选） / Solve a linear model with unified options and optionally return a solution pool.
    ///
    /// 默认实现委托旧的解池入口，保留既有 backend 的兼容性；需要原生取消或其它
    /// 求解参数的 backend 应覆盖此方法。
    /// The default delegates to the legacy pool entry for backend compatibility; backends that support native cancellation or other solve options should override this method.
    fn solve_linear_with_solution_pool_with_options(
        &self,
        model: &LinearTriadModel,
        solution_amount: usize,
        options: &super::SolveOptions<'_>,
    ) -> Result<Option<(SolverOutput, Vec<Vec<f64>>)>> {
        validate_per_solve_limits(options)?;
        self.solve_linear_with_solution_pool(model, solution_amount)
    }

    /// 求解线性模型（参数对象）/ Solve linear model with options object
    fn solve_linear_with_options(
        &self,
        model: &LinearTriadModel,
        options: &super::SolveOptions<'_>,
    ) -> Result<SolverOutput> {
        validate_per_solve_limits(options)?;
        if let Some(callback) = options.solving_status_callback {
            callback(&SolvingStatus::solving(self.name()))?;
        }
        let output = self.solve_linear(model)?;
        if let Some(callback) = options.solving_status_callback {
            let status = SolvingStatus::from_output(self.name(), &output);
            callback(&status)?;
        }
        Ok(output)
    }

    /// 以统一报告和参数求解线性模型 / Solve a linear model as a unified report with options.
    fn solve_linear_report_with_options(
        &self,
        model: &LinearTriadModel,
        options: &super::SolveOptions<'_>,
    ) -> Result<SolveReport<f64>> {
        validate_per_solve_limits(options)?;
        if options.time_limit.is_some()
            || options.node_limit.is_some()
            || options.solution_limit.is_some()
        {
            return Err(CoreError::Solver(SolverError::UnsupportedValueType(
                "this LinearSolver does not expose per-solve time, node, or solution limits"
                    .to_owned(),
            )));
        }
        if options.solution_amount <= 1
            && options.solving_status_callback.is_none()
            && options.cancellation_handle.is_none()
            && options.progress_reporter.is_none()
        {
            return self.solve_linear_report(model);
        }
        emit_progress_start(options.progress_reporter, self.name())?;
        if let Some(handle) = options.cancellation_handle
            && handle.is_cancelled()
        {
            let report = cancelled_solve_report(
                SolverProvenance {
                    solver_id: self.name().to_owned(),
                    backend_name: self.name().to_owned(),
                    ..SolverProvenance::default()
                },
                handle,
            )?;
            emit_progress_terminal(options.progress_reporter, self.name(), &report)?;
            return Ok(report);
        }
        if let Some(callback) = options.solving_status_callback {
            callback(&SolvingStatus::solving(self.name()))?;
        }

        // 报告入口统一管理 callback；backend options 不再携带同一 callback，避免重复上报。
        // The report entry owns callbacks; backend options omit the same callback to avoid duplicates.
        let backend_options = super::SolveOptions {
            solving_status_callback: None,
            ..*options
        };
        if options.solution_amount > 1
            && let Some((output, pool)) = self.solve_linear_with_solution_pool_with_options(
                model,
                options.solution_amount,
                options,
            )?
        {
            let final_status = SolvingStatus::from_output(self.name(), &output);
            let mut report = compatibility_linear_report(
                self.name(),
                model,
                output,
                options.cancellation_handle,
            )?;
            if let Some(solution) = report.solution.as_mut() {
                solution.pool = pool;
            }
            if let Some(callback) = options.solving_status_callback {
                callback(&final_status)?;
            }
            emit_progress_terminal(options.progress_reporter, self.name(), &report)?;
            return Ok(report);
        }
        let output = self.solve_linear_with_options(model, &backend_options)?;
        let final_status = SolvingStatus::from_output(self.name(), &output);
        let report =
            compatibility_linear_report(self.name(), model, output, options.cancellation_handle)?;
        if let Some(callback) = options.solving_status_callback {
            callback(&final_status)?;
        }
        emit_progress_terminal(options.progress_reporter, self.name(), &report)?;
        Ok(report)
    }
}

/// 二次模型求解 trait / Quadratic-model solver trait
pub trait QuadraticSolver: SolverInfo {
    /// 求解二次模型 / Solve quadratic model
    fn solve_quadratic(&self, model: &QuadraticTetradModel) -> Result<SolverOutput>;

    /// 以统一报告求解二次模型 / Solve a quadratic model as a unified report.
    fn solve_quadratic_report(&self, model: &QuadraticTetradModel) -> Result<SolveReport<f64>> {
        compatibility_quadratic_report(self.name(), model, self.solve_quadratic(model)?, None)
    }

    /// 求解二次模型并尝试返回解池（可选）/ Solve quadratic model and optionally return solution pool
    ///
    /// 默认实现返回 `None`，表示求解器未提供原生多解支持 / Default returns `None`, meaning no native multi-solution support.
    fn solve_quadratic_with_solution_pool(
        &self,
        _model: &QuadraticTetradModel,
        _solution_amount: usize,
    ) -> Result<Option<(SolverOutput, Vec<Vec<f64>>)>> {
        Ok(None)
    }

    /// 使用统一参数求解二次模型并尝试返回解池（可选） / Solve a quadratic model with unified options and optionally return a solution pool.
    ///
    /// 默认实现委托旧的解池入口，保留既有 backend 的兼容性；需要原生取消或其它
    /// 求解参数的 backend 应覆盖此方法。
    /// The default delegates to the legacy pool entry for backend compatibility; backends that support native cancellation or other solve options should override this method.
    fn solve_quadratic_with_solution_pool_with_options(
        &self,
        model: &QuadraticTetradModel,
        solution_amount: usize,
        options: &super::SolveOptions<'_>,
    ) -> Result<Option<(SolverOutput, Vec<Vec<f64>>)>> {
        validate_per_solve_limits(options)?;
        self.solve_quadratic_with_solution_pool(model, solution_amount)
    }

    /// 求解二次模型（参数对象）/ Solve quadratic model with options object
    fn solve_quadratic_with_options(
        &self,
        model: &QuadraticTetradModel,
        options: &super::SolveOptions<'_>,
    ) -> Result<SolverOutput> {
        validate_per_solve_limits(options)?;
        if let Some(callback) = options.solving_status_callback {
            callback(&SolvingStatus::solving(self.name()))?;
        }
        let output = self.solve_quadratic(model)?;
        if let Some(callback) = options.solving_status_callback {
            let status = SolvingStatus::from_output(self.name(), &output);
            callback(&status)?;
        }
        Ok(output)
    }

    /// 以统一报告和参数求解二次模型 / Solve a quadratic model as a unified report with options.
    fn solve_quadratic_report_with_options(
        &self,
        model: &QuadraticTetradModel,
        options: &super::SolveOptions<'_>,
    ) -> Result<SolveReport<f64>> {
        validate_per_solve_limits(options)?;
        if options.time_limit.is_some()
            || options.node_limit.is_some()
            || options.solution_limit.is_some()
        {
            return Err(CoreError::Solver(SolverError::UnsupportedValueType(
                "this QuadraticSolver does not expose per-solve time, node, or solution limits"
                    .to_owned(),
            )));
        }
        if options.solution_amount <= 1
            && options.solving_status_callback.is_none()
            && options.cancellation_handle.is_none()
            && options.progress_reporter.is_none()
        {
            return self.solve_quadratic_report(model);
        }
        emit_progress_start(options.progress_reporter, self.name())?;
        if let Some(handle) = options.cancellation_handle
            && handle.is_cancelled()
        {
            let report = cancelled_solve_report(
                SolverProvenance {
                    solver_id: self.name().to_owned(),
                    backend_name: self.name().to_owned(),
                    ..SolverProvenance::default()
                },
                handle,
            )?;
            emit_progress_terminal(options.progress_reporter, self.name(), &report)?;
            return Ok(report);
        }
        if let Some(callback) = options.solving_status_callback {
            callback(&SolvingStatus::solving(self.name()))?;
        }

        // 报告入口统一管理 callback；backend options 不再携带同一 callback，避免重复上报。
        // The report entry owns callbacks; backend options omit the same callback to avoid duplicates.
        let backend_options = super::SolveOptions {
            solving_status_callback: None,
            ..*options
        };
        if options.solution_amount > 1
            && let Some((output, pool)) = self.solve_quadratic_with_solution_pool_with_options(
                model,
                options.solution_amount,
                options,
            )?
        {
            let final_status = SolvingStatus::from_output(self.name(), &output);
            let mut report = compatibility_quadratic_report(
                self.name(),
                model,
                output,
                options.cancellation_handle,
            )?;
            if let Some(solution) = report.solution.as_mut() {
                solution.pool = pool;
            }
            if let Some(callback) = options.solving_status_callback {
                callback(&final_status)?;
            }
            emit_progress_terminal(options.progress_reporter, self.name(), &report)?;
            return Ok(report);
        }
        let output = self.solve_quadratic_with_options(model, &backend_options)?;
        let final_status = SolvingStatus::from_output(self.name(), &output);
        let report = compatibility_quadratic_report(
            self.name(),
            model,
            output,
            options.cancellation_handle,
        )?;
        if let Some(callback) = options.solving_status_callback {
            callback(&final_status)?;
        }
        emit_progress_terminal(options.progress_reporter, self.name(), &report)?;
        Ok(report)
    }
}

/// 组合求解器 trait / Combined solver trait
pub trait Solver: LinearSolver + QuadraticSolver {}

impl<T> Solver for T where T: LinearSolver + QuadraticSolver + ?Sized {}

/// 可配置求解器 Trait / Configurable Solver Trait
pub trait ConfigurableSolver: SolverInfo {
    /// 配置类型 / Configuration type
    type Config;

    /// 获取配置 / Get configuration
    fn config(&self) -> &Self::Config;

    /// 设置配置 / Set configuration
    fn set_config(&mut self, config: Self::Config);

    /// 应用通用配置 / Apply common solver configuration
    fn set_common_config(&mut self, config: &super::SolverConfig)
    where
        for<'a> Self::Config: From<&'a super::SolverConfig>,
    {
        self.set_config(Self::Config::from(config));
    }

    /// 使用通用配置返回新求解器 / Return solver with common configuration
    fn with_common_config(mut self, config: &super::SolverConfig) -> Self
    where
        Self: Sized,
        for<'a> Self::Config: From<&'a super::SolverConfig>,
    {
        self.set_common_config(config);
        self
    }
}

/// 异步求解器 Trait / Async Solver Trait
#[cfg(feature = "async")]
#[async_trait::async_trait]
pub trait AsyncSolver: Send + Sync {
    /// 获取求解器名称 / Get solver name
    fn name(&self) -> &str;

    /// 异步求解线性模型 / Solve linear model asynchronously
    async fn solve_linear(&self, model: &LinearTriadModel) -> Result<SolverOutput>;

    /// 异步求解二次模型 / Solve quadratic model asynchronously
    async fn solve_quadratic(&self, model: &QuadraticTetradModel) -> Result<SolverOutput>;

    /// 异步求解线性模型并返回统一报告 / Solve a linear model asynchronously as a unified report
    async fn solve_linear_report(&self, model: &LinearTriadModel) -> Result<SolveReport<f64>> {
        compatibility_linear_report(self.name(), model, self.solve_linear(model).await?, None)
    }

    /// 异步求解二次模型并返回统一报告 / Solve a quadratic model asynchronously as a unified report
    async fn solve_quadratic_report(
        &self,
        model: &QuadraticTetradModel,
    ) -> Result<SolveReport<f64>> {
        compatibility_quadratic_report(self.name(), model, self.solve_quadratic(model).await?, None)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::model::intermediate::{BasicLinearTriadModel, BasicQuadraticTetradModel};
    use crate::solver::{
        CancellationOrigin, ProblemStatus, SolveHandle, SolverStatus, TerminationReason,
    };
    use std::time::Duration;

    #[derive(Debug)]
    struct DummySolver;

    impl SolverInfo for DummySolver {
        fn name(&self) -> &str {
            "dummy_status_solver"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear, SolverCapability::Quadratic]
        }
    }

    impl LinearSolver for DummySolver {
        fn solve_linear(&self, _model: &LinearTriadModel) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(1.0, vec![2.0]))
        }
    }

    impl QuadraticSolver for DummySolver {
        fn solve_quadratic(&self, _model: &QuadraticTetradModel) -> Result<SolverOutput> {
            Ok(SolverOutput::new(SolverStatus::Optimal).with_solution(vec![3.0]))
        }
    }

    type PoolLimits = Option<(Option<Duration>, Option<usize>, Option<usize>)>;

    #[derive(Debug)]
    struct OptionsAwarePoolSolver {
        linear_handle_seen: Arc<AtomicBool>,
        quadratic_handle_seen: Arc<AtomicBool>,
        linear_limits_seen: Arc<Mutex<PoolLimits>>,
        quadratic_limits_seen: Arc<Mutex<PoolLimits>>,
    }

    impl SolverInfo for OptionsAwarePoolSolver {
        fn name(&self) -> &str {
            "options_aware_pool_solver"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear, SolverCapability::Quadratic]
        }
    }

    impl LinearSolver for OptionsAwarePoolSolver {
        fn solve_linear(&self, _model: &LinearTriadModel) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(1.0, vec![]))
        }

        fn solve_linear_with_solution_pool_with_options(
            &self,
            _model: &LinearTriadModel,
            _solution_amount: usize,
            options: &super::super::SolveOptions<'_>,
        ) -> Result<Option<(SolverOutput, Vec<Vec<f64>>)>> {
            self.linear_handle_seen
                .store(options.cancellation_handle.is_some(), Ordering::SeqCst);
            *self.linear_limits_seen.lock().unwrap() = Some((
                options.time_limit,
                options.node_limit,
                options.solution_limit,
            ));
            Ok(Some((
                SolverOutput::optimal(1.0, vec![]),
                vec![vec![], vec![]],
            )))
        }
    }

    impl QuadraticSolver for OptionsAwarePoolSolver {
        fn solve_quadratic(&self, _model: &QuadraticTetradModel) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(2.0, vec![]))
        }

        fn solve_quadratic_with_solution_pool_with_options(
            &self,
            _model: &QuadraticTetradModel,
            _solution_amount: usize,
            options: &super::super::SolveOptions<'_>,
        ) -> Result<Option<(SolverOutput, Vec<Vec<f64>>)>> {
            self.quadratic_handle_seen
                .store(options.cancellation_handle.is_some(), Ordering::SeqCst);
            *self.quadratic_limits_seen.lock().unwrap() = Some((
                options.time_limit,
                options.node_limit,
                options.solution_limit,
            ));
            Ok(Some((
                SolverOutput::optimal(2.0, vec![]),
                vec![vec![], vec![]],
            )))
        }
    }

    #[derive(Debug)]
    struct LegacyPoolSolver;

    impl SolverInfo for LegacyPoolSolver {
        fn name(&self) -> &str {
            "legacy_pool_solver"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear, SolverCapability::Quadratic]
        }
    }

    impl LinearSolver for LegacyPoolSolver {
        fn solve_linear(&self, _model: &LinearTriadModel) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(1.0, vec![]))
        }

        fn solve_linear_with_solution_pool(
            &self,
            _model: &LinearTriadModel,
            _solution_amount: usize,
        ) -> Result<Option<(SolverOutput, Vec<Vec<f64>>)>> {
            Ok(Some((
                SolverOutput::optimal(1.0, vec![]),
                vec![vec![], vec![]],
            )))
        }
    }

    impl QuadraticSolver for LegacyPoolSolver {
        fn solve_quadratic(&self, _model: &QuadraticTetradModel) -> Result<SolverOutput> {
            Ok(SolverOutput::optimal(2.0, vec![]))
        }

        fn solve_quadratic_with_solution_pool(
            &self,
            _model: &QuadraticTetradModel,
            _solution_amount: usize,
        ) -> Result<Option<(SolverOutput, Vec<Vec<f64>>)>> {
            Ok(Some((
                SolverOutput::optimal(2.0, vec![]),
                vec![vec![], vec![]],
            )))
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct DummyConfig {
        time_limit: Option<Duration>,
    }

    impl From<&super::super::SolverConfig> for DummyConfig {
        fn from(config: &super::super::SolverConfig) -> Self {
            Self {
                time_limit: config.time_limit,
            }
        }
    }

    #[derive(Debug)]
    struct DummyConfigurableSolver {
        config: DummyConfig,
    }

    impl SolverInfo for DummyConfigurableSolver {
        fn name(&self) -> &str {
            "dummy_configurable_solver"
        }

        fn capabilities(&self) -> Vec<SolverCapability> {
            vec![SolverCapability::Linear]
        }
    }

    impl ConfigurableSolver for DummyConfigurableSolver {
        type Config = DummyConfig;

        fn config(&self) -> &Self::Config {
            &self.config
        }

        fn set_config(&mut self, config: Self::Config) {
            self.config = config;
        }
    }

    #[test]
    fn solve_linear_with_options_reports_solving_then_final() {
        let solver = DummySolver;
        let model = LinearTriadModel::from_basic(BasicLinearTriadModel::new("cb_linear"));

        let statuses = Arc::new(Mutex::new(Vec::new()));
        let statuses_for_callback = statuses.clone();
        let callback: super::super::SolvingStatusCallback = Arc::new(move |status| {
            statuses_for_callback.lock().unwrap().push(status.status);
            Ok(())
        });

        let options = super::super::SolveOptions::new().with_solving_callback(Some(&callback));
        let output = solver
            .solve_linear_with_options(&model, &options)
            .expect("linear solve with callback should succeed");
        assert!(output.status.is_optimal());

        let statuses = statuses.lock().unwrap();
        assert_eq!(statuses.len(), 2);
        assert_eq!(statuses[0], SolverStatus::Solving);
        assert_eq!(statuses[1], SolverStatus::Optimal);
    }

    #[test]
    fn solve_quadratic_with_options_reports_solving_then_final() {
        let solver = DummySolver;
        let model =
            QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new("cb_quadratic"));

        let statuses = Arc::new(Mutex::new(Vec::new()));
        let statuses_for_callback = statuses.clone();
        let callback: super::super::SolvingStatusCallback = Arc::new(move |status| {
            statuses_for_callback.lock().unwrap().push(status.status);
            Ok(())
        });

        let options = super::super::SolveOptions::new().with_solving_callback(Some(&callback));
        let output = solver
            .solve_quadratic_with_options(&model, &options)
            .expect("quadratic solve with callback should succeed");
        assert!(output.status.is_optimal());

        let statuses = statuses.lock().unwrap();
        assert_eq!(statuses.len(), 2);
        assert_eq!(statuses[0], SolverStatus::Solving);
        assert_eq!(statuses[1], SolverStatus::Optimal);
    }

    #[test]
    fn report_with_options_emits_one_start_and_one_final_callback() {
        let solver = DummySolver;
        let model = LinearTriadModel::from_basic(BasicLinearTriadModel::new("report_callback"));
        let statuses = Arc::new(Mutex::new(Vec::new()));
        let statuses_for_callback = statuses.clone();
        let callback: super::super::SolvingStatusCallback = Arc::new(move |status| {
            statuses_for_callback.lock().unwrap().push(status.status);
            Ok(())
        });

        let options = super::super::SolveOptions::new().with_solving_callback(Some(&callback));
        let report = solver
            .solve_linear_report_with_options(&model, &options)
            .expect("report solve should succeed");

        assert_eq!(
            report.problem_status,
            super::super::report::ProblemStatus::Feasible
        );
        assert_eq!(
            statuses.lock().unwrap().as_slice(),
            &[SolverStatus::Solving, SolverStatus::Optimal]
        );
    }

    #[test]
    fn report_solution_pool_receives_cancellation_options_for_linear_and_quadratic() {
        let solver = OptionsAwarePoolSolver {
            linear_handle_seen: Arc::new(AtomicBool::new(false)),
            quadratic_handle_seen: Arc::new(AtomicBool::new(false)),
            linear_limits_seen: Arc::new(Mutex::new(None)),
            quadratic_limits_seen: Arc::new(Mutex::new(None)),
        };
        let handle = SolveHandle::new();
        let options = super::super::SolveOptions::new()
            .with_solution_amount(2)
            .with_cancellation_handle(Some(&handle));

        let linear_model =
            LinearTriadModel::from_basic(BasicLinearTriadModel::new("pool_options_linear"));
        let linear_report = solver
            .solve_linear_report_with_options(&linear_model, &options)
            .expect("linear solution-pool report should succeed");
        assert_eq!(
            linear_report
                .solution
                .as_ref()
                .map(|solution| solution.pool.len()),
            Some(2)
        );

        let quadratic_model = QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new(
            "pool_options_quadratic",
        ));
        let quadratic_report = solver
            .solve_quadratic_report_with_options(&quadratic_model, &options)
            .expect("quadratic solution-pool report should succeed");
        assert_eq!(
            quadratic_report
                .solution
                .as_ref()
                .map(|solution| solution.pool.len()),
            Some(2)
        );
        assert!(solver.linear_handle_seen.load(Ordering::SeqCst));
        assert!(solver.quadratic_handle_seen.load(Ordering::SeqCst));

        let limited_options = super::super::SolveOptions::new()
            .with_time_limit(Some(Duration::from_secs(3)))
            .with_node_limit(Some(7))
            .with_solution_limit(Some(11))
            .with_cancellation_handle(Some(&handle));
        solver
            .solve_linear_with_solution_pool_with_options(&linear_model, 2, &limited_options)
            .expect("linear pool options should be forwarded");
        solver
            .solve_quadratic_with_solution_pool_with_options(&quadratic_model, 2, &limited_options)
            .expect("quadratic pool options should be forwarded");
        assert_eq!(
            *solver.linear_limits_seen.lock().unwrap(),
            Some((Some(Duration::from_secs(3)), Some(7), Some(11)))
        );
        assert_eq!(
            *solver.quadratic_limits_seen.lock().unwrap(),
            Some((Some(Duration::from_secs(3)), Some(7), Some(11)))
        );
    }

    #[test]
    fn solution_pool_options_reject_zero_limits_before_backend_dispatch() {
        let solver = LegacyPoolSolver;
        let linear_model =
            LinearTriadModel::from_basic(BasicLinearTriadModel::new("pool_invalid_linear"));
        let quadratic_model = QuadraticTetradModel::from_basic(BasicQuadraticTetradModel::new(
            "pool_invalid_quadratic",
        ));

        for options in [
            super::super::SolveOptions::new().with_time_limit(Some(Duration::ZERO)),
            super::super::SolveOptions::new().with_node_limit(Some(0)),
            super::super::SolveOptions::new().with_solution_limit(Some(0)),
        ] {
            assert!(
                solver
                    .solve_linear_with_solution_pool_with_options(&linear_model, 2, &options)
                    .is_err()
            );
            assert!(
                solver
                    .solve_quadratic_with_solution_pool_with_options(&quadratic_model, 2, &options)
                    .is_err()
            );
        }
    }

    #[test]
    fn report_with_options_emits_structured_progress_start_and_terminal() {
        let solver = DummySolver;
        let model = LinearTriadModel::from_basic(BasicLinearTriadModel::new("report_progress"));
        let snapshots = Arc::new(Mutex::new(Vec::new()));
        let snapshots_for_reporter = Arc::clone(&snapshots);
        let reporter: super::super::SolveProgressReporter = Arc::new(move |snapshot| {
            snapshots_for_reporter
                .lock()
                .unwrap()
                .push(snapshot.clone());
            Ok(())
        });
        let options = super::super::SolveOptions::new().with_progress_reporter(Some(&reporter));

        solver
            .solve_linear_report_with_options(&model, &options)
            .expect("report solve with progress should succeed");

        let snapshots = snapshots.lock().unwrap();
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].stage, SolveStage::Solving);
        assert!(!snapshots[0].terminal);
        assert_eq!(snapshots[1].stage, SolveStage::Completed);
        assert!(snapshots[1].terminal);
        assert_eq!(snapshots[1].overall_progress.as_known(), Some(100.0));
    }

    #[test]
    fn pre_cancelled_report_returns_a_structured_cancelled_terminal() {
        let solver = DummySolver;
        let model = LinearTriadModel::from_basic(BasicLinearTriadModel::new("cancelled_report"));
        let handle = SolveHandle::new();
        assert!(handle.cancel(CancellationOrigin::User));
        let options = super::super::SolveOptions::new().with_cancellation_handle(Some(&handle));

        let report = solver
            .solve_linear_report_with_options(&model, &options)
            .expect("pre-cancelled solve should produce a report");
        assert_eq!(report.problem_status, ProblemStatus::Unknown);
        assert_eq!(report.termination_reason, TerminationReason::Cancelled);
        assert_eq!(
            report.diagnostics.extensions.get("cancellation.origin"),
            Some(&"USER".to_owned())
        );
        assert!(!report.has_incumbent());
    }

    #[test]
    fn configurable_solver_accepts_common_config_when_backend_config_can_convert() {
        let common =
            super::super::SolverConfig::new("dummy").with_time_limit(Duration::from_secs(7));
        let solver = DummyConfigurableSolver {
            config: DummyConfig { time_limit: None },
        }
        .with_common_config(&common);

        assert_eq!(solver.config().time_limit, Some(Duration::from_secs(7)));
    }
}
