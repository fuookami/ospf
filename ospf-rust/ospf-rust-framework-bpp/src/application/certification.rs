//! 完整源 MILP 收尾，不将加速模型的证明提升为源证明。 / Complete-source MILP finishing without promoting accelerator proofs.

use std::time::{Duration, Instant};
use ospf_rust_core::error::{CoreError, Result, SolverErrorClass};
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::model::LinearObjectiveEvaluationFlt64;
use ospf_rust_core::solver::audit::evaluate_linear_solution;
use ospf_rust_core::solver::{ProblemStatus, ProofStatus, SolveReport, SolveHandle, SolveSolution, SolutionPresence, TerminationReason,
    linear_model_fingerprint, linear_model_mapping};
use ospf_rust_quantities::unit::UnitTrait;
use super::{DiscreteLoadingApplicationService, DiscreteLoadingSolveOptions, DiscreteLoadingSolveResult, RegisteredDiscreteLoading, invalid};
use super::benders::{recoverable, same_domain_bindings, same_numeric_model};

/// 完整源收尾结果，实际认证报告与最终保留方案分别保存。 / Complete-source finishing result, separating the actual certification report from the retained plan.
pub struct DiscreteLoadingCertificationResult<U: UnitTrait> {
    /// 最终完整源报告、注册与已验证方案。 / Final complete-source report, registration and validated plan.
    pub result: DiscreteLoadingSolveResult<U>,
    /// 实际完整源求解报告；预先取消或耗时无求解时为空。 / Actual complete-source solve report; absent when no solve starts before cancellation or timeout.
    pub certification_report: Option<SolveReport<f64>>,
    /// 收尾后端的可恢复错误，不丢弃既有方案。 / Recoverable finishing-backend error, without discarding an existing plan.
    pub certification_error: Option<CoreError>,
}

/// 在剩余预算内使用全新等价源模型收尾，复用已验证向量作 warm start。
/// Finish on a fresh equivalent source model within the remaining budget, reusing the validated vector as a warm start.
pub struct DiscreteLoadingCertificationService;

impl DiscreteLoadingCertificationService {
    /// 只复核原注册、源身份、完整可行向量和目标，不创建模型、不启动求解、不改写终止原因。
    /// Validate the original registration, source identity, complete feasible vector and objective without creating models, solving or rewriting termination.
    pub fn validate<U: UnitTrait>(previous: &mut DiscreteLoadingSolveResult<U>, tolerance: f64) -> Result<()> {
        if !tolerance.is_finite() || tolerance <= 0.0 || tolerance >= 0.5 {
            return Err(invalid("invalid source certification tolerance"));
        }
        previous.report.validate()?;
        let registration = previous.registration.as_ref().ok_or_else(|| invalid("source certification requires an original registration"))?;
        let original = registration.model().linear_model_snapshot()?.compile_flt64()?;
        let mapping = linear_model_mapping(&original.model)?;
        let fingerprint = linear_model_fingerprint(&original.model)?;
        if previous.report.model_mapping.as_ref().is_some_and(|actual| actual != &mapping)
            || previous.report.fingerprints.model.as_ref().is_some_and(|actual| actual != &fingerprint)
            || ((previous.report.problem_status != ProblemStatus::Unknown || previous.report.solution.is_some() || previous.report.proof.is_some())
                && (previous.report.model_mapping.is_none() || previous.report.fingerprints.model.is_none())) {
            return Err(invalid("previous report must identify its actual source registration"));
        }
        previous.report.model_mapping = Some(mapping);
        previous.report.fingerprints.model = Some(fingerprint);
        if let Some(incumbent) = &previous.report.solution {
            previous.solution = Some(registration.context().extract_solution(registration.model(), &incumbent.values, &tolerance)?);
            let evaluation = LinearObjectiveEvaluationFlt64::new(original.model.c.iter().copied().zip(incumbent.values.iter().copied()), original.objective_constant)?;
            evaluation.validate_reported(incumbent.objective_value.or(incumbent.objective), tolerance.max(1e-7))?;
            previous.objective = Some(evaluation.total_value()?);
        } else if previous.solution.is_some() || previous.objective.is_some() {
            return Err(invalid("source plan requires a complete report incumbent"));
        }
        Ok(())
    }

