//! 完整 MetaModel 的固定连续追索分解。 / Fixed continuous-recourse decomposition of a complete MetaModel.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use ospf_rust_core::error::{CoreError, ModelError, Result, SolverErrorClass};
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::model::{LinearModelSnapshot, LinearVariableState, MetaModel, SubObjective};
use ospf_rust_core::solver::{ProblemStatus, SolveHandle, SolveReport, TerminationReason, linear_model_fingerprint,
    linear_model_mapping, require_infeasibility_certificate_for_model, require_optimal_lp_certificate_for_linear_model,
    validate_linear_farkas_ray};
use ospf_rust_core::token::TokenVariableData;
use ospf_rust_core::variable::{VariableId, VariableType, new_standalone_id};

fn invalid(message: &str) -> CoreError { ModelError::InvalidConstraint(format!("fixed recourse: {message}")).into() }
fn finite(value: f64) -> Result<f64> { if value.is_finite() { Ok(value) } else { Err(invalid("nonfinite arithmetic")) } }

fn exact(value: f64) -> Result<BigRational> {
    BigRational::from_float(value).ok_or_else(|| invalid("nonfinite exact operand"))
}

fn unsupported() -> CoreError {
    CoreError::NotImplemented("fixed recourse arithmetic is not safely representable as finite f64; use a fresh full MILP".into())
}

fn rounded(value: &BigRational, upward: bool) -> Result<f64> {
    let mut result = value.to_f64().filter(|value| value.is_finite()).ok_or_else(unsupported)?;
    if upward && exact(result)? < *value { result = result.next_up(); }
    if !upward && exact(result)? > *value { result = result.next_down(); }
    if result.is_finite() { Ok(result) } else { Err(unsupported()) }
}

/// 完整展开变量的显式阶段；整数追索一律拒绝。 / Explicit stages for expanded variables; integer recourse is always rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinearBendersStage {
    /// 保留在 master。 / Retain in the master.
    Master,
    /// 固定矩阵的连续追索。 / Continuous recourse with a fixed matrix.
    ContinuousRecourse,
}

/// 不声明时，全部整数及非零目标变量留在 master。 / By default, all integers and nonzero objective variables stay in the master.
#[derive(Debug, Clone, Default)]
pub struct LinearBendersPartitionOptions {
    /// 按完整展开源变量身份声明阶段。 / Stages keyed by fully expanded source-variable identity.
    pub stages: BTreeMap<VariableId, LinearBendersStage>,
}

/// 分解预算和证书恢复策略；全部调用共用总时限。 / Decomposition budgets and certificate recovery; all calls share one deadline.
#[derive(Debug, Clone, Copy)]
pub struct FixedRecourseConfig {
    /// master／sub 轮数上限。 / Master/subproblem round limit.
    pub iteration_limit: usize,
    /// 含显式对偶与 Farkas 恢复的总时限。 / Total time including explicit dual/Farkas recovery.
    pub time_limit: Duration,
    /// 同域解与切割验证容差。 / Same-domain solution and cut tolerance.
    pub tolerance: f64,
    /// 缺少数值证据时求显式证书模型；已存在但无效的证据不被替换。
    /// Solve explicit certificate models when numeric evidence is missing; existing invalid evidence is not replaced.
    pub recover_missing_certificates: bool,
}

impl Default for FixedRecourseConfig {
    fn default() -> Self { Self { iteration_limit: 128, time_limit: Duration::from_secs(60), tolerance: 1e-7, recover_missing_certificates: true } }
}

