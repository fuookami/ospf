use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};
use ospf_rust_core::error::{ModelError, Result};
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::model::LinearObjectiveEvaluationFlt64;
use ospf_rust_core::solver::audit::linear_model_mapping;
use ospf_rust_core::solver::cancellation::SolveHandle;
use ospf_rust_core::solver::fingerprint::linear_model_fingerprint;
use ospf_rust_core::solver::report::{require_infeasibility_certificate_for_model, require_optimal_lp_certificate_for_linear_model};
use ospf_rust_core::solver::{ProblemStatus, SolveReport, TerminationReason};
use ospf_rust_quantities::unit::UnitTrait;
use crate::domain::*;

fn invalid(message: &str) -> ospf_rust_core::error::CoreError {
    ModelError::InvalidConstraint(format!("BPP1D solve contract: {message}")).into()
}

/// 求解总预算，候选搜索与所有 LP／final 共享总时限。 / Solve budgets, sharing one deadline across generation, every LP and final.
#[derive(Debug, Clone, Copy)]
pub struct Bpp1dSolveConfig {
    /// 候选搜索预算。 / Candidate search budget.
    pub generation: PatternGenerationConfig,
    /// 最大定价轮数。 / Maximum pricing rounds.
    pub iteration_limit: usize,
    /// 每轮最多激活的列数。 / Maximum columns activated per round.
    pub columns_per_iteration: usize,
    /// 初始列与结果展开件数上限。 / Seed and output piece-expansion limit.
    pub expansion_limit: u64,
    /// 完整调用的时限。 / Deadline for the entire call.
    pub time_limit: Duration,
    /// 同域验证容差，必须位于 [0, 0.5)。 / Same-domain validation tolerance in [0, 0.5).
    pub tolerance: f64,
}

impl Default for Bpp1dSolveConfig {
    fn default() -> Self {
        Self { generation: Default::default(), iteration_limit: 32, columns_per_iteration: 32,
            expansion_limit: 100_000, time_limit: Duration::from_secs(60), tolerance: 1e-7 }
    }
}

/// 领域结果与原始统一报告；报告仅证明其实际候选模型，不自动证明整个装箱问题。
/// Domain result with the original unified report; the report certifies its actual candidate model, not automatically the entire packing problem.
pub struct Bpp1dSolution<L: UnitTrait> {
    /// 原始最终模型报告；取消或无预算时为相应执行终态。 / Original final-model report, or cancellation/budget terminal.
    pub report: SolveReport<f64>,
    /// 独立验证后的物理结果；没有 incumbent 时为空。 / Independently validated physical result; absent without an incumbent.
    pub packing: Option<PackingResult<f64, L>>,
    /// 本次有界搜索是否穷尽候选。 / Whether this bounded search exhausted the candidates.
    pub candidate_complete: bool,
    /// final 使用的 seeds + generated 去重总列数。 / Total deduplicated seed/generated columns used by final.
    pub column_count: usize,
    /// 已验证的 LP 求解轮数。 / Number of validated LP rounds.
    pub pricing_iterations: usize,
    /// 是否以当前完整行的有效对偶证明有限池中无改善列。 / Whether valid current full-row duals proved no improving column in the finite pool.
    pub pricing_converged: bool,
    /// 同域、默认规则、完备候选及整数证据同时成立时才为真。 / True only with same-domain evidence, default rules, complete candidates and integer proof.
    pub globally_optimal: bool,
    /// 同域、默认规则、完备候选及不可行证据同时成立时才为真。 / True only with same-domain evidence, default rules, complete candidates and infeasibility proof.
    pub globally_infeasible: bool,
    /// 下游提取指标。 / Downstream extracted indicators.
    pub indicators: BTreeMap<String, String>,
}

/// 普通一维装箱编排，f64 仅是明确的同域 solver 边界。
/// Ordinary one-dimensional packing orchestration; f64 is an explicit same-domain solver boundary.
pub struct Bpp1dMilp;

/// 有界有限池列激活编排，保留完整扩展矩阵与 final 总池。
/// Bounded finite-pool column-activation orchestration, preserving the complete extension matrix and final pool.
pub struct Bpp1dColumnGeneration;

