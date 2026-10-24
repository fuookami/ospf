use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use ospf_rust_core::error::{CoreError, Result};
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::model::LinearObjectiveEvaluationFlt64;
use ospf_rust_core::solver::audit::linear_model_mapping;
use ospf_rust_core::solver::fingerprint::linear_model_fingerprint;
use ospf_rust_core::solver::{ProblemStatus, SolveFingerprints, SolveHandle, SolveReport, SolveSolution, TerminationReason};
use ospf_rust_core::variable::VariableId;
use ospf_rust_framework::solver::column_projection::{ColumnMasterRegistration, ColumnPricingRequest};
use ospf_rust_quantities::unit::UnitTrait;
use crate::{DiscreteLoadingSolution, PartitionLoadingContext, PartitionLoadingOptions};
use super::{RegisteredDiscreteLoading, invalid};

/// 根节点列生成及独立资源补全预算。 / Root CG and separate resource-completion budgets.
#[derive(Debug, Clone, Copy)]
pub struct PartitionColumnGenerationConfig {
    /// 包含 Phase I 的根节点 LP 轮数。 / Root LP rounds including Phase I.
    pub iteration_limit: usize,
    /// 根节点总列数上限。 / Total root-pool column limit.
    pub column_limit: usize,
    /// 补全阶段最多另加列数，必须能容纳每分区一列。 / Additional completion-column cap, sufficient for one column per partition.
    pub completion_column_limit: usize,
    /// 全过程总时限。 / Total deadline across the entire execution.
    pub time_limit: Duration,
    /// 定价与人工变量容差，必须为正且小于 0.5。 / Pricing/artificial tolerance, positive and below 0.5.
    pub tolerance: f64,
}

impl Default for PartitionColumnGenerationConfig {
    fn default() -> Self {
        Self { iteration_limit: 64, column_limit: 10_000, completion_column_limit: 64, time_limit: Duration::from_secs(60), tolerance: 1e-7 }
    }
}

/// 每轮采用同一对偶快照的定价轨迹。 / Pricing trace for one shared dual snapshot.
#[derive(Debug, Clone)]
pub struct PartitionColumnGenerationIteration {
    /// 是否 Phase I。 / Whether Phase I.
    pub phase_one: bool,
    /// 本轮 LP 目标，含独立常数。 / This LP's objective, including the separate constant.
    pub objective: f64,
    /// 实际新增列数。 / Actual added columns.
    pub added_columns: usize,
    /// 本轮所有局部 MILP 是否有可靠最优证明。 / Whether all local MILPs have reliable optimality proofs.
    pub pricing_proven: bool,
}

/// 根 LP 与受限整数池结果，不冒充 branch-and-price 或原问题不可行证明。
/// Root LP and restricted integer-pool result, never branch-and-price or a proof of original infeasibility.
pub struct PartitionColumnGenerationResult {
    /// 已有完整源方案之后的可恢复加速后端错误。 / Recoverable accelerator backend error after obtaining a complete-source plan.
    pub acceleration_error: Option<CoreError>,
    /// 源模型 Feasible 或执行终态；受限模型证明另行保留，不被提升。
    /// Source-model Feasible or execution terminal; restricted-model proofs stay separate and are never promoted.
    pub report: SolveReport<f64>,
    /// 实际资源补全模型报告，不转为源模型证明。 / Actual refinement-model report, never promoted to a source-model proof.
    pub refinement_report: Option<SolveReport<f64>>,
    /// 实际最终受限模型报告，不转为源模型证明。 / Actual final restricted-model report, never promoted to a source-model proof.
    pub final_report: Option<SolveReport<f64>>,
    /// 独立验证后的源模型装载解。 / Independently validated source-model loading solution.
    pub solution: Option<DiscreteLoadingSolution>,
    /// 按源变量身份的完整向量。 / Complete vector keyed by source-variable identity.
    pub values: BTreeMap<VariableId, f64>,
    /// 完整源模型最小化目标，含常数一次。 / Minimizing full-source objective, including its constant once.
    pub objective: Option<f64>,
    /// 根节点定价是否以完整可靠局部证明收敛。 / Whether root pricing converged with complete reliable local proofs.
    pub lp_converged: bool,
    /// 根节点实际列数。 / Actual root column count.
    pub root_column_count: usize,
    /// 资源补全实际另加列数。 / Actual additional resource-completion columns.
    pub completion_column_count: usize,
    /// 根节点轨迹。 / Root trace.
    pub trace: Vec<PartitionColumnGenerationIteration>,
}