/// 执行停止原因，不把证书缺失／无效混为数学不可行。 / Execution stop, separating missing/invalid evidence from mathematical infeasibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedRecourseStop {
    /// 完整源解与可靠下界收敛。 / Complete source incumbent and reliable lower bound converged.
    Converged,
    /// master 的真实终态。 / Actual master terminal.
    MasterTermination(ProblemStatus, TerminationReason),
    /// 子问题或证书恢复的真实终态。 / Actual subproblem or certificate-recovery terminal.
    SubproblemTermination(ProblemStatus, TerminationReason),
    /// 缺少所需证书。 / Required certificate is missing.
    CertificateMissing,
    /// 证据存在但独立复核失败。 / Evidence exists but fails independent verification.
    CertificateInvalid,
    /// 无法在当前 f64 边界安全表达有效切割。 / A valid cut cannot be safely represented at the current f64 boundary.
    Unsupported,
    /// 无可分离的新切割。 / No separating new cut.
    CutStall,
    /// 达到迭代上限。 / Iteration limit reached.
    IterationLimit,
    /// 取消或外部中断。 / Cancellation or external interruption.
    Cancelled,
    /// 总时间预算耗尽。 / Total deadline exhausted.
    TimeLimit,
    /// 已验证方案之后的可恢复后端失败。 / Recoverable backend failure after a validated incumbent.
    BackendFailure(TerminationReason),
}

impl FixedRecourseStop {
    /// 是否允许调用方在剩余预算内使用全新工厂回退。 / Whether a caller may fall back through a fresh factory within remaining time.
    pub fn allows_fallback(self) -> bool {
        !matches!(self, Self::Converged | Self::Cancelled | Self::TimeLimit)
            && !matches!(self, Self::MasterTermination(_, TerminationReason::Cancelled | TerminationReason::Interrupted | TerminationReason::TimeLimit)
                | Self::SubproblemTermination(_, TerminationReason::Cancelled | TerminationReason::Interrupted | TerminationReason::TimeLimit))
    }
}

/// 完整原域方案和实际分解报告，不把 master 报告重新标记为源模型证明。
/// Complete native-domain incumbent and actual decomposition reports; master reports are never relabeled as source-model proofs.
pub struct FixedRecourseResult {
    /// 保留既有完整源方案时记录的原始后端错误。 / Original backend error retained alongside an existing complete-source plan.
    pub backend_error: Option<CoreError>,
    /// 明确停止原因。 / Explicit stop reason.
    pub stop: FixedRecourseStop,
    /// 按源变量身份的完整已验证向量。 / Complete validated vector keyed by source-variable identity.
    pub values: BTreeMap<VariableId, f64>,
    /// 含常数一次的最小化源目标。 / Minimizing source objective including its constant once.
    pub objective: Option<f64>,
    /// 由最优 master 证明支持的源目标下界。 / Source-objective lower bound supported by an optimal master proof.
    pub lower_bound: Option<f64>,
    /// 最后实际 master 报告。 / Last actual master report.
    pub master_report: Option<SolveReport<f64>>,
    /// 最后实际追索报告。 / Last actual recourse report.
    pub subproblem_report: Option<SolveReport<f64>>,
    /// 实际完成轮数。 / Executed rounds.
    pub iterations: usize,
    /// 已验证并提交的可行性割数。 / Verified and committed feasibility cuts.
    pub feasibility_cuts: usize,
    /// 已验证并提交的最优性割数。 / Verified and committed optimality cuts.
    pub optimality_cuts: usize,
}

impl FixedRecourseResult {
    fn retain_backend_error(&mut self, error: CoreError, handle: &SolveHandle) -> Result<()> {
        if self.objective.is_none() || !matches!(error.solver_error_class(), SolverErrorClass::Unsupported | SolverErrorClass::Environment
            | SolverErrorClass::License | SolverErrorClass::Backend | SolverErrorClass::Numerical) { return Err(error); }
        self.stop = if handle.is_cancelled() { FixedRecourseStop::Cancelled }
            else { FixedRecourseStop::BackendFailure(if error.solver_error_class() == SolverErrorClass::Numerical {
                TerminationReason::NumericalFailure } else { TerminationReason::BackendFailure }) };
        self.backend_error = Some(error);
        Ok(())
    }
}

struct RecourseRow {
    master: BTreeMap<VariableId, f64>,
    recourse: BTreeMap<VariableId, f64>,
    rhs: f64,
    name: String,
}