    /// 工厂必须保留完整数值模型及领域绑定；旧 token、对偶和证明不能搬入新模型。
    /// The factory must preserve the complete numeric model and domain bindings; old tokens, duals and proofs never enter the fresh model.
    pub fn solve<U, F, S>(
        mut previous: DiscreteLoadingSolveResult<U>,
        factory: F,
        options: DiscreteLoadingSolveOptions,
        handle: &SolveHandle,
        mut solve: S,
    ) -> Result<DiscreteLoadingCertificationResult<U>>
    where
        U: UnitTrait,
        F: FnOnce() -> Result<RegisteredDiscreteLoading<f64, U>>,
        S: FnMut(&LinearTriadModel, Duration, &SolveHandle) -> Result<SolveReport<f64>>,
    {
        let started = Instant::now();
        Self::validate(&mut previous, options.tolerance)?;
        let registration = previous.registration.as_ref().ok_or_else(|| invalid("source certification requires an original registration"))?;
        let original = registration.model().linear_model_snapshot()?.compile_flt64()?;
        let warm = previous.report.solution.as_ref().map(|incumbent| incumbent.values.clone());
        let certified_terminal = matches!(previous.report.problem_status,
            ProblemStatus::Infeasible | ProblemStatus::Unbounded | ProblemStatus::InfeasibleOrUnbounded)
            && previous.report.proof.as_ref().is_some_and(|proof| proof.status == ProofStatus::Verified);
        if previous.report.is_optimal() || certified_terminal {
            return Ok(DiscreteLoadingCertificationResult { result: previous, certification_report: None, certification_error: None });
        }
        let stopped = if handle.is_cancelled() { Some(TerminationReason::Cancelled) }
            else if matches!(previous.report.termination_reason, TerminationReason::Cancelled | TerminationReason::Interrupted) { Some(previous.report.termination_reason) }
            else if options.time_limit.is_zero() { Some(TerminationReason::TimeLimit) } else { None };
        if let Some(reason) = stopped {
            previous.report.termination_reason = reason;
            previous.report.validate()?;
            return Ok(DiscreteLoadingCertificationResult { result: previous, certification_report: None, certification_error: None });
        }
        let mut invoked = false;
        let attempt = DiscreteLoadingApplicationService::solve(|| {
            let fresh = factory()?;
            let snapshot = fresh.model().linear_model_snapshot()?;
            let compiled = snapshot.compile_flt64()?;
            if snapshot.revision.owner == original.revision.owner || compiled.objective_constant != original.objective_constant
                || !same_numeric_model(&original.model, &compiled.model) || !same_domain_bindings(registration, &fresh)
            { return Err(invalid("certification factory must create a fresh equivalent complete source model")); }
            Ok(fresh)
        }, DiscreteLoadingSolveOptions { time_limit: options.time_limit.saturating_sub(started.elapsed()), ..options }, handle, |model, remaining, handle| {
            // 仅复制完整可行数值，不复制旧身份、受限池证明或对偶。 / Copy only complete feasible values, never old identity, restricted-pool proofs or duals.
            if let Some(values) = &warm {
                for (token, value) in model.basic.variables.iter().zip(values) { token.set_result(*value); }
            }
            invoked = true;
            solve(model, remaining, handle)
        });
        let mut candidate = match attempt {
            Ok(candidate) => candidate,
            Err(error) if invoked && recoverable(&error) => {
                previous.report.termination_reason = if handle.is_cancelled() { TerminationReason::Cancelled }
                    else if error.solver_error_class() == SolverErrorClass::Numerical { TerminationReason::NumericalFailure }
                    else { TerminationReason::BackendFailure };
                previous.report.validate()?;
                return Ok(DiscreteLoadingCertificationResult { result: previous, certification_report: None, certification_error: Some(error) });
            }
            Err(error) => return Err(error),
        };
        let certification_report = invoked.then(|| candidate.report.clone());
        if warm.is_some() && candidate.report.problem_status == ProblemStatus::Infeasible {
            return Err(invalid("source infeasibility contradicts the validated incumbent"));
        }
        if candidate.report.is_optimal() {
            if previous.objective.zip(candidate.objective).is_some_and(|(old, new)| old < new - options.tolerance * old.abs().max(new.abs()).max(1.0)) {
                return Err(invalid("source optimality contradicts a better validated incumbent"));
            }
        } else if !matches!(candidate.report.problem_status, ProblemStatus::Unbounded | ProblemStatus::InfeasibleOrUnbounded)
            && previous.objective.is_some_and(|old| candidate.objective.is_none_or(|new| old <= new)) {
            if let Some(fresh) = candidate.registration.as_ref() {
                let values = warm.ok_or_else(|| invalid("missing source incumbent values"))?;
                candidate.solution = Some(fresh.context().extract_solution(fresh.model(), &values, &options.tolerance)?);
                candidate.objective = previous.objective;
                let mut incumbent = SolveSolution::vector(values);
                incumbent.objective_value = previous.report.solution.as_ref().and_then(|solution| solution.objective_value.or(solution.objective));
                candidate.report.problem_status = ProblemStatus::Feasible;
                candidate.report.solution_presence = SolutionPresence::Incumbent;
                candidate.report.proof = None;
                candidate.report.solution = Some(incumbent);
                let compiled = fresh.model().linear_model_snapshot()?.compile_flt64()?;
                candidate.report.diagnostics = evaluate_linear_solution(&compiled.model,
                    &candidate.report.solution.as_ref().ok_or_else(|| invalid("missing retained incumbent"))?.values, options.tolerance)?;
                candidate.report.diagnostics.extensions.insert("selected_incumbent".into(), "validated_accelerator".into());
                candidate.report.trace = Default::default();
                let statistics = &mut candidate.report.statistics;
                statistics.absolute_gap = candidate.report.solution.as_ref().and_then(|solution| solution.objective_value)
                    .zip(statistics.best_bound_value).map(|(objective, bound)| (objective - bound).abs());
                statistics.relative_gap = statistics.absolute_gap.zip(candidate.report.solution.as_ref().and_then(|solution| solution.objective_value))
                    .map(|(gap, objective)| gap / objective.abs().max(1.0));
                candidate.report.validate()?;
            } else {
                previous.report.termination_reason = candidate.report.termination_reason;
                previous.report.validate()?;
                candidate = previous;
            }
        }
        Ok(DiscreteLoadingCertificationResult { result: candidate, certification_report, certification_error: None })
    }
}