struct Priced {
    values: BTreeMap<String, f64>,
    cost: f64,
    optimal: bool,
}

enum PricingOutcome {
    Priced(Priced),
    Stopped(TerminationReason),
}

fn check_report(report: &SolveReport<f64>, model: &LinearTriadModel) -> Result<()> {
    report.validate()?;
    if report.model_mapping.as_ref() != Some(&linear_model_mapping(model)?)
        || report.fingerprints.model.as_ref() != Some(&linear_model_fingerprint(model)?)
    { return Err(invalid("partition report does not match the actual compilation")); }
    Ok(())
}

fn check_objective(report: &SolveReport<f64>, model: &LinearTriadModel, tolerance: f64) -> Result<()> {
    let solution = report.solution.as_ref().ok_or_else(|| invalid("restricted partition model has no incumbent; original infeasibility is not proven"))?;
    if solution.values.len() != model.num_variables() { return Err(invalid("incomplete partition objective vector")); }
    let objective = LinearObjectiveEvaluationFlt64::new(model.c.iter().copied().zip(solution.values.iter().copied()), 0.0)?;
    objective.validate_reported(solution.objective_value.or(solution.objective), tolerance.max(1e-7))?;
    Ok(())
}

struct Budget<'a> {
    started: Instant,
    limit: Duration,
    handle: &'a SolveHandle,
}

impl Budget<'_> {
    fn remaining(&self) -> Duration { self.limit.saturating_sub(self.started.elapsed()) }
    fn terminal(&self) -> Option<TerminationReason> {
        if self.handle.is_cancelled() { Some(TerminationReason::Cancelled) }
        else if self.remaining().is_zero() { Some(TerminationReason::TimeLimit) } else { None }
    }
}

fn price<U, F>(
    context: &PartitionLoadingContext<f64, U>,
    source: &ospf_rust_core::model::MetaModel,
    master: Option<&ColumnMasterRegistration<f64>>,
    request: &ColumnPricingRequest<f64>,
    tolerance: f64,
    budget: &Budget<'_>,
    solve: &mut F,
) -> Result<PricingOutcome>
where
    U: UnitTrait + Clone,
    F: FnMut(&LinearTriadModel, Duration, &SolveHandle) -> Result<SolveReport<f64>>,
{
    if let Some(reason) = budget.terminal() { return Ok(PricingOutcome::Stopped(reason)); }
    let model = context.pricing_model(source, master, request)?;
    let snapshot = model.linear_model_snapshot()?;
    let compilation = snapshot.compile_flt64()?;
    let report = solve(&compilation.model, budget.remaining(), budget.handle)?;
    if let Some(reason) = budget.terminal() { return Ok(PricingOutcome::Stopped(reason)); }
    check_report(&report, &compilation.model)?;
    if matches!(report.termination_reason, TerminationReason::Cancelled | TerminationReason::Interrupted | TerminationReason::TimeLimit) {
        return Ok(PricingOutcome::Stopped(report.termination_reason));
    }
    if report.solution.is_none() {
        return Err(CoreError::NotImplemented("local pricing has no configuration incumbent; only a complete-source solve can resolve the source problem".into()));
    }
    check_objective(&report, &compilation.model, tolerance)?;
    let values = &report.solution.as_ref().ok_or_else(|| invalid("missing local incumbent"))?.values;
    if values.len() != snapshot.variables.len() { return Err(invalid("incomplete local pricing vector")); }
    let by_variable = snapshot.variables.iter().zip(values).map(|(state, value)| (state.variable, *value)).collect::<BTreeMap<_, _>>();
    snapshot.validate_solution(&by_variable, &tolerance)?;
    let values = context.decisions().iter().filter(|decision| decision.block_id == request.block_id())
        .map(|decision| by_variable.get(&decision.variable).copied().map(|value| (decision.id.clone(), value)).ok_or_else(|| invalid("missing local pricing decision")))
        .collect::<Result<BTreeMap<_, _>>>()?;
    let cost = request.reduced_cost(&values)?;
    let full_objective = snapshot.evaluate_objective_flt64(&by_variable)?.total_value()?;
    if (cost - full_objective).abs() > tolerance.max(1e-7) * cost.abs().max(1.0) {
        return Err(invalid("local reduced cost differs from complete pricing objective"));
    }
    Ok(PricingOutcome::Priced(Priced { values, cost, optimal: report.is_optimal() }))
}