/// 不可变固定矩阵划分，所有行来自完整展开的同域 f64 源模型。
/// Immutable fixed-matrix partition; every row comes from the fully expanded same-domain f64 source model.
pub struct LinearBendersPartition {
    snapshot: LinearModelSnapshot<f64>,
    master: BTreeSet<VariableId>,
    recourse: BTreeSet<VariableId>,
    rows: Vec<RecourseRow>,
    theta: Option<VariableId>,
    theta_lower: f64,
}

fn register(model: &mut MetaModel, state: &LinearVariableState<f64>) -> Result<usize> {
    model.register_variable_data(TokenVariableData { id: state.variable, index: 0, name: state.name.clone(), display_name: None,
        var_type: state.var_type, lower_bound: state.lower_bound, upper_bound: state.upper_bound })
}

fn add_row(model: &mut MetaModel, row: &BTreeMap<VariableId, f64>, rhs: f64, name: &str) -> Result<()> {
    let indices = model.tokens().iter().enumerate().map(|(index, token)| (token.id(), index)).collect::<BTreeMap<_, _>>();
    let terms = row.iter().map(|(variable, coefficient)| indices.get(variable).map(|index| (*index, *coefficient))
        .ok_or_else(|| invalid("cut or row contains a nonlocal variable"))).collect::<Result<Vec<_>>>()?;
    model.add_le_constraint(&terms, finite(rhs)?, name)
}

fn objective(model: &mut MetaModel, coefficients: &BTreeMap<VariableId, f64>, constant: f64) {
    let terms = model.tokens().iter().enumerate().filter_map(|(index, token)| coefficients.get(&token.id())
        .map(|coefficient| LinearMonomial::new(*coefficient, index))).collect();
    model.add_sub_objective(SubObjective::minimize(Linear::new(terms, constant), "complete_objective"));
}

impl LinearBendersPartition {
    /// 完整展开后划分；显式追索目标必须有从真实变量边界推导的有限下界。
    /// Partition after complete expansion; explicit objective recourse needs a finite lower bound derived from actual variable bounds.
    pub fn build(source: &MetaModel, options: LinearBendersPartitionOptions) -> Result<Self> {
        let snapshot = source.linear_model_snapshot()?;
        let compiled = snapshot.compile_flt64()?;
        let variables = snapshot.variables.iter().map(|state| state.variable).collect::<BTreeSet<_>>();
        if options.stages.keys().any(|variable| !variables.contains(variable)) { return Err(invalid("unknown staged variable")); }
        let mut master = BTreeSet::new();
        let mut recourse = BTreeSet::new();
        let mut theta_lower = BigRational::zero();
        let mut has_recourse_objective = false;
        for state in &snapshot.variables {
            let stage = options.stages.get(&state.variable);
            if state.var_type.is_integer() && stage == Some(&LinearBendersStage::ContinuousRecourse) {
                return Err(invalid("all expanded integer variables must remain in master"));
            }
            let coefficient = snapshot.objective.get(&state.variable).copied().unwrap_or(0.0);
            if state.var_type.is_integer() || stage == Some(&LinearBendersStage::Master)
                || (coefficient != 0.0 && stage != Some(&LinearBendersStage::ContinuousRecourse))
            { master.insert(state.variable); }
            else {
                recourse.insert(state.variable);
                if coefficient != 0.0 {
                    let bounds = state.relaxed()?;
                    let bound = if coefficient > 0.0 { bounds.lower_bound } else { bounds.upper_bound }
                        .ok_or_else(|| CoreError::NotImplemented("fixed recourse requires a finite box-derived objective lower bound; use a fresh full MILP when the bound is only implied by constraints".into()))?;
                    theta_lower += exact(coefficient)? * exact(bound)?;
                    has_recourse_objective = true;
                }
            }
        }
        let mut rows = Vec::new();
        for (index, row) in compiled.model.A.rows.iter().enumerate() {
            let mut entry = RecourseRow { master: BTreeMap::new(), recourse: BTreeMap::new(), rhs: compiled.model.b[index], name: format!("source_{index}") };
            for (column, coefficient) in &row.entries {
                let variable = snapshot.variables[*column].variable;
                if master.contains(&variable) { entry.master.insert(variable, *coefficient); }
                else { entry.recourse.insert(variable, *coefficient); }
            }
            rows.push(entry);
        }
        // 将全部追索边界显式化，子问题变量自由；割乘子因此包含完整边界贡献。
        // Make every recourse bound explicit and subproblem variables free, so cut multipliers include all bound contributions.
        for state in snapshot.variables.iter().filter(|state| recourse.contains(&state.variable)) {
            let bounds = state.relaxed()?;
            for (bound, sign, suffix) in [(bounds.lower_bound, -1.0, "lb"), (bounds.upper_bound, 1.0, "ub")] {
                if let Some(bound) = bound {
                    rows.push(RecourseRow { master: BTreeMap::new(), recourse: BTreeMap::from([(state.variable, sign)]),
                        rhs: finite(sign * bound)?, name: format!("bound_{suffix}_{:?}", state.variable) });
                }
            }
        }
        Ok(Self { snapshot, master, recourse, rows, theta: has_recourse_objective.then(new_standalone_id), theta_lower: rounded(&theta_lower, false)? })
    }

