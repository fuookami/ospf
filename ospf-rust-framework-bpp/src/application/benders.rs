use std::time::{Duration, Instant};
use ospf_rust_core::error::{CoreError, Result, SolverErrorClass};
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::solver::{ProblemStatus, SolveFingerprints, SolveHandle, SolveReport, SolveSolution, TerminationReason,
    linear_model_fingerprint, linear_model_mapping};
use ospf_rust_framework::solver::{FixedRecourseAlgorithm, FixedRecourseConfig, FixedRecourseResult, FixedRecourseStop,
    LinearBendersPartition, LinearBendersPartitionOptions};
use ospf_rust_quantities::unit::UnitTrait;
use super::{DiscreteLoadingApplicationService, DiscreteLoadingSolveOptions, DiscreteLoadingSolveResult, RegisteredDiscreteLoading, invalid};

/// 安全分解与独立普通 MILP 回退选项。 / Safe decomposition and independent ordinary-MILP fallback options.
#[derive(Debug, Clone, Default)]
pub struct DiscreteLoadingBendersOptions {
    /// 完整展开变量的显式阶段；默认连续可行性追索。 / Explicit expanded-variable stages; defaults to continuous feasibility recourse.
    pub partition: LinearBendersPartitionOptions,
    /// 包含回退在内的总预算和分解配置。 / Total budget including fallback, and decomposition configuration.
    pub config: FixedRecourseConfig,
    /// 证书／能力等非取消失败时允许全新工厂普通 MILP。 / Allow fresh-factory ordinary MILP after non-cancellation certificate/capability failures.
    pub fallback_to_milp: bool,
}

/// 源模型领域结果和不被重标记的分解证据。 / Source-model domain result and decomposition evidence that is never relabeled.
pub struct DiscreteLoadingBendersResult<U: UnitTrait> {
    /// 完整源模型复核后的方案；分解合并报告保守为 Feasible。 / Fully source-validated plan; merged decomposition reports conservatively remain Feasible.
    pub result: DiscreteLoadingSolveResult<U>,
    /// 实际分解状态和 master／sub 报告。 / Actual decomposition state and master/subproblem reports.
    pub decomposition: Option<FixedRecourseResult>,
    /// 可回退的结构化分解能力／技术错误，保留原始分类。 / Recoverable structured decomposition capability/technical error, retaining its original classification.
    pub decomposition_error: Option<CoreError>,
    /// 实际回退报告，独立保存，即使最终保留更好的原方案。 / Actual fallback report, retained separately even when the better original incumbent is kept.
    pub fallback_report: Option<SolveReport<f64>>,
    /// 是否实际启动独立普通 MILP 回退。 / Whether independent ordinary-MILP fallback actually started.
    pub fallback_used: bool,
}

pub(super) fn same_numeric_model(left: &LinearTriadModel, right: &LinearTriadModel) -> bool {
    left.c == right.c && left.lb == right.lb && left.ub == right.ub && left.b == right.b
        && left.basic.var_types == right.basic.var_types
        && left.A.rows.len() == right.A.rows.len()
        && left.A.rows.iter().zip(&right.A.rows).all(|(left, right)| left.entries == right.entries)
}

pub(super) fn same_domain_bindings<U: UnitTrait>(left: &RegisteredDiscreteLoading<f64, U>, right: &RegisteredDiscreteLoading<f64, U>) -> bool {
    let left_context = left.context();
    let right_context = right.context();
    let left_index = |variable| left.model().tokens().iter().position(|token| token.id() == variable);
    let right_index = |variable| right.model().tokens().iter().position(|token| token.id() == variable);
    left_context.problem().items.iter().map(|item| (&item.id, item.required, &item.fixed_position_id))
        .eq(right_context.problem().items.iter().map(|item| (&item.id, item.required, &item.fixed_position_id)))
        && left_context.aggregation().assignments.iter().map(|(key, variable)| (key, left_index(*variable)))
            .eq(right_context.aggregation().assignments.iter().map(|(key, variable)| (key, right_index(*variable))))
        && left_context.aggregation().occupied.iter().map(|(key, variable)| (key, left_index(*variable)))
            .eq(right_context.aggregation().occupied.iter().map(|(key, variable)| (key, right_index(*variable))))
        && left_context.aggregation().layouts.iter().map(|(key, variable)| (key, left_index(*variable)))
            .eq(right_context.aggregation().layouts.iter().map(|(key, variable)| (key, right_index(*variable))))
}