fn stopped(reason: TerminationReason, root: usize, trace: Vec<PartitionColumnGenerationIteration>) -> Result<PartitionColumnGenerationResult> {
    Ok(PartitionColumnGenerationResult { acceleration_error: None, report: SolveReport::builder(ProblemStatus::Unknown, reason).build()?, refinement_report: None, final_report: None, solution: None,
        values: BTreeMap::new(), objective: None, lp_converged: false, root_column_count: root, completion_column_count: 0, trace })
}

struct Candidate {
    values: BTreeMap<VariableId, f64>,
    solution: Option<DiscreteLoadingSolution>,
    objective: Option<f64>,
}

fn read_candidate<U: UnitTrait + Clone>(
    registration: &RegisteredDiscreteLoading<f64, U>,
    context: &PartitionLoadingContext<f64, U>,
    master: &ColumnMasterRegistration<f64>,
    report: &SolveReport<f64>,
    tolerance: f64,
) -> Result<Candidate> {
    let mut candidate = Candidate { values: BTreeMap::new(), solution: None, objective: None };
    if let Some(incumbent) = &report.solution {
        let compilation = master.snapshot().compile_flt64()?;
        check_objective(report, &compilation.model, tolerance)?;
        candidate.values = context.extract_solution(registration.model(), master, &incumbent.values)?;
        let snapshot = registration.model().linear_model_snapshot()?;
        let ordered = snapshot.variables.iter().map(|state| candidate.values[&state.variable]).collect::<Vec<_>>();
        candidate.solution = Some(registration.context().extract_solution(registration.model(), &ordered, &tolerance)?);
        let full = snapshot.evaluate_objective_flt64(&candidate.values)?;
        let restricted = LinearObjectiveEvaluationFlt64::new(compilation.model.c.iter().copied().zip(incumbent.values.iter().copied()), compilation.objective_constant)?;
        full.validate_total(&restricted, tolerance.max(1e-7))?;
        candidate.objective = Some(full.total_value()?);
    }
    Ok(candidate)
}

fn source_report<U: UnitTrait>(
    registration: &RegisteredDiscreteLoading<f64, U>,
    candidate: &Candidate,
    reason: TerminationReason,
) -> Result<SolveReport<f64>> {
    let snapshot = registration.model().linear_model_snapshot()?;
    let compilation = snapshot.compile_flt64()?;
    let mut builder = SolveReport::builder(if candidate.solution.is_some() { ProblemStatus::Feasible } else { ProblemStatus::Unknown }, reason)
        .model_mapping(Some(linear_model_mapping(&compilation.model)?))
        .fingerprints(SolveFingerprints { model: Some(linear_model_fingerprint(&compilation.model)?), ..Default::default() });
    if candidate.solution.is_some() {
        let mut solution = SolveSolution::vector(snapshot.variables.iter().map(|state| candidate.values[&state.variable]).collect());
        solution.objective_value = Some(snapshot.evaluate_objective_flt64(&candidate.values)?.variable_value()?);
        builder = builder.solution(solution);
    }
    builder.build()
}