    /// 完整展开的源模型快照。 / Fully expanded source-model snapshot.
    pub fn snapshot(&self) -> &LinearModelSnapshot<f64> { &self.snapshot }

    /// master 源变量身份，含所有辅助整数。 / Master source-variable identities including every auxiliary integer.
    pub fn master_variables(&self) -> &BTreeSet<VariableId> { &self.master }

    /// 连续追索源变量身份。 / Continuous-recourse source-variable identities.
    pub fn recourse_variables(&self) -> &BTreeSet<VariableId> { &self.recourse }

    fn require_current(&self, source: &MetaModel) -> Result<()> { source.require_linear_model_revision(self.snapshot.revision) }

    fn master_model(&self) -> Result<MetaModel> {
        let mut model = MetaModel::new("fixed_recourse_master");
        for state in self.snapshot.variables.iter().filter(|state| self.master.contains(&state.variable)) { register(&mut model, state)?; }
        let mut coefficients = self.snapshot.objective.iter().filter(|(variable, _)| self.master.contains(variable))
            .map(|(variable, coefficient)| (*variable, *coefficient)).collect::<BTreeMap<_, _>>();
        if let Some(theta) = self.theta {
            register(&mut model, &LinearVariableState { variable: theta, name: "recourse_theta".into(), var_type: VariableType::Continuous,
                lower_bound: Some(self.theta_lower), upper_bound: None })?;
            coefficients.insert(theta, 1.0);
        }
        if model.tokens().is_empty() {
            register(&mut model, &LinearVariableState { variable: new_standalone_id(), name: "empty_master_anchor".into(),
                var_type: VariableType::Continuous, lower_bound: Some(0.0), upper_bound: Some(0.0) })?;
        }
        for row in self.rows.iter().filter(|row| row.recourse.is_empty()) { add_row(&mut model, &row.master, row.rhs, &row.name)?; }
        objective(&mut model, &coefficients, self.snapshot.objective_constant);
        Ok(model)
    }

    fn subproblem(&self, anchor: &BTreeMap<VariableId, f64>) -> Result<MetaModel> {
        let mut model = MetaModel::new("fixed_recourse_subproblem");
        for state in self.snapshot.variables.iter().filter(|state| self.recourse.contains(&state.variable)) {
            register(&mut model, &LinearVariableState { var_type: VariableType::Continuous, lower_bound: None, upper_bound: None, ..state.clone() })?;
        }
        for row in self.rows.iter().filter(|row| !row.recourse.is_empty()) {
            let rhs = row.master.iter().try_fold(exact(row.rhs)?, |rhs, (variable, coefficient)|
                Ok::<_, CoreError>(rhs - exact(*coefficient)? * exact(anchor[variable])?))?;
            let represented = rounded(&rhs, true)?;
            // 非精确 RHS 会改变证书所证明的系统，不能静默舍入。 / An inexact RHS changes the certified system and must not be silently rounded.
            if exact(represented)? != rhs { return Err(unsupported()); }
            add_row(&mut model, &row.recourse, represented, &row.name)?;
        }
        objective(&mut model, &self.snapshot.objective, 0.0);
        Ok(model)
    }