macro_rules! entry {
    ($entry:ty, $cg:expr) => {
        impl $entry {
            /// 使用默认领域规则求解，callback 必须报告实际接收模型的完整统一报告。
            /// Solve default domain rules; the callback must return a complete unified report for its actual input model.
            pub fn solve<L, W, F>(
                packing: PackingContext<f64, L, W>,
                config: Bpp1dSolveConfig,
                handle: &SolveHandle,
                solve: F,
            ) -> Result<Bpp1dSolution<L>>
            where
                L: UnitTrait + Clone,
                W: UnitTrait + Clone,
                F: FnMut(&LinearTriadModel, Duration, &SolveHandle) -> Result<SolveReport<f64>>,
            {
                run(packing, NoPatternExtension, &AcceptAllPatterns, config, handle, solve, RunScope { cg: $cg, default_rules: true })
            }

            /// 注入统一建模与候选策略；默认仅返回已复核 Feasible，不擅自证明自定义扩展的跨阶段等价性。
            /// Inject shared modeling and candidate policies; conservatively return validated feasibility without claiming cross-stage equivalence of custom extensions.
            pub fn solve_with_extension<L, W, E, F>(
                packing: PackingContext<f64, L, W>,
                extension: E,
                policy: &dyn PatternPolicy,
                config: Bpp1dSolveConfig,
                handle: &SolveHandle,
                solve: F,
            ) -> Result<Bpp1dSolution<L>>
            where
                L: UnitTrait + Clone,
                W: UnitTrait + Clone,
                E: PatternModelingExtension<f64, L, W> + Clone,
                F: FnMut(&LinearTriadModel, Duration, &SolveHandle) -> Result<SolveReport<f64>>,
            {
                run(packing, extension, policy, config, handle, solve, RunScope { cg: $cg, default_rules: false })
            }
        }
    };
}

entry!(Bpp1dMilp, false);
entry!(Bpp1dColumnGeneration, true);

struct RunScope { cg: bool, default_rules: bool }

fn terminal<L: UnitTrait>(
    reason: TerminationReason,
    complete: bool,
    count: usize,
    iterations: usize,
) -> Result<Bpp1dSolution<L>> {
    Ok(Bpp1dSolution { report: SolveReport::builder(ProblemStatus::Unknown, reason).build()?, packing: None,
        candidate_complete: complete, column_count: count, pricing_iterations: iterations, pricing_converged: false,
        globally_optimal: false, globally_infeasible: false, indicators: BTreeMap::new() })
}

fn check_report(report: &SolveReport<f64>, model: &LinearTriadModel) -> Result<()> {
    report.validate()?;
    if report.model_mapping.as_ref() != Some(&linear_model_mapping(model)?)
        || report.fingerprints.model.as_ref() != Some(&linear_model_fingerprint(model)?)
    { return Err(invalid("missing or mismatched model mapping/fingerprint")); }
    Ok(())
}