/// 独立分区列生成，不与 Benders 混合；目前只接受同域 f64 完整注册。
/// Independent partition CG, never mixed with Benders; currently accepts same-domain f64 complete registration only.
pub struct PartitionColumnGenerationApplicationService;

impl PartitionColumnGenerationApplicationService {
    /// 从同一完整源模型定价、整轮批量提交、可选资源补全并求无人工 final。
    /// Price from one complete source model, commit whole-round batches, optionally complete resources, then solve an artificial-free final.
    pub fn solve<U, F>(
        registration: &RegisteredDiscreteLoading<f64, U>,
        options: PartitionLoadingOptions<f64>,
        config: PartitionColumnGenerationConfig,
        handle: &SolveHandle,
        mut solve: F,
    ) -> Result<PartitionColumnGenerationResult>
    where
        U: UnitTrait + Clone,
        F: FnMut(&LinearTriadModel, Duration, &SolveHandle) -> Result<SolveReport<f64>>,
    {
        let budget = Budget { started: Instant::now(), limit: config.time_limit, handle };
        let resources = !options.refinable_resources.is_empty();
        if config.iteration_limit == 0 || config.column_limit < options.partitions.len()
            || (resources && config.completion_column_limit < options.partitions.len())
            || !config.tolerance.is_finite() || !(0.0..0.5).contains(&config.tolerance) || config.tolerance == 0.0
            || options.tolerance != config.tolerance
        { return Err(invalid("invalid partition budgets or inconsistent tolerances")); }
        let source = registration.model();
        let mut context = PartitionLoadingContext::new(registration.context(), source, options)?;
        let blocks = context.partitions().iter().map(|partition| partition.id.clone()).collect::<Vec<_>>();
        let mut trace = Vec::new();
        let mut seeds = Vec::new();
        for block in &blocks {
            if let Some(reason) = budget.terminal() { return stopped(reason, 0, trace); }
            let request = context.pricing_request(source, None, block, true)?;
            let priced = match price(&context, source, None, &request, config.tolerance, &budget, &mut solve)? {
                PricingOutcome::Priced(priced) => priced,
                PricingOutcome::Stopped(reason) => return stopped(reason, 0, trace),
            };
            seeds.push((block.clone(), priced.values));
        }
        context.add_columns(source, seeds)?;
        let mut phase_one = true;
        let mut converged = false;
        for _ in 0..config.iteration_limit {
            if let Some(reason) = budget.terminal() { return stopped(reason, context.columns().len(), trace); }
            let master = context.master(source, phase_one, false, false)?;
            let mut terminal = None;
            let lp = context.solve_master_lp(source, &master, |model| {
                let report = solve(model, budget.remaining(), handle)?;
                check_report(&report, model)?;
                if matches!(report.termination_reason, TerminationReason::Cancelled | TerminationReason::Interrupted | TerminationReason::TimeLimit) {
                    terminal = Some(report.termination_reason);
                } else if !report.is_optimal() || report.solution.as_ref().is_none_or(|solution| solution.dual_solution.is_none()) {
                    return Err(CoreError::NotImplemented("partition pricing requires an optimal LP with dual evidence; finish on the complete source model".into()));
                }
                Ok(report)
            });
            if let Some(reason) = terminal { return stopped(reason, context.columns().len(), trace); }
            let lp = lp?;
            if let Some(reason) = budget.terminal() { return stopped(reason, context.columns().len(), trace); }
            let solution = lp.solution.as_ref().ok_or_else(|| invalid("optimal master LP lacks incumbent"))?;
            let values = master.snapshot().variables.iter().zip(&solution.values).map(|(state, value)| (state.variable, *value)).collect();
            let objective = master.snapshot().evaluate_objective_flt64(&values)?.total_value()?;
            if phase_one && objective <= config.tolerance {
                trace.push(PartitionColumnGenerationIteration { phase_one, objective, added_columns: 0, pricing_proven: false });
                phase_one = false;
                continue;
            }
            let mut batches = Vec::new();
            let mut proven = true;
            let mut improving = false;
            let before = context.columns().len();
            let mut available = config.column_limit.saturating_sub(before);
            for block in &blocks {
                if let Some(reason) = budget.terminal() { return stopped(reason, before, trace); }
                let request = context.pricing_request(source, Some(&master), block, phase_one)?;
                let priced = match price(&context, source, Some(&master), &request, config.tolerance, &budget, &mut solve)? {
                    PricingOutcome::Priced(priced) => priced,
                    PricingOutcome::Stopped(reason) => return stopped(reason, before, trace),
                };
                proven &= priced.optimal;
                let mut columns = Vec::new();
                if priced.cost < -config.tolerance {
                    improving = true;
                    if available > 0 && !context.contains_configuration(source, block, &priced.values)? {
                        columns.push(priced.values);
                        available -= 1;
                    }
                }
                batches.push((request, columns));
            }
            context.add_priced_batch(source, &master, batches)?;
            let added_columns = context.columns().len() - before;
            trace.push(PartitionColumnGenerationIteration { phase_one, objective, added_columns, pricing_proven: proven });
            if added_columns == 0 { converged = !phase_one && proven && !improving; break; }
            if context.columns().len() == config.column_limit { break; }
        }
        let root = context.columns().len();
        let mut best = Candidate { values: BTreeMap::new(), solution: None, objective: None };
        let mut refinement_report = None;
        let mut final_report = None;
        let mut acceleration_error = None;
        let mut terminal = None;
        if resources {
            if let Some(reason) = budget.terminal() { return stopped(reason, root, trace); }
            let refinement = context.master(source, false, true, true)?;
            let compilation = refinement.snapshot().compile_flt64()?;
            let report = solve(&compilation.model, budget.remaining(), handle)?;
            check_report(&report, &compilation.model)?;
            best = read_candidate(registration, &context, &refinement, &report, config.tolerance)?;
            terminal = budget.terminal().or_else(|| matches!(report.termination_reason,
                TerminationReason::TimeLimit | TerminationReason::Cancelled | TerminationReason::Interrupted).then_some(report.termination_reason));
            if terminal.is_none() && best.solution.is_some() {
                context.materialize_completed_resources(source, &refinement,
                    &report.solution.as_ref().ok_or_else(|| invalid("resource completion has no incumbent"))?.values)?;
                if context.columns().len() - root > config.completion_column_limit { return Err(invalid("resource completion exceeded its separate column cap")); }
            } else if terminal.is_none() { terminal = Some(report.termination_reason); }
            refinement_report = Some(report);
        }
        terminal = terminal.or_else(|| budget.terminal());
        if terminal.is_none() {
            let master = context.master(source, false, true, false)?;
            let compilation = master.snapshot().compile_flt64()?;
            match solve(&compilation.model, budget.remaining(), handle) {
                Ok(report) => {
                    check_report(&report, &compilation.model)?;
                    let candidate = read_candidate(registration, &context, &master, &report, config.tolerance)?;
                    if candidate.objective.is_some_and(|value| best.objective.is_none_or(|previous| value < previous)) { best = candidate; }
                    terminal = Some(budget.terminal().unwrap_or(report.termination_reason));
                    final_report = Some(report);
                }
                Err(error) if best.solution.is_some() && super::benders::recoverable(&error) => {
                    terminal = Some(budget.terminal().unwrap_or(if error.solver_error_class() == ospf_rust_core::error::SolverErrorClass::Numerical {
                        TerminationReason::NumericalFailure } else { TerminationReason::BackendFailure }));
                    acceleration_error = Some(error);
                }
                Err(error) => return Err(error),
            }
        }
        let report = source_report(registration, &best, terminal.unwrap_or(TerminationReason::Unknown))?;
        Ok(PartitionColumnGenerationResult { acceleration_error, report, refinement_report, final_report, solution: best.solution, values: best.values,
            objective: best.objective, lp_converged: converged, root_column_count: root,
            completion_column_count: context.columns().len() - root, trace })
    }
}