    fn cut(&self, multipliers: &[f64], optimal: bool) -> Result<(BTreeMap<VariableId, f64>, f64)> {
        let rows = self.rows.iter().filter(|row| !row.recourse.is_empty()).collect::<Vec<_>>();
        if rows.len() != multipliers.len() { return Err(invalid("certificate/cut row mismatch")); }
        let exact = |value| BigRational::from_float(value).ok_or_else(|| invalid("nonfinite cut operand"));
        let unrepresentable = || CoreError::NotImplemented("a certified cut is not safely representable as finite f64; use a fresh full MILP".into());
        let mut coefficients = BTreeMap::<VariableId, BigRational>::new();
        let mut rhs = BigRational::zero();
        let sign = if optimal { -1.0 } else { 1.0 };
        for (row, multiplier) in rows.iter().zip(multipliers) {
            let multiplier = exact(sign)? * exact(*multiplier)?;
            rhs += &multiplier * exact(row.rhs)?;
            for (variable, coefficient) in &row.master {
                *coefficients.entry(*variable).or_insert_with(BigRational::zero) += &multiplier * exact(*coefficient)?;
            }
        }
        if optimal { coefficients.insert(self.theta.ok_or_else(|| invalid("optimality cut without theta"))?, exact(-1.0)?); }
        let mut rounded = BTreeMap::new();
        for (variable, coefficient) in coefficients {
            let value = coefficient.to_f64().filter(|value| value.is_finite()).ok_or_else(unrepresentable)?;
            let error = coefficient - exact(value)?;
            if !error.is_zero() {
                let state = self.snapshot.variables.iter().find(|state| state.variable == variable).ok_or_else(|| invalid("unknown rounded-cut variable"))?.relaxed()?;
                let bound = if error > BigRational::zero() { state.lower_bound } else { state.upper_bound }
                    .ok_or_else(|| CoreError::NotImplemented("cut rounding has an unbounded error direction; use a fresh full MILP".into()))?;
                rhs -= error * exact(bound)?;
            }
            if value != 0.0 { rounded.insert(variable, value); }
        }
        // 仅向外舍入 RHS；系数舍入误差已按真实 master 边界保守补偿。
        // Round the RHS outwards only; coefficient-rounding errors were conservatively compensated using actual master bounds.
        let mut bound = rhs.to_f64().filter(|value| value.is_finite()).ok_or_else(unrepresentable)?;
        if exact(bound)? < rhs { bound = bound.next_up(); }
        if !bound.is_finite() { return Err(unrepresentable()); }
        Ok((rounded, bound))
    }
}

fn checked_report(report: &SolveReport<f64>, model: &LinearTriadModel) -> Result<()> {
    report.validate()?;
    if report.fingerprints.model.as_ref() != Some(&linear_model_fingerprint(model)?)
        || report.model_mapping.as_ref() != Some(&linear_model_mapping(model)?)
    { return Err(invalid("report does not identify the actual solved model")); }
    Ok(())
}

fn incumbent(report: &SolveReport<f64>, snapshot: &LinearModelSnapshot<f64>, tolerance: f64) -> Result<(BTreeMap<VariableId, f64>, f64)> {
    let solution = report.solution.as_ref().ok_or_else(|| invalid("missing incumbent"))?;
    if solution.values.len() != snapshot.variables.len() { return Err(invalid("incomplete incumbent vector")); }
    let values = snapshot.variables.iter().zip(&solution.values).map(|(state, value)| (state.variable, *value)).collect();
    snapshot.validate_solution(&values, &tolerance)?;
    let objective = snapshot.evaluate_objective_flt64(&values)?;
    objective.validate_reported(solution.objective_value.or(solution.objective), tolerance.max(1e-7))?;
    Ok((values, objective.total_value()?))
}