fn reason(stop: FixedRecourseStop) -> TerminationReason {
    match stop {
        FixedRecourseStop::Converged => TerminationReason::Completed,
        FixedRecourseStop::Cancelled => TerminationReason::Cancelled,
        FixedRecourseStop::TimeLimit => TerminationReason::TimeLimit,
        FixedRecourseStop::IterationLimit => TerminationReason::IterationLimit,
        FixedRecourseStop::BackendFailure(reason) => reason,
        FixedRecourseStop::MasterTermination(_, reason) | FixedRecourseStop::SubproblemTermination(_, reason) => reason,
        _ => TerminationReason::Suboptimal,
    }
}

pub(super) fn recoverable(error: &CoreError) -> bool {
    matches!(error.solver_error_class(), SolverErrorClass::Unsupported | SolverErrorClass::Environment
        | SolverErrorClass::License | SolverErrorClass::Backend | SolverErrorClass::Numerical)
}

fn verified_source_result<U: UnitTrait>(
    registration: RegisteredDiscreteLoading<f64, U>,
    decomposition: &FixedRecourseResult,
    termination: TerminationReason,
    tolerance: f64,
) -> Result<DiscreteLoadingSolveResult<U>> {
    let snapshot = registration.model().linear_model_snapshot()?;
    let compilation = snapshot.compile_flt64()?;
    let mut solution = None;
    let mut incumbent = None;
    if decomposition.objective.is_some() {
        let ordered = snapshot.variables.iter().map(|state| decomposition.values.get(&state.variable).copied()
            .ok_or_else(|| invalid("missing source incumbent variable"))).collect::<Result<Vec<_>>>()?;
        solution = Some(registration.context().extract_solution(registration.model(), &ordered, &tolerance)?);
        let evaluation = snapshot.evaluate_objective_flt64(&decomposition.values)?;
        let mut candidate = SolveSolution::vector(ordered);
        candidate.objective_value = Some(evaluation.variable_value()?);
        incumbent = Some(candidate);
    }
    let mut builder = SolveReport::builder(if incumbent.is_some() { ProblemStatus::Feasible } else { ProblemStatus::Unknown }, termination)
        .model_mapping(Some(linear_model_mapping(&compilation.model)?))
        .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(&compilation.model)?), ..Default::default() });
    if let Some(incumbent) = incumbent { builder = builder.solution(incumbent); }
    Ok(DiscreteLoadingSolveResult { report: builder.build()?, registration: Some(registration), solution, objective: decomposition.objective })
}

/// 独立安全 Benders 应用入口；不与列生成混合，也不隐式跨域转换证明。
/// Independent safe-Benders application entry; never mixed with CG and never silently converts proof domains.
pub struct DiscreteLoadingBendersApplicationService;

