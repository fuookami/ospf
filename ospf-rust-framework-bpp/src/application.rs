//! 共享离散装载的普通求解编排。 / Ordinary solve orchestration for shared discrete loading.

use std::time::{Duration, Instant};
use ospf_rust_core::error::{ModelError, Result};
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::model::{LinearModelValue, LinearObjectiveEvaluationFlt64, MetaModel};
use ospf_rust_core::solver::audit::linear_model_mapping;
use ospf_rust_core::solver::fingerprint::linear_model_fingerprint;
use ospf_rust_core::solver::{ProblemStatus, SolveHandle, SolveReport, TerminationReason};
use ospf_rust_quantities::unit::UnitTrait;
use crate::{DiscreteLoadingContext, DiscreteLoadingSolution};

mod partition;
mod benders;
mod certification;

pub use partition::*;
pub use benders::*;
pub use certification::*;

fn invalid(message: &str) -> ospf_rust_core::error::CoreError {
    ModelError::InvalidConstraint(format!("discrete loading solve: {message}")).into()
}

/// 完整领域注册，普通／分区／回退均由独立工厂创建，不共享模型会话。
/// Complete domain registration, created by independent factories for ordinary, partition and fallback paths without sharing model sessions.
pub struct RegisteredDiscreteLoading<V: LinearModelValue, U: UnitTrait> {
    context: DiscreteLoadingContext<V, U>,
    model: MetaModel<V>,
}

impl<U: UnitTrait> RegisteredDiscreteLoading<f64, U> {
    /// 接收同一模型的完整已注册 context，包含全部扩展和附加重量联动。
    /// Accept a fully registered context belonging to the same model, including extensions and additional-weight links.
    pub fn new(context: DiscreteLoadingContext<f64, U>, model: MetaModel<f64>) -> Result<Self> {
        context.require_model(&model)?;
        model.linear_model_snapshot()?;
        Ok(Self { context, model })
    }

    /// 完整不可变源模型。 / Complete immutable source model.
    pub fn model(&self) -> &MetaModel<f64> { &self.model }

    /// 完整领域 context。 / Complete domain context.
    pub fn context(&self) -> &DiscreteLoadingContext<f64, U> { &self.context }
}

/// 总预算和同域复核容差。 / Total budget and same-domain validation tolerance.
#[derive(Debug, Clone, Copy)]
pub struct DiscreteLoadingSolveOptions {
    /// 工厂及全部求解共享时限。 / Shared deadline across factory and all solves.
    pub time_limit: Duration,
    /// 原始 f64 源模型验证容差。 / Validation tolerance for the original f64 source model.
    pub tolerance: f64,
}

impl Default for DiscreteLoadingSolveOptions {
    fn default() -> Self { Self { time_limit: Duration::from_secs(60), tolerance: 1e-7 } }
}

/// 原报告与独立验证的领域方案；不会另造状态或证明系统。
/// Original report with an independently validated domain plan, without another status/proof system.
pub struct DiscreteLoadingSolveResult<U: UnitTrait> {
    /// 实际求解模型的 Core 统一报告。 / Core unified report for the actual solved model.
    pub report: SolveReport<f64>,
    /// 完整已注册模型；取消／预算前置终态时为空。 / Complete registered model; absent on a pre-solve cancellation/budget terminal.
    pub registration: Option<RegisteredDiscreteLoading<f64, U>>,
    /// 已验证领域解。 / Validated domain solution.
    pub solution: Option<DiscreteLoadingSolution>,
    /// 源模型最小化目标，包含只计一次的常数。 / Minimizing source-model objective, including its constant exactly once.
    pub objective: Option<f64>,
}

/// 普通 MILP 应用入口，领域变量、行和目标均来自工厂注册的 MetaModel。
/// Ordinary MILP application entry; every domain variable, row and objective comes from the factory-registered MetaModel.
pub struct DiscreteLoadingApplicationService;

impl DiscreteLoadingApplicationService {
    /// 在明确的同域 f64 边界求解并完整复核；不接受跨数值域证明提升。
    /// Solve and fully validate at an explicit same-domain f64 boundary, without cross-domain proof promotion.
    pub fn solve<U, F, S>(
        factory: F,
        options: DiscreteLoadingSolveOptions,
        handle: &SolveHandle,
        solve: S,
    ) -> Result<DiscreteLoadingSolveResult<U>>
    where
        U: UnitTrait,
        F: FnOnce() -> Result<RegisteredDiscreteLoading<f64, U>>,
        S: FnOnce(&LinearTriadModel, Duration, &SolveHandle) -> Result<SolveReport<f64>>,
    {
        let started = Instant::now();
        if !options.tolerance.is_finite() || !(0.0..0.5).contains(&options.tolerance) { return Err(invalid("invalid tolerance")); }
        let terminal = |reason| Ok(DiscreteLoadingSolveResult { report: SolveReport::builder(ProblemStatus::Unknown, reason).build()?,
            registration: None, solution: None, objective: None });
        if handle.is_cancelled() { return terminal(TerminationReason::Cancelled); }
        if options.time_limit.is_zero() { return terminal(TerminationReason::TimeLimit); }
        let registration = factory()?;
        let compilation = registration.model.linear_model_snapshot()?.compile_flt64()?;
        let remaining = options.time_limit.saturating_sub(started.elapsed());
        if handle.is_cancelled() { return terminal(TerminationReason::Cancelled); }
        if remaining.is_zero() { return terminal(TerminationReason::TimeLimit); }
        let report = solve(&compilation.model, remaining, handle)?;
        report.validate()?;
        if report.model_mapping.as_ref() != Some(&linear_model_mapping(&compilation.model)?)
            || report.fingerprints.model.as_ref() != Some(&linear_model_fingerprint(&compilation.model)?)
        { return Err(invalid("report does not identify the actual compiled source model")); }
        let mut solution = None;
        let mut objective = None;
        if let Some(incumbent) = &report.solution {
            solution = Some(registration.context.extract_solution(&registration.model, &incumbent.values, &options.tolerance)?);
            let evaluation = LinearObjectiveEvaluationFlt64::new(compilation.model.c.iter().copied().zip(incumbent.values.iter().copied()), compilation.objective_constant)?;
            evaluation.validate_reported(incumbent.objective_value.or(incumbent.objective), options.tolerance.max(1e-7))?;
            objective = Some(evaluation.total_value()?);
        }
        Ok(DiscreteLoadingSolveResult { report, registration: Some(registration), solution, objective })
    }
}

#[cfg(test)]
mod tests;