struct Budget<'a> { started: Instant, config: FixedRecourseConfig, handle: &'a SolveHandle }
impl Budget<'_> {
    fn remaining(&self) -> Duration { self.config.time_limit.saturating_sub(self.started.elapsed()) }
    fn stop(&self) -> Option<FixedRecourseStop> {
        if self.handle.is_cancelled() { Some(FixedRecourseStop::Cancelled) }
        else if self.remaining().is_zero() { Some(FixedRecourseStop::TimeLimit) } else { None }
    }
}

fn report_stop(report: &SolveReport<f64>) -> Option<FixedRecourseStop> {
    match report.termination_reason {
        TerminationReason::Cancelled | TerminationReason::Interrupted => Some(FixedRecourseStop::Cancelled),
        TerminationReason::TimeLimit => Some(FixedRecourseStop::TimeLimit),
        _ => None,
    }
}

enum Certificate { Values(Vec<f64>), Stop(FixedRecourseStop) }

fn certificate<F>(
    model: &LinearTriadModel,
    report: &SolveReport<f64>,
    optimal: bool,
    budget: &Budget<'_>,
    solve: &mut F,
) -> Result<Certificate>
where F: FnMut(&LinearTriadModel, Duration, &SolveHandle) -> Result<SolveReport<f64>> {
    let raw = if optimal { report.solution.as_ref().and_then(|solution| solution.dual_solution.clone()) }
        else { report.proof.as_ref().and_then(|proof| proof.evidence.clone()) };
    let mut dual = raw;
    if dual.is_none() && budget.config.recover_missing_certificates {
        if let Some(stop) = budget.stop() { return Ok(Certificate::Stop(stop)); }
        let recovery = if optimal { model.to_dual() } else { model.to_farkas_dual() };
        let recovered = solve(&recovery, budget.remaining(), budget.handle)?;
        if let Some(stop) = budget.stop().or_else(|| report_stop(&recovered)) { return Ok(Certificate::Stop(stop)); }
        checked_report(&recovered, &recovery)?;
        if !recovered.is_optimal() {
            return Ok(Certificate::Stop(FixedRecourseStop::SubproblemTermination(recovered.problem_status, recovered.termination_reason)));
        }
        dual = recovered.solution.as_ref().map(|solution| solution.values.clone());
    }
    let Some(mut dual) = dual else { return Ok(Certificate::Stop(FixedRecourseStop::CertificateMissing)); };
    if dual.len() != model.num_constraints() { return Ok(Certificate::Stop(FixedRecourseStop::CertificateInvalid)); }
    if optimal {
        let mut candidate = report.clone();
        if let Some(solution) = &mut candidate.solution { solution.dual_solution = Some(dual.clone()); }
        if require_optimal_lp_certificate_for_linear_model(&candidate, model).is_err() {
            return Ok(Certificate::Stop(FixedRecourseStop::CertificateInvalid));
        }
    } else {
        if dual.iter().all(|value| *value <= 0.0) { for value in &mut dual { *value = -*value; } }
        if validate_linear_farkas_ray(model, &dual, budget.config.tolerance).is_err() {
            return Ok(Certificate::Stop(FixedRecourseStop::CertificateInvalid));
        }
    }
    Ok(Certificate::Values(dual))
}

/// 受控固定追索算法；不接受调用方自制切割或裸对偶。 / Controlled fixed-recourse algorithm accepting neither caller-built cuts nor bare duals.
pub struct FixedRecourseAlgorithm;