impl DiscreteLoadingBendersApplicationService {
    /// 工厂必须可重入并创建同一完整业务模型的全新会话；回退会独立核对数值结构和新 owner。
    /// The factory must be reentrant and create fresh sessions for the same complete business model; fallback checks numeric structure and a fresh owner.
    /// 能力不支持、环境／许可证、后端与数值错误可回退；输入、建模、合同与回调错误直接传播。
    /// Unsupported capability, environment/license, backend and numerical errors may fall back; input, modeling, contract and callback errors propagate.
    /// 取消、中断及无剩余预算绝不启动回退。 / Cancellation, interruption and exhausted budgets never launch fallback.
    pub fn solve<U, F, S>(
        mut factory: F,
        options: DiscreteLoadingBendersOptions,
        handle: &SolveHandle,
        mut solve: S,
    ) -> Result<DiscreteLoadingBendersResult<U>>
    where
        U: UnitTrait,
        F: FnMut() -> Result<RegisteredDiscreteLoading<f64, U>>,
        S: FnMut(&LinearTriadModel, Duration, &SolveHandle) -> Result<SolveReport<f64>>,
    {
        let started = Instant::now();
        let config = options.config;
        if config.iteration_limit == 0 || !config.tolerance.is_finite() || config.tolerance <= 0.0 || config.tolerance >= 0.5 {
            return Err(invalid("invalid Benders budget or tolerance"));
        }
        let terminal = |reason| -> Result<DiscreteLoadingBendersResult<U>> {
            Ok(DiscreteLoadingBendersResult { result: DiscreteLoadingSolveResult {
                report: SolveReport::builder(ProblemStatus::Unknown, reason).build()?, registration: None, solution: None, objective: None,
            }, decomposition: None, decomposition_error: None, fallback_report: None, fallback_used: false })
        };
        if handle.is_cancelled() { return terminal(TerminationReason::Cancelled); }
        if config.time_limit.is_zero() { return terminal(TerminationReason::TimeLimit); }
        let registration = factory()?;
        let snapshot = registration.model().linear_model_snapshot()?;
        let attempt = LinearBendersPartition::build(registration.model(), options.partition).and_then(|partition| {
            let remaining = config.time_limit.saturating_sub(started.elapsed());
            FixedRecourseAlgorithm::solve(registration.model(), &partition, FixedRecourseConfig { time_limit: remaining, ..config }, handle, &mut solve)
        });
        registration.model().require_linear_model_revision(snapshot.revision)?;
        let (decomposition, decomposition_error) = match attempt {
            Ok(decomposition) => (Some(decomposition), None),
            Err(error) if options.fallback_to_milp && recoverable(&error) && !handle.is_cancelled()
                && !config.time_limit.saturating_sub(started.elapsed()).is_zero() => (None, Some(error)),
            Err(error) => return Err(error),
        };
        let remaining = config.time_limit.saturating_sub(started.elapsed());
        if options.fallback_to_milp && (decomposition.as_ref().is_some_and(|result| result.stop.allows_fallback()) || decomposition_error.is_some())
            && !handle.is_cancelled() && !remaining.is_zero()
        {
            let original = snapshot.compile_flt64()?;
            let remaining = config.time_limit.saturating_sub(started.elapsed());
            let mut invoked = false;
            let attempt = DiscreteLoadingApplicationService::solve(|| {
                let fresh = factory()?;
                let snapshot = fresh.model().linear_model_snapshot()?;
                let compiled = snapshot.compile_flt64()?;
                if snapshot.revision.owner == original.revision.owner || compiled.objective_constant != original.objective_constant
                    || !same_numeric_model(&original.model, &compiled.model) || !same_domain_bindings(&registration, &fresh)
                { return Err(invalid("fallback factory must create a fresh equivalent complete source model")); }
                Ok(fresh)
            }, DiscreteLoadingSolveOptions { time_limit: remaining, tolerance: config.tolerance }, handle, |model, remaining, handle| {
                invoked = true;
                solve(model, remaining, handle)
            });
            let result = match attempt {
                Ok(result) => result,
                Err(error) if invoked && recoverable(&error) && decomposition.as_ref().is_some_and(|previous| previous.objective.is_some()) => {
                    let terminal = if handle.is_cancelled() { TerminationReason::Cancelled }
                        else if error.solver_error_class() == SolverErrorClass::Numerical { TerminationReason::NumericalFailure }
                        else { TerminationReason::BackendFailure };
                    let result = verified_source_result(registration, decomposition.as_ref().ok_or_else(|| invalid("missing retained decomposition"))?, terminal, config.tolerance)?;
                    return Ok(DiscreteLoadingBendersResult { result, decomposition, decomposition_error: Some(error), fallback_report: None, fallback_used: true });
                }
                Err(error) => return Err(error),
            };
            let fallback_used = result.registration.is_some();
            let fallback_report = Some(result.report.clone());
            if result.report.problem_status == ProblemStatus::Infeasible && decomposition.as_ref().is_some_and(|previous| previous.objective.is_some()) {
                return Err(invalid("fallback source infeasibility contradicts the validated incumbent"));
            }
            if result.report.is_optimal() && decomposition.as_ref().and_then(|previous| previous.objective).zip(result.objective)
                .is_some_and(|(old, new)| old < new - config.tolerance * old.abs().max(new.abs()).max(1.0)) {
                return Err(invalid("fallback source optimality contradicts the validated incumbent"));
            }
            if let Some(previous) = &decomposition
                && !result.report.is_optimal()
                && !matches!(result.report.problem_status, ProblemStatus::Unbounded | ProblemStatus::InfeasibleOrUnbounded)
                && previous.objective.is_some_and(|old| result.objective.is_none_or(|new| old <= new))
            {
                let result = verified_source_result(registration, previous, result.report.termination_reason, config.tolerance)?;
                return Ok(DiscreteLoadingBendersResult { result, decomposition, decomposition_error, fallback_report, fallback_used });
            }
            return Ok(DiscreteLoadingBendersResult { result, decomposition, decomposition_error, fallback_report, fallback_used });
        }
        if let Some(error) = decomposition_error { return Err(error); }
        let decomposition = decomposition.ok_or_else(|| invalid("missing decomposition result"))?;
        let result = verified_source_result(registration, &decomposition, reason(decomposition.stop), config.tolerance)?;
        Ok(DiscreteLoadingBendersResult { result, decomposition: Some(decomposition), decomposition_error: None, fallback_report: None, fallback_used: false })
    }
}