fn run<L, W, E, F>(
    packing: PackingContext<f64, L, W>,
    extension: E,
    policy: &dyn PatternPolicy,
    config: Bpp1dSolveConfig,
    handle: &SolveHandle,
    mut solve: F,
    scope: RunScope,
) -> Result<Bpp1dSolution<L>>
where
    L: UnitTrait + Clone,
    W: UnitTrait + Clone,
    E: PatternModelingExtension<f64, L, W> + Clone,
    F: FnMut(&LinearTriadModel, Duration, &SolveHandle) -> Result<SolveReport<f64>>,
{
    let started = Instant::now();
    if !config.tolerance.is_finite() || !(0.0..0.5).contains(&config.tolerance)
        || (scope.cg && (config.iteration_limit == 0 || config.columns_per_iteration == 0))
    { return Err(invalid("invalid tolerance or iteration budget")); }
    if handle.is_cancelled() { return terminal(TerminationReason::Cancelled, false, 0, 0); }
    if config.time_limit.is_zero() { return terminal(TerminationReason::TimeLimit, false, 0, 0); }
    let generated = packing.generate_patterns(config.generation, policy)?;
    let mut pool = generated.patterns.into_iter().collect::<BTreeSet<_>>();
    let mut active = BTreeSet::new();
    if scope.cg {
        for best_fit in [false, true] { active.extend(packing.initial_patterns(best_fit, config.expansion_limit, policy)?); }
        pool.extend(active.iter().cloned());
    }
    let mut iterations = 0;
    let mut converged = false;
    if scope.cg {
        let lp = PatternAssignmentContext::new(packing.clone(), pool.iter().cloned(), PackingModelStage::Lp, extension.clone())?;
        for _ in 0..config.iteration_limit {
            if handle.is_cancelled() { return terminal(TerminationReason::Cancelled, generated.complete, pool.len(), iterations); }
            let remaining = config.time_limit.saturating_sub(started.elapsed());
            if remaining.is_zero() { return terminal(TerminationReason::TimeLimit, generated.complete, pool.len(), iterations); }
            let compilation = lp.screened_lp(&active)?;
            let report = solve(&compilation.model, remaining, handle)?;
            check_report(&report, &compilation.model)?;
            if handle.is_cancelled() || report.termination_reason == TerminationReason::Cancelled {
                return terminal(TerminationReason::Cancelled, generated.complete, pool.len(), iterations);
            }
            if report.termination_reason == TerminationReason::Interrupted {
                return terminal(TerminationReason::Interrupted, generated.complete, pool.len(), iterations);
            }
            if report.problem_status == ProblemStatus::Infeasible {
                require_infeasibility_certificate_for_model(&report, &linear_model_fingerprint(&compilation.model)?)?;
                // 库存耦合可能令 seeds 不可行；转完整 final，不标记定价收敛。 / Stock coupling may invalidate seeds; use full final without claiming convergence.
                break;
            }
            if !report.is_optimal() { break; }
            let certificate = require_optimal_lp_certificate_for_linear_model(&report, &compilation.model)?;
            iterations += 1;
            let mut costs = compilation.model.c.clone();
            for (row, dual) in compilation.model.basic.A.rows.iter().zip(certificate.dual) {
                for (index, coefficient) in &row.entries { costs[*index] -= coefficient * dual; }
            }
            if costs.iter().any(|cost| !cost.is_finite()) { return Err(invalid("nonfinite reduced cost")); }
            let indices = compilation.model.basic.variables.iter().enumerate().map(|(index, token)| (token.id(), index)).collect::<BTreeMap<_, _>>();
            let mut candidates = lp.aggregation().variables.iter().filter(|(pattern, _)| !active.contains(*pattern))
                .map(|(pattern, variable)| (pattern, costs[indices[variable]]))
                .filter(|(_, cost)| *cost < -config.tolerance).collect::<Vec<_>>();
            candidates.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(b.0)));
            if candidates.is_empty() { converged = true; break; }
            active.extend(candidates.into_iter().take(config.columns_per_iteration).map(|(pattern, _)| pattern.clone()));
        }
    }
    if handle.is_cancelled() { return terminal(TerminationReason::Cancelled, generated.complete, pool.len(), iterations); }
    let final_master = PatternAssignmentContext::new(packing.clone(), pool.iter().cloned(),
        if scope.cg { PackingModelStage::FinalMilp } else { PackingModelStage::Milp }, extension)?;
    let compilation = final_master.model().linear_model_snapshot()?.compile_flt64()?;
    let remaining = config.time_limit.saturating_sub(started.elapsed());
    if remaining.is_zero() { return terminal(TerminationReason::TimeLimit, generated.complete, pool.len(), iterations); }
    let report = solve(&compilation.model, remaining, handle)?;
    check_report(&report, &compilation.model)?;
    let mut indicators = BTreeMap::new();
    let physical = if let Some(solution) = &report.solution {
        let usages = final_master.extract_solution(&solution.values, &config.tolerance)?;
        let objective = LinearObjectiveEvaluationFlt64::new(compilation.model.c.iter().copied().zip(solution.values.iter().copied()), compilation.objective_constant)?;
        objective.validate_reported(solution.objective_value.or(solution.objective), config.tolerance.max(1e-7))?;
        indicators = usages.indicators;
        Some(packing.pack(&usages.usages, config.expansion_limit)?)
    } else { None };
    let globally_optimal = physical.is_some() && scope.default_rules && generated.complete && report.is_optimal();
    let globally_infeasible = scope.default_rules && generated.complete && report.problem_status == ProblemStatus::Infeasible
        && require_infeasibility_certificate_for_model(&report, &linear_model_fingerprint(&compilation.model)?).is_ok();
    Ok(Bpp1dSolution { report, packing: physical, candidate_complete: generated.complete, column_count: pool.len(),
        pricing_iterations: iterations, pricing_converged: converged, globally_optimal, globally_infeasible, indicators })
}