impl FixedRecourseAlgorithm {
    /// 在同域 f64 边界执行完整模型分解，切割只由本轮实际 LP 的已验证证书产生。
    /// Decompose the full model at a same-domain f64 boundary; cuts use only verified certificates from this round's actual LP.
    pub fn solve<F>(
        source: &MetaModel,
        partition: &LinearBendersPartition,
        config: FixedRecourseConfig,
        handle: &SolveHandle,
        mut solve: F,
    ) -> Result<FixedRecourseResult>
    where F: FnMut(&LinearTriadModel, Duration, &SolveHandle) -> Result<SolveReport<f64>> {
        if config.iteration_limit == 0 || !config.tolerance.is_finite() || config.tolerance <= 0.0 || config.tolerance >= 0.5 {
            return Err(invalid("invalid iteration budget or tolerance"));
        }
        let budget = Budget { started: Instant::now(), config, handle };
        let mut result = FixedRecourseResult { backend_error: None, stop: FixedRecourseStop::IterationLimit, values: BTreeMap::new(), objective: None, lower_bound: None,
            master_report: None, subproblem_report: None, iterations: 0, feasibility_cuts: 0, optimality_cuts: 0 };
        partition.require_current(source)?;
        if let Some(stop) = budget.stop() { result.stop = stop; return Ok(result); }
        let mut master = partition.master_model()?;
        for iteration in 0..config.iteration_limit {
            partition.require_current(source)?;
            if let Some(stop) = budget.stop() { result.stop = stop; break; }
            let master_snapshot = master.linear_model_snapshot()?;
            let master_compiled = master_snapshot.compile_flt64()?;
            let report = match solve(&master_compiled.model, budget.remaining(), handle) {
                Ok(report) => report,
                Err(error) => { result.retain_backend_error(error, handle)?; break; }
            };
            partition.require_current(source)?;
            let execution_stop = budget.stop().or_else(|| report_stop(&report));
            report.validate()?;
            if report.has_incumbent() || execution_stop.is_none() { checked_report(&report, &master_compiled.model)?; }
            result.iterations = iteration + 1;
            result.master_report = Some(report.clone());
            if !report.has_incumbent() {
                result.stop = execution_stop.unwrap_or(FixedRecourseStop::MasterTermination(report.problem_status, report.termination_reason)); break;
            }
            let (anchor, upper) = incumbent(&report, &master_snapshot, config.tolerance)?;
            let lower = master_snapshot.evaluate_objective_flt64(&anchor)?.lower_bound()?;
            let master_optimal = report.is_optimal();
            if master_optimal { result.lower_bound = Some(result.lower_bound.map_or(lower, |previous| previous.max(lower))); }
            if partition.recourse.is_empty() {
                let values = anchor.into_iter().filter(|(variable, _)| partition.master.contains(variable)).collect();
                partition.snapshot.validate_solution(&values, &config.tolerance)?;
                result.values = values;
                result.objective = Some(upper);
                result.stop = execution_stop.or_else(|| budget.stop()).unwrap_or(if master_optimal { FixedRecourseStop::Converged }
                    else { FixedRecourseStop::MasterTermination(report.problem_status, report.termination_reason) });
                break;
            }
            if let Some(stop) = execution_stop.or_else(|| budget.stop()) { result.stop = stop; break; }
            let sub = match partition.subproblem(&anchor) {
                Ok(sub) => sub,
                Err(error) if error.solver_error_class() == ospf_rust_core::error::SolverErrorClass::Unsupported => {
                    result.stop = FixedRecourseStop::Unsupported; break;
                }
                Err(error) => return Err(error),
            };
            let sub_snapshot = sub.linear_model_snapshot()?;
            let sub_compiled = sub_snapshot.compile_flt64()?;
            if let Some(stop) = budget.stop() { result.stop = stop; break; }
            let sub_report = match solve(&sub_compiled.model, budget.remaining(), handle) {
                Ok(report) => report,
                Err(error) => { result.retain_backend_error(error, handle)?; break; }
            };
            partition.require_current(source)?;
            let execution_stop = budget.stop().or_else(|| report_stop(&sub_report));
            sub_report.validate()?;
            if sub_report.has_incumbent() || execution_stop.is_none() { checked_report(&sub_report, &sub_compiled.model)?; }
            result.subproblem_report = Some(sub_report.clone());
            let feasible = sub_report.has_incumbent();
            if feasible {
                let (recourse, _) = incumbent(&sub_report, &sub_snapshot, config.tolerance)?;
                let values = partition.snapshot.variables.iter().map(|state| (state.variable,
                    if partition.master.contains(&state.variable) { anchor[&state.variable] } else { recourse[&state.variable] })).collect::<BTreeMap<_, _>>();
                partition.snapshot.validate_solution(&values, &config.tolerance)?;
                let upper = partition.snapshot.evaluate_objective_flt64(&values)?.total_value()?;
                if result.objective.is_none_or(|previous| upper < previous) { result.objective = Some(upper); result.values = values; }
                if let Some(stop) = execution_stop.or_else(|| budget.stop()) { result.stop = stop; break; }
                if !sub_report.is_optimal() { result.stop = FixedRecourseStop::SubproblemTermination(sub_report.problem_status, sub_report.termination_reason); break; }
            } else if let Some(stop) = execution_stop { result.stop = stop; break; }
            else if sub_report.problem_status == ProblemStatus::Infeasible {
                if require_infeasibility_certificate_for_model(&sub_report, &linear_model_fingerprint(&sub_compiled.model)?).is_err() {
                    result.stop = if sub_report.proof.is_none() { FixedRecourseStop::CertificateMissing } else { FixedRecourseStop::CertificateInvalid }; break;
                }
            } else { result.stop = FixedRecourseStop::SubproblemTermination(sub_report.problem_status, sub_report.termination_reason); break; }
            if !feasible || partition.theta.is_some() {
                let evidence = match certificate(&sub_compiled.model, &sub_report, feasible, &budget, &mut solve) {
                    Ok(evidence) => evidence,
                    Err(error) => { result.retain_backend_error(error, handle)?; break; }
                };
                partition.require_current(source)?;
                if let Some(stop) = budget.stop() { result.stop = stop; break; }
                let multipliers = match evidence {
                    Certificate::Values(values) => values,
                    Certificate::Stop(stop) => { result.stop = stop; break; }
                };
                let (coefficients, rhs) = match partition.cut(&multipliers, feasible) {
                    Ok(cut) => cut,
                    Err(error) if error.solver_error_class() == ospf_rust_core::error::SolverErrorClass::Unsupported => {
                        result.stop = FixedRecourseStop::Unsupported; break;
                    }
                    Err(error) => return Err(error),
                };
                let lhs = coefficients.iter().try_fold(BigRational::zero(), |total, (variable, coefficient)|
                    Ok::<_, CoreError>(total + exact(*coefficient)? * exact(anchor[variable])?))?;
                let rhs = exact(rhs)?;
                if lhs > &rhs + exact(config.tolerance)? * lhs.abs().max(rhs.abs()).max(BigRational::one()) {
                    add_row(&mut master, &coefficients, rounded(&rhs, true)?, &format!("verified_cut_{iteration}"))?;
                    if feasible { result.optimality_cuts += 1; } else { result.feasibility_cuts += 1; }
                } else if !feasible { result.stop = FixedRecourseStop::CutStall; break; }
            }
            if let Some(stop) = budget.stop() { result.stop = stop; break; }
            if !master_optimal { result.stop = FixedRecourseStop::MasterTermination(report.problem_status, report.termination_reason); break; }
            if let (Some(lower), Some(upper)) = (result.lower_bound, result.objective) {
                if lower > upper + config.tolerance * lower.abs().max(upper.abs()).max(1.0) { return Err(invalid("certified bounds contradict the source incumbent")); }
                if upper - lower <= config.tolerance * lower.abs().max(upper.abs()).max(1.0) { result.stop = FixedRecourseStop::Converged; break; }
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests;
