//! 约束规划求解 SPI 与合同 fake / Constraint-programming solver SPI and contract fake.
//!
//! 本模块只提供精确 CP 快照的求解边界和离线穷举实现。生产 backend 必须在此边界
//! 返回统一 `SolveReport<i64>`，不能定义平行终态。
//! This module provides the exact CP snapshot boundary and an offline exhaustive implementation.
//! Production backends must return the unified `SolveReport<i64>` at this boundary and must not
//! define parallel terminals.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use crate::error::{CoreError, ModelError, Result, SolverError};
use crate::model::constraint_programming::{
    BooleanLiteral, ConstraintProgrammingSnapshot, IntegerVariable,
};
use crate::solver::{
    ProblemStatus, SolveDiagnostics, SolveFingerprints, SolveHandle, SolveProgressReporter,
    SolveProgressSnapshot, SolveProof, SolveReport, SolveSolution, SolveStage, SolveStatistics,
    SolverCapability, SolverDescriptor, SolverInfo, SolverProvenance, StableVariableId,
    TerminationReason, solver_provenance_fingerprint,
};

pub mod conflict;
pub mod mip;

#[cfg(feature = "scip")]
mod scip;

#[cfg(feature = "serde")]
pub mod checkpoint;

pub use conflict::*;
pub use mip::*;

#[cfg(feature = "scip")]
pub use scip::*;

#[cfg(feature = "serde")]
pub use checkpoint::*;

/// 根据 assumptions 构建可审计的 CP 快照 / Build an auditable CP snapshot with assumptions.
///
/// fake 与 MIP-backed session 必须使用同一套 canonical 编码，避免同一组 assumptions
/// 产生不同的 report model fingerprint。/ Fake and MIP-backed sessions use the same canonical
/// encoding so identical assumptions cannot produce different report model fingerprints.
pub fn snapshot_with_assumptions(
    snapshot: &ConstraintProgrammingSnapshot,
    assumptions: &[ConstraintProgrammingAssumption],
) -> Result<ConstraintProgrammingSnapshot> {
    mip::snapshot_with_assumptions(snapshot, assumptions)
}

fn invalid(message: impl Into<String>) -> CoreError {
    ModelError::ConstraintProgramming(message.into()).into()
}

/// CP 后端对单项能力的支持等级 / Backend support level for one CP capability.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstraintProgrammingSupport {
    /// 后端直接表达 / Expressed directly by the backend.
    Native,
    /// 通过已证明等价的有限降维表达 / Expressed by a proven exact finite lowering.
    ExactLowering,
    /// 仅在明确运行时条件成立时支持 / Supported only under explicit runtime conditions.
    Conditional,
    /// 当前不支持 / Currently unsupported.
    Unsupported,
}

/// CP assumption 的稳定身份 / Stable identity of a CP assumption.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConstraintProgrammingAssumptionId(pub String);

impl std::fmt::Display for ConstraintProgrammingAssumptionId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// CP 支持分析报告 / CP support-analysis report.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintProgrammingSupportReport {
    /// 按稳定约束 ID 的支持等级 / Support level by stable constraint ID.
    pub constraints: BTreeMap<crate::solver::StableConstraintId, ConstraintProgrammingSupport>,
    /// 是否支持 satisfaction / Whether satisfaction is supported.
    pub satisfaction: bool,
    /// 是否支持单整数目标 / Whether a single integer objective is supported.
    pub integer_objective: bool,
    /// 是否支持稀疏值域 / Whether sparse domains are supported.
    pub sparse_domain: bool,
    /// 是否支持一次性求解 / Whether one-shot solving is supported.
    pub one_shot: bool,
    /// 是否支持 snapshot 重建 session / Whether snapshot-rebuild sessions are supported.
    pub rebuild_session: bool,
    /// 是否支持真正增量 session / Whether a true incremental session is supported.
    pub incremental_session: bool,
    /// 是否支持 assumptions / Whether assumptions are supported.
    pub assumptions: bool,
    /// 是否支持统一取消 / Whether unified cancellation is supported.
    pub cancellation: bool,
    /// 是否接受经过校验的 solution hint / Whether validated solution hints are accepted.
    pub solution_hint: bool,
    /// 是否能生成经过验证的 conflict seed / Whether a verified conflict seed can be produced.
    pub verified_conflict_seed: bool,
    /// 是否能执行并验证不可约 conflict shrinking / Whether irreducible conflict shrinking is supported.
    pub irreducible_conflict: bool,
    /// 是否支持统一 progress 上报 / Whether unified progress reporting is supported.
    pub progress: bool,
    /// 是否保证确定性搜索或显式确定性配置 / Whether deterministic search or an explicit deterministic configuration is guaranteed.
    pub deterministic: bool,
    /// 是否支持解池 / Whether a solution pool is supported.
    pub solution_pool: bool,
    /// 结构化说明 / Structured explanation.
    pub notes: Vec<String>,
}

impl ConstraintProgrammingSupportReport {
    fn exhaustive(snapshot: &ConstraintProgrammingSnapshot) -> Self {
        let constraints = snapshot
            .constraints
            .iter()
            .map(|constraint| (constraint.id.clone(), ConstraintProgrammingSupport::Native))
            .collect();
        Self {
            constraints,
            satisfaction: true,
            integer_objective: true,
            sparse_domain: true,
            one_shot: true,
            rebuild_session: true,
            incremental_session: false,
            assumptions: true,
            cancellation: true,
            solution_hint: true,
            verified_conflict_seed: true,
            irreducible_conflict: true,
            progress: true,
            deterministic: true,
            solution_pool: false,
            notes: vec![
                "support is provided by deterministic source-constraint enumeration".to_owned(),
            ],
        }
    }
}

/// CP 求解参数 / Constraint-programming solve options.
#[derive(Clone, Copy)]
pub struct ConstraintProgrammingSolveOptions<'a> {
    /// 时间上限 / Time limit.
    pub time_limit: Option<Duration>,
    /// 赋值枚举节点上限 / Assignment-enumeration node limit.
    pub node_limit: Option<usize>,
    /// 找到可行解的数量上限 / Feasible-solution limit.
    pub solution_limit: Option<usize>,
    /// 单个变量域允许的最大枚举规模 / Maximum enumeration size per variable domain.
    pub enumeration_limit: usize,
    /// 稳定变量到提示值的映射 / Stable-variable solution hint.
    pub solution_hint: Option<&'a BTreeMap<StableVariableId, i64>>,
    /// 取消句柄 / Cancellation handle.
    pub cancellation_handle: Option<&'a SolveHandle>,
    /// 统一进度上报器 / Unified progress reporter.
    pub progress_reporter: Option<&'a SolveProgressReporter>,
    /// 是否请求 CP conflict 诊断 / Whether to request CP conflict diagnostics.
    pub request_conflict: bool,
    /// 是否对 verified conflict seed 执行删除收缩 / Whether to deletion-shrink a verified conflict seed.
    pub shrink_conflict: bool,
    /// conflict 诊断最多执行的额外重求解次数 / Maximum additional conflict re-solves.
    pub max_conflict_resolves: usize,
}

impl<'a> Default for ConstraintProgrammingSolveOptions<'a> {
    fn default() -> Self {
        Self {
            time_limit: None,
            node_limit: None,
            solution_limit: None,
            enumeration_limit: 10_000,
            solution_hint: None,
            cancellation_handle: None,
            progress_reporter: None,
            request_conflict: false,
            shrink_conflict: false,
            max_conflict_resolves: 128,
        }
    }
}

impl<'a> ConstraintProgrammingSolveOptions<'a> {
    /// 创建默认参数 / Create default options.
    pub fn new() -> Self {
        Self {
            enumeration_limit: 10_000,
            max_conflict_resolves: 128,
            ..Self::default()
        }
    }

    /// 创建参数构建器 / Create an options builder.
    pub fn builder() -> ConstraintProgrammingSolveOptionsBuilder<'a> {
        ConstraintProgrammingSolveOptionsBuilder::new()
    }

    pub(crate) fn validate(&self) -> Result<()> {
        // 零值限制表示立即达到对应终态；与通用 MIP options 的输入校验保持隔离。
        // A zero CP limit means that the corresponding limit is reached immediately; keep this
        // contract separate from the legacy generic MIP options validation.
        if self.enumeration_limit == 0 {
            return Err(invalid("CP enumeration limit must be positive"));
        }
        if self.shrink_conflict && !self.request_conflict {
            return Err(invalid(
                "CP conflict shrinking requires request_conflict to be enabled",
            ));
        }
        if self.request_conflict && self.max_conflict_resolves == 0 {
            return Err(invalid(
                "CP conflict resolve budget must be positive when conflict diagnostics are requested",
            ));
        }
        Ok(())
    }

    fn without_conflict(self) -> Self {
        Self {
            request_conflict: false,
            shrink_conflict: false,
            ..self
        }
    }
}

/// CP 求解参数构建器 / CP solve-options builder.
#[derive(Clone, Copy, Default)]
pub struct ConstraintProgrammingSolveOptionsBuilder<'a> {
    options: ConstraintProgrammingSolveOptions<'a>,
}

impl<'a> ConstraintProgrammingSolveOptionsBuilder<'a> {
    /// 创建构建器 / Create a builder.
    pub fn new() -> Self {
        Self {
            options: ConstraintProgrammingSolveOptions::new(),
        }
    }

    /// 设置时间上限 / Set a time limit.
    pub fn time_limit(mut self, limit: Option<Duration>) -> Self {
        self.options.time_limit = limit;
        self
    }

    /// 设置节点上限 / Set a node limit.
    pub fn node_limit(mut self, limit: Option<usize>) -> Self {
        self.options.node_limit = limit;
        self
    }

    /// 设置解数量上限 / Set a solution limit.
    pub fn solution_limit(mut self, limit: Option<usize>) -> Self {
        self.options.solution_limit = limit;
        self
    }

    /// 设置域枚举上限 / Set a domain enumeration limit.
    pub fn enumeration_limit(mut self, limit: usize) -> Self {
        self.options.enumeration_limit = limit;
        self
    }

    /// 设置解提示 / Set a solution hint.
    pub fn solution_hint(mut self, hint: Option<&'a BTreeMap<StableVariableId, i64>>) -> Self {
        self.options.solution_hint = hint;
        self
    }

    /// 设置取消句柄 / Set a cancellation handle.
    pub fn cancellation_handle(mut self, handle: Option<&'a SolveHandle>) -> Self {
        self.options.cancellation_handle = handle;
        self
    }

    /// 设置进度上报器 / Set a progress reporter.
    pub fn progress_reporter(mut self, reporter: Option<&'a SolveProgressReporter>) -> Self {
        self.options.progress_reporter = reporter;
        self
    }

    /// 请求 conflict 诊断 / Request conflict diagnostics.
    pub fn request_conflict(mut self, request: bool) -> Self {
        self.options.request_conflict = request;
        self
    }

    /// 设置 conflict 删除收缩 / Enable conflict deletion shrinking.
    pub fn shrink_conflict(mut self, shrink: bool) -> Self {
        self.options.shrink_conflict = shrink;
        self
    }

    /// 设置 conflict 重求解预算 / Set the conflict re-solve budget.
    pub fn max_conflict_resolves(mut self, limit: usize) -> Self {
        self.options.max_conflict_resolves = limit;
        self
    }

    /// 完成参数构造 / Finish building options.
    pub fn finish(self) -> ConstraintProgrammingSolveOptions<'a> {
        self.options
    }
}

/// CP 求解假设 / CP solving assumption.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConstraintProgrammingAssumption {
    /// 要求 Boolean literal 为真 / Require a Boolean literal to be true.
    Literal(BooleanLiteral),
    /// 固定整数变量 / Fix an integer variable.
    Equal(IntegerVariable, i64),
    /// 收紧下界 / Tighten a lower bound.
    LowerBound(IntegerVariable, i64),
    /// 收紧上界 / Tighten an upper bound.
    UpperBound(IntegerVariable, i64),
}

impl ConstraintProgrammingAssumption {
    /// 返回确定性的 assumption 身份 / Return a deterministic assumption identity.
    pub fn stable_id(&self) -> ConstraintProgrammingAssumptionId {
        fn component(value: &str) -> String {
            format!("{}:{value}", value.len())
        }

        match self {
            Self::Literal(literal) => ConstraintProgrammingAssumptionId(format!(
                "literal/{}/{}",
                component(&literal.variable.stable_id.0),
                if literal.negated { "false" } else { "true" }
            )),
            Self::Equal(variable, value) => ConstraintProgrammingAssumptionId(format!(
                "equal/{}/{}",
                component(&variable.stable_id.0),
                value
            )),
            Self::LowerBound(variable, value) => ConstraintProgrammingAssumptionId(format!(
                "lower-bound/{}/{}",
                component(&variable.stable_id.0),
                value
            )),
            Self::UpperBound(variable, value) => ConstraintProgrammingAssumptionId(format!(
                "upper-bound/{}/{}",
                component(&variable.stable_id.0),
                value
            )),
        }
    }
}

/// CP 求解会话 / CP solve session.
pub trait ConstraintProgrammingSession: Send {
    /// 在假设下求解 / Solve under assumptions.
    fn solve_with_assumptions(
        &mut self,
        assumptions: &[ConstraintProgrammingAssumption],
        options: &ConstraintProgrammingSolveOptions<'_>,
    ) -> Result<SolveReport<i64>>;
}

/// 约束规划求解器 / Constraint-programming solver.
pub trait ConstraintProgrammingSolver: SolverInfo {
    /// 查询给定快照的实际支持情况 / Analyze effective support for a snapshot.
    fn analyze_support(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
    ) -> ConstraintProgrammingSupportReport;

    /// 求解不可变快照 / Solve an immutable snapshot.
    fn solve_constraint_programming(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
        options: &ConstraintProgrammingSolveOptions<'_>,
    ) -> Result<SolveReport<i64>>;

    /// 求解并按统一报告附加 CP conflict 诊断 / Solve and attach CP conflict diagnostics to the unified report.
    ///
    /// conflict 只在 base model 与完整 assumptions 都通过 verified infeasibility gate 后产生。
    /// 删除收缩被取消、限额或 backend failure 打断时，保留最后一个 verified seed 并标记为 partial。
    /// A conflict is produced only after both the base model and the full assumption set pass the
    /// verified-infeasibility gate. If deletion shrinking is interrupted by cancellation, a limit,
    /// or backend failure, the last verified seed is retained and marked partial.
    fn solve_constraint_programming_with_conflict(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
        options: &ConstraintProgrammingSolveOptions<'_>,
    ) -> Result<SolveReport<i64>> {
        self.solve_constraint_programming_with_assumptions_and_conflict(snapshot, &[], options)
    }

    /// 在 assumptions 下求解并附加 CP conflict 诊断 / Solve under assumptions and attach CP conflict diagnostics.
    fn solve_constraint_programming_with_assumptions_and_conflict(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
        assumptions: &[ConstraintProgrammingAssumption],
        options: &ConstraintProgrammingSolveOptions<'_>,
    ) -> Result<SolveReport<i64>> {
        options.validate()?;
        validate_conflict_assumption_ids(assumptions)?;
        let solve_options = options.without_conflict();
        if !options.request_conflict && assumptions.is_empty() {
            return self.solve_constraint_programming(snapshot, &solve_options);
        }
        let mut session = match self.create_session(snapshot) {
            Ok(session) => session,
            Err(error) if assumptions.is_empty() => {
                let mut report = self.solve_constraint_programming(snapshot, &solve_options)?;
                attach_unavailable_conflict(&mut report, error.to_string());
                return Ok(report);
            }
            Err(error) => return Err(error),
        };
        let mut report = session.solve_with_assumptions(assumptions, &solve_options)?;
        if !options.request_conflict {
            return Ok(report);
        }

        if report
            .diagnostics
            .infeasibility_evidence
            .as_ref()
            .is_some_and(crate::solver::InfeasibilityEvidence::is_authoritative)
        {
            report.diagnostics.issues.push(crate::solver::SolveIssue::new(
                "CpConflictAuthoritativeEvidencePreserved",
                "native or authoritative infeasibility evidence was retained without CP fallback re-solving",
            ));
            return Ok(report);
        }

        match compute_constraint_programming_conflict(
            &mut *session,
            snapshot,
            assumptions,
            &report,
            &solve_options,
            options,
        ) {
            Ok(conflict) => conflict.attach_to_report(&mut report),
            Err(error) => attach_unavailable_conflict(&mut report, error.to_string()),
        }
        Ok(report)
    }

    /// 创建求解会话 / Create a solve session.
    fn create_session(
        &self,
        _snapshot: &ConstraintProgrammingSnapshot,
    ) -> Result<Box<dyn ConstraintProgrammingSession>> {
        Err(CoreError::Solver(SolverError::UnsupportedValueType(
            "constraint-programming session is not supported by this solver".to_owned(),
        )))
    }
}

/// 离线穷举 CP solver / Offline exhaustive CP solver.
///
/// 该实现用于无 native solver 环境的合同测试和小实例 oracle，不是生产级 CP 搜索器。
/// This implementation is a contract-test and small-instance oracle for environments without a
/// native solver; it is not a production CP search engine.
#[derive(Debug, Clone, Copy, Default)]
pub struct FakeConstraintProgrammingSolver;

impl FakeConstraintProgrammingSolver {
    /// 创建 fake solver / Create the fake solver.
    pub const fn new() -> Self {
        Self
    }

    fn solve_with_assumptions(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
        assumptions: &[ConstraintProgrammingAssumption],
        options: &ConstraintProgrammingSolveOptions<'_>,
    ) -> Result<SolveReport<i64>> {
        options.validate()?;
        validate_conflict_assumption_ids(assumptions)?;
        snapshot.validate_identity()?;
        validate_assumptions(snapshot, assumptions)?;
        let base_snapshot = snapshot;
        let snapshot = snapshot_with_assumptions(base_snapshot, assumptions)?;
        validate_hint(base_snapshot, options.solution_hint)?;

        let provenance = SolverProvenance {
            solver_id: self.name().to_owned(),
            backend_name: self.name().to_owned(),
            deterministic: Some(true),
            ..SolverProvenance::default()
        };
        let solver_fingerprint = solver_provenance_fingerprint(&provenance);
        let started = Instant::now();
        emit_progress(options.progress_reporter, SolveStage::Solving, false, None)?;

        let domains = snapshot
            .variables
            .iter()
            .map(|variable| {
                let mut values = variable.domain.enumerate(options.enumeration_limit)?;
                if let Some(hint_value) = options
                    .solution_hint
                    .and_then(|hint| hint.get(&variable.variable.stable_id))
                    && let Some(index) = values.iter().position(|value| value == hint_value)
                {
                    values.swap(0, index);
                }
                Ok((variable.variable.stable_id.clone(), values))
            })
            .collect::<Result<Vec<_>>>()?;
        let mut iterator =
            CartesianProduct::new(domains.iter().map(|(_, values)| values.clone()).collect());
        let mut best: Option<(BTreeMap<StableVariableId, i64>, Option<i64>)> = None;
        let mut evaluated = 0usize;
        let mut feasible_count = 0usize;
        let mut stop_reason = None;

        // 完整 hint 在立即时间限制下仍是一个可保留的 incumbent，但不能因此伪造完成证明。
        // A complete hint remains a usable incumbent under an immediate time limit, but it must
        // not be promoted to a completion proof merely because it was supplied by the caller.
        let complete_hint =
            validate_complete_hint(base_snapshot, options.solution_hint, assumptions)?;
        if let Some(values) = complete_hint {
            let objective = snapshot.objective_value(&values)?;
            best = Some((values, objective));
            feasible_count = 1;
        }

        let immediate_time_limit = options.time_limit.is_some_and(|limit| limit.is_zero());
        let immediate_node_limit = options.node_limit == Some(0);
        let immediate_solution_limit = options.solution_limit == Some(0);
        if immediate_time_limit || immediate_node_limit || immediate_solution_limit {
            stop_reason = if options
                .cancellation_handle
                .is_some_and(SolveHandle::is_cancelled)
            {
                Some(TerminationReason::Cancelled)
            } else if immediate_time_limit {
                Some(TerminationReason::TimeLimit)
            } else if immediate_node_limit {
                Some(TerminationReason::NodeLimit)
            } else {
                Some(TerminationReason::SolutionLimit)
            };
        }

        while stop_reason.is_none() {
            let Some(candidate_values) = iterator.next() else {
                break;
            };
            if options
                .cancellation_handle
                .is_some_and(SolveHandle::is_cancelled)
            {
                stop_reason = Some(TerminationReason::Cancelled);
                break;
            }
            if options
                .time_limit
                .is_some_and(|limit| started.elapsed() >= limit)
            {
                stop_reason = Some(TerminationReason::TimeLimit);
                break;
            }
            if options.node_limit.is_some_and(|limit| evaluated >= limit) {
                stop_reason = Some(TerminationReason::NodeLimit);
                break;
            }
            evaluated = evaluated.saturating_add(1);
            let values = domains
                .iter()
                .zip(candidate_values)
                .map(|((id, _), value)| (id.clone(), value))
                .collect::<BTreeMap<_, _>>();
            if !assumptions_hold(assumptions, &values)? {
                continue;
            }
            if snapshot.validate_assignment(&values).is_err() {
                continue;
            }
            feasible_count = feasible_count.saturating_add(1);
            let objective = snapshot.objective_value(&values)?;
            if best.is_none()
                || best
                    .as_ref()
                    .is_some_and(|(_, incumbent)| better(&snapshot, objective, *incumbent))
            {
                best = Some((values, objective));
            }
            if snapshot.objective.is_none() && options.solution_limit.is_none() {
                break;
            }
            if options
                .solution_limit
                .is_some_and(|limit| feasible_count >= limit && !iterator.is_exhausted())
            {
                stop_reason = Some(TerminationReason::SolutionLimit);
                break;
            }
        }

        let completed = stop_reason.is_none();
        let termination = stop_reason.unwrap_or(TerminationReason::Completed);
        let status = if best.is_some() {
            ProblemStatus::Feasible
        } else if completed {
            ProblemStatus::Infeasible
        } else {
            ProblemStatus::Unknown
        };
        let mut diagnostics = SolveDiagnostics::default();
        diagnostics
            .extensions
            .insert("cp.enumeratedAssignments".to_owned(), evaluated.to_string());
        diagnostics.extensions.insert(
            "cp.feasibleAssignments".to_owned(),
            feasible_count.to_string(),
        );
        if let Some(handle) = options.cancellation_handle
            && let Some(cancellation) = handle.cancellation()
        {
            diagnostics.extensions.insert(
                "cancellation.origin".to_owned(),
                cancellation.origin.to_string(),
            );
            diagnostics.extensions.insert(
                "cancellation.requestedAtEpochMs".to_owned(),
                cancellation.requested_at_epoch_ms.to_string(),
            );
        }
        let objective_bound = objective_bound(&snapshot)?;
        let exact_bound = if completed {
            best.as_ref().and_then(|(_, objective)| *objective)
        } else {
            objective_bound
        };
        let objective_value = best
            .as_ref()
            .and_then(|(_, objective)| *objective)
            .and_then(f64_snapshot);
        let best_bound_value = exact_bound.and_then(f64_snapshot);
        let (absolute_gap, relative_gap) = objective_value
            .zip(best_bound_value)
            .map(|(objective, bound)| {
                let absolute = (objective - bound).abs();
                (Some(absolute), Some(absolute / objective.abs().max(1.0)))
            })
            .unwrap_or((None, None));
        let statistics = SolveStatistics {
            solve_time: started.elapsed(),
            nodes: Some(evaluated),
            best_bound: exact_bound,
            best_bound_value,
            absolute_gap,
            relative_gap,
            solution_count: Some(feasible_count),
            ..SolveStatistics::default()
        };

        let mut builder = SolveReport::builder(status, termination)
            .statistics(statistics)
            .diagnostics(diagnostics)
            .provenance(provenance)
            .fingerprints(SolveFingerprints {
                model: Some(snapshot.fingerprint.clone()),
                configuration: None,
                solver: Some(solver_fingerprint),
            });
        if let Some((values, objective)) = best {
            let vector = snapshot
                .variables
                .iter()
                .map(|variable| values[&variable.variable.stable_id])
                .collect::<Vec<_>>();
            let mut solution = SolveSolution::vector(vector);
            solution.stable_values = values;
            solution.objective = objective;
            solution.objective_value = objective_value;
            builder = builder.solution(solution);
        }
        if completed && status == ProblemStatus::Feasible && snapshot.objective.is_some() {
            builder = builder.proof(SolveProof::optimality());
        } else if completed && status == ProblemStatus::Infeasible {
            builder = builder.proof(SolveProof::infeasibility());
        }
        let report = builder.build()?;
        if let Some(handle) = options.cancellation_handle {
            handle.mark_completed();
        }
        emit_progress(
            options.progress_reporter,
            SolveStage::Completed,
            true,
            report
                .solution
                .as_ref()
                .and_then(|solution| solution.objective),
        )?;
        Ok(report)
    }
}

impl SolverInfo for FakeConstraintProgrammingSolver {
    fn name(&self) -> &str {
        "fake-constraint-programming"
    }

    fn capabilities(&self) -> Vec<SolverCapability> {
        vec![SolverCapability::ConstraintProgramming]
    }

    fn descriptor(&self) -> SolverDescriptor {
        SolverDescriptor {
            solver_id: self.name().to_owned(),
            display_name: "Fake Constraint Programming Solver".to_owned(),
            backend_name: self.name().to_owned(),
            backend_version: Some("source-enumeration".to_owned()),
            runtime_available: Some(true),
            capabilities: crate::solver::SolverCapabilities::from_legacy(&self.capabilities()),
            warnings: vec!["contract-test exhaustive solver; not production search".to_owned()],
        }
    }
}

impl ConstraintProgrammingSolver for FakeConstraintProgrammingSolver {
    fn analyze_support(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
    ) -> ConstraintProgrammingSupportReport {
        ConstraintProgrammingSupportReport::exhaustive(snapshot)
    }

    fn solve_constraint_programming(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
        options: &ConstraintProgrammingSolveOptions<'_>,
    ) -> Result<SolveReport<i64>> {
        self.solve_with_assumptions(snapshot, &[], options)
    }

    fn create_session(
        &self,
        snapshot: &ConstraintProgrammingSnapshot,
    ) -> Result<Box<dyn ConstraintProgrammingSession>> {
        snapshot.validate_identity()?;
        Ok(Box::new(FakeConstraintProgrammingSession {
            solver: *self,
            snapshot: snapshot.clone(),
        }))
    }
}

/// fake solver 的 snapshot 重建会话 / Snapshot-rebuild session for the fake solver.
pub struct FakeConstraintProgrammingSession {
    solver: FakeConstraintProgrammingSolver,
    snapshot: ConstraintProgrammingSnapshot,
}

impl ConstraintProgrammingSession for FakeConstraintProgrammingSession {
    fn solve_with_assumptions(
        &mut self,
        assumptions: &[ConstraintProgrammingAssumption],
        options: &ConstraintProgrammingSolveOptions<'_>,
    ) -> Result<SolveReport<i64>> {
        self.solver
            .solve_with_assumptions(&self.snapshot, assumptions, options)
    }
}

fn validate_hint(
    snapshot: &ConstraintProgrammingSnapshot,
    hint: Option<&BTreeMap<StableVariableId, i64>>,
) -> Result<()> {
    let Some(hint) = hint else { return Ok(()) };
    for (id, value) in hint {
        let variable = snapshot.variable(id).ok_or_else(|| {
            invalid(format!(
                "solution hint references unknown variable {}",
                id.0
            ))
        })?;
        if !variable.domain.contains(*value) {
            return Err(invalid(format!(
                "solution hint value {}={} is outside the variable domain",
                id.0, value
            )));
        }
    }
    Ok(())
}

pub(crate) fn validate_complete_hint(
    snapshot: &ConstraintProgrammingSnapshot,
    hint: Option<&BTreeMap<StableVariableId, i64>>,
    assumptions: &[ConstraintProgrammingAssumption],
) -> Result<Option<BTreeMap<StableVariableId, i64>>> {
    let Some(hint) = hint else {
        return Ok(None);
    };
    snapshot.validate_hint(hint)?;
    if hint.len() != snapshot.variables.len() {
        return Ok(None);
    }
    let assignment = snapshot
        .variables
        .iter()
        .map(|variable| {
            hint.get(&variable.variable.stable_id)
                .copied()
                .map(|value| (variable.variable.stable_id.clone(), value))
                .ok_or_else(|| invalid("complete CP solution hint is missing a variable"))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    snapshot.validate_assignment(&assignment).map_err(|error| {
        invalid(format!(
            "complete CP solution hint violates the source snapshot: {error}"
        ))
    })?;
    if !assumptions_hold(assumptions, &assignment)? {
        return Err(invalid(
            "complete CP solution hint violates the active assumptions",
        ));
    }
    Ok(Some(assignment))
}

fn validate_assumptions(
    snapshot: &ConstraintProgrammingSnapshot,
    assumptions: &[ConstraintProgrammingAssumption],
) -> Result<()> {
    for assumption in assumptions {
        match assumption {
            ConstraintProgrammingAssumption::Literal(literal) => {
                validate_boolean_variable(snapshot, &literal.variable)?;
            }
            ConstraintProgrammingAssumption::Equal(variable, value) => {
                validate_variable_value(snapshot, variable, *value)?;
            }
            ConstraintProgrammingAssumption::LowerBound(variable, value) => {
                let entry = validate_assumption_variable(snapshot, variable)?;
                if *value > entry.domain.upper() {
                    return Err(invalid(format!(
                        "assumption lower bound exceeds variable domain for {}",
                        variable.stable_id.0
                    )));
                }
            }
            ConstraintProgrammingAssumption::UpperBound(variable, value) => {
                let entry = validate_assumption_variable(snapshot, variable)?;
                if *value < entry.domain.lower() {
                    return Err(invalid(format!(
                        "assumption upper bound is below variable domain for {}",
                        variable.stable_id.0
                    )));
                }
            }
        }
    }
    Ok(())
}

fn validate_boolean_variable(
    snapshot: &ConstraintProgrammingSnapshot,
    variable: &IntegerVariable,
) -> Result<()> {
    let entry = validate_assumption_variable(snapshot, variable)?;
    if !entry.domain.is_boolean() {
        return Err(invalid(format!(
            "assumption variable {} is not Boolean",
            variable.stable_id.0
        )));
    }
    Ok(())
}

fn validate_variable_value(
    snapshot: &ConstraintProgrammingSnapshot,
    variable: &IntegerVariable,
    value: i64,
) -> Result<()> {
    let entry = validate_assumption_variable(snapshot, variable)?;
    if !entry.domain.contains(value) {
        return Err(invalid(format!(
            "assumption value is outside variable domain for {}",
            variable.stable_id.0
        )));
    }
    Ok(())
}

fn validate_assumption_variable<'a>(
    snapshot: &'a ConstraintProgrammingSnapshot,
    variable: &IntegerVariable,
) -> Result<&'a crate::model::constraint_programming::VariableSnapshot> {
    let entry = snapshot.variable(&variable.stable_id).ok_or_else(|| {
        invalid(format!(
            "assumption references unknown variable {}",
            variable.stable_id.0
        ))
    })?;
    entry.domain.validate()?;
    Ok(entry)
}

fn objective_bound(snapshot: &ConstraintProgrammingSnapshot) -> Result<Option<i64>> {
    let Some(objective) = &snapshot.objective else {
        return Ok(None);
    };
    let (lower, upper) = objective
        .expression
        .bounds(|id| snapshot.variable(id).map(|entry| &entry.domain))?;
    Ok(Some(if objective.category.is_minimum() {
        lower
    } else {
        upper
    }))
}

fn f64_snapshot(value: i64) -> Option<f64> {
    const MAX_EXACT_F64_INTEGER: i128 = 1_i128 << 53;
    if i128::from(value).abs() > MAX_EXACT_F64_INTEGER {
        return None;
    }
    let converted = value as f64;
    converted.is_finite().then_some(converted)
}

fn assumptions_hold(
    assumptions: &[ConstraintProgrammingAssumption],
    values: &BTreeMap<StableVariableId, i64>,
) -> Result<bool> {
    for assumption in assumptions {
        let satisfied = match assumption {
            ConstraintProgrammingAssumption::Literal(literal) => literal.evaluate(values)?,
            ConstraintProgrammingAssumption::Equal(variable, expected) => {
                values.get(&variable.stable_id).copied() == Some(*expected)
            }
            ConstraintProgrammingAssumption::LowerBound(variable, lower) => values
                .get(&variable.stable_id)
                .is_some_and(|value| value >= lower),
            ConstraintProgrammingAssumption::UpperBound(variable, upper) => values
                .get(&variable.stable_id)
                .is_some_and(|value| value <= upper),
        };
        if !satisfied {
            return Ok(false);
        }
    }
    Ok(true)
}

fn better(
    snapshot: &ConstraintProgrammingSnapshot,
    candidate: Option<i64>,
    incumbent: Option<i64>,
) -> bool {
    match (candidate, incumbent, snapshot.objective.as_ref()) {
        (Some(candidate), Some(incumbent), Some(objective)) if objective.category.is_minimum() => {
            candidate < incumbent
        }
        (Some(candidate), Some(incumbent), Some(_)) => candidate > incumbent,
        (Some(_), None, Some(_)) => true,
        (Some(_), None, None) => true,
        (Some(_), Some(_), None) => false,
        (None, None, None) => false,
        (None, Some(_), None) => false,
        (None, _, Some(_)) => false,
    }
}

fn emit_progress(
    reporter: Option<&SolveProgressReporter>,
    stage: SolveStage,
    terminal: bool,
    objective: Option<i64>,
) -> Result<()> {
    let Some(reporter) = reporter else {
        return Ok(());
    };
    // i64 values outside the exact f64 range must not be rounded into telemetry.  The typed
    // report remains authoritative; progress only carries an optional lossless snapshot.
    // 超出 f64 精确范围的 i64 不得被四舍五入写入遥测；typed report 仍是权威结果，进度只携带
    // 可无损转换的可选快照。
    let objective = objective.and_then(f64_snapshot);
    let snapshot = SolveProgressSnapshot::new(
        "cp",
        stage,
        vec!["cp".to_owned(), stage.stable_name().to_owned()],
        if terminal {
            crate::solver::ProgressValue::known(100.0)?
        } else {
            crate::solver::ProgressValue::indeterminate()
        },
        if terminal {
            crate::solver::ProgressValue::known(100.0)?
        } else {
            crate::solver::ProgressValue::indeterminate()
        },
        Duration::ZERO,
        objective,
        None,
        None,
        terminal,
    )?;
    reporter(&snapshot)
}

struct CartesianProduct {
    domains: Vec<Vec<i64>>,
    indices: Vec<usize>,
    first: bool,
    exhausted: bool,
}

impl CartesianProduct {
    fn new(domains: Vec<Vec<i64>>) -> Self {
        Self {
            indices: vec![0; domains.len()],
            domains,
            first: true,
            exhausted: false,
        }
    }

    fn is_exhausted(&self) -> bool {
        self.exhausted
    }
}

impl Iterator for CartesianProduct {
    type Item = Vec<i64>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }
        if self.domains.is_empty() {
            self.exhausted = true;
            return Some(Vec::new());
        }
        if self.first {
            self.first = false;
        } else {
            let mut advanced = false;
            for index in (0..self.indices.len()).rev() {
                if self.indices[index] + 1 < self.domains[index].len() {
                    self.indices[index] += 1;
                    for reset in index + 1..self.indices.len() {
                        self.indices[reset] = 0;
                    }
                    advanced = true;
                    break;
                }
            }
            if !advanced {
                self.exhausted = true;
                return None;
            }
        }
        Some(
            self.indices
                .iter()
                .enumerate()
                .map(|(index, position)| self.domains[index][*position])
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::constraint_programming::IntegerDomain;
    use crate::model::constraint_programming::{
        ConstraintDefinition, ConstraintProgrammingConstraint, IntegerExpression, IntegerRelation,
    };
    use crate::solver::{
        CancellationOrigin, InfeasibilityEvidenceSource, ProofCompleteness, ProofReliability,
        ProofStatus, SolveProgressReporter,
    };
    use std::sync::{Arc, Mutex};

    fn model() -> ConstraintProgrammingSnapshot {
        let x = IntegerVariable::new("x");
        let y = IntegerVariable::new("y");
        let mut model =
            crate::model::constraint_programming::ConstraintProgrammingModel::new("fake");
        model
            .register_variable(x.clone(), IntegerDomain::range(0, 2).expect("domain"))
            .expect("register x");
        model
            .register_variable(y.clone(), IntegerDomain::boolean())
            .expect("register y");
        model
            .add_constraint(ConstraintDefinition::new(
                "x-lower",
                ConstraintProgrammingConstraint::integer(
                    IntegerExpression::variable(x),
                    IntegerRelation::GreaterOrEqual,
                    1,
                ),
            ))
            .expect("constraint");
        model.freeze().expect("snapshot")
    }

    #[test]
    fn fake_solver_returns_verified_optimal_report() {
        let snapshot = model();
        let report = FakeConstraintProgrammingSolver::new()
            .solve_constraint_programming(&snapshot, &ConstraintProgrammingSolveOptions::new())
            .expect("solve");
        assert_eq!(report.problem_status, ProblemStatus::Feasible);
        assert_eq!(
            report.solution_presence,
            crate::solver::SolutionPresence::Incumbent
        );
        assert_eq!(report.proof, None);
        assert_eq!(
            report
                .solution
                .as_ref()
                .map(|solution| solution.values.len()),
            Some(2)
        );
        report.validate().expect("report is valid");
    }

    #[test]
    fn fake_solver_uses_a_valid_solution_hint_as_the_first_search_candidate() {
        let snapshot = model();
        let hint = BTreeMap::from([(StableVariableId::from("x"), 2_i64)]);
        let options = ConstraintProgrammingSolveOptions::builder()
            .solution_hint(Some(&hint))
            .finish();
        let report = FakeConstraintProgrammingSolver::new()
            .solve_constraint_programming(&snapshot, &options)
            .expect("solve with hint");
        assert_eq!(
            report
                .solution
                .as_ref()
                .and_then(|solution| solution.stable_values.get(&StableVariableId::from("x")))
                .copied(),
            Some(2)
        );
        assert_eq!(
            report
                .diagnostics
                .extensions
                .get("cp.enumeratedAssignments")
                .map(String::as_str),
            Some("1")
        );
    }

    #[test]
    fn fake_solver_rejects_a_complete_hint_that_violates_a_constraint() {
        let snapshot = model();
        let hint = BTreeMap::from([
            (StableVariableId::from("x"), 0_i64),
            (StableVariableId::from("y"), 0_i64),
        ]);
        let options = ConstraintProgrammingSolveOptions::builder()
            .time_limit(Some(Duration::ZERO))
            .solution_hint(Some(&hint))
            .finish();
        let error = FakeConstraintProgrammingSolver::new()
            .solve_constraint_programming(&snapshot, &options)
            .expect_err("an invalid complete hint must not be silently discarded");
        assert!(error.to_string().contains("complete CP solution hint"));
    }

    #[test]
    fn fake_solver_rejects_a_complete_hint_that_violates_an_active_assumption() {
        let snapshot = model();
        let x = snapshot
            .variable(&StableVariableId::from("x"))
            .expect("x")
            .variable
            .clone();
        let hint = BTreeMap::from([
            (StableVariableId::from("x"), 1_i64),
            (StableVariableId::from("y"), 0_i64),
        ]);
        let assumption = ConstraintProgrammingAssumption::Equal(x, 2);
        let options = ConstraintProgrammingSolveOptions::builder()
            .time_limit(Some(Duration::ZERO))
            .solution_hint(Some(&hint))
            .finish();
        let error = FakeConstraintProgrammingSolver::new()
            .solve_constraint_programming_with_assumptions_and_conflict(
                &snapshot,
                &[assumption],
                &options,
            )
            .expect_err("a complete hint must satisfy active assumptions");
        assert!(
            error
                .to_string()
                .contains("violates the active assumptions")
        );
    }

    #[test]
    fn fake_solver_honors_cancellation_requested_by_progress_callback() {
        let snapshot = model();
        let handle = SolveHandle::new();
        let callback_handle = handle.clone();
        let snapshots = Arc::new(Mutex::new(Vec::new()));
        let callback_snapshots = Arc::clone(&snapshots);
        let reporter: SolveProgressReporter = Arc::new(move |snapshot| {
            callback_snapshots
                .lock()
                .expect("progress mutex")
                .push(snapshot.clone());
            if snapshot.stage == SolveStage::Solving {
                callback_handle.cancel(CancellationOrigin::Callback);
            }
            Ok(())
        });
        let options = ConstraintProgrammingSolveOptions::builder()
            .cancellation_handle(Some(&handle))
            .progress_reporter(Some(&reporter))
            .finish();
        let report = FakeConstraintProgrammingSolver::new()
            .solve_constraint_programming(&snapshot, &options)
            .expect("callback cancellation should be represented by a report");
        assert_eq!(report.termination_reason, TerminationReason::Cancelled);
        assert_eq!(report.statistics.nodes, Some(0));
        assert_eq!(report.proof, None);
        assert!(!snapshots.lock().expect("progress mutex").is_empty());
    }

    #[test]
    fn fake_solver_reports_directional_bounds_for_minimum_and_maximum_limits() {
        let x = IntegerVariable::new("x");
        let mut minimum =
            crate::model::constraint_programming::ConstraintProgrammingModel::new("minimum");
        minimum
            .register_variable(x.clone(), IntegerDomain::range(0, 10).expect("domain"))
            .expect("variable");
        minimum.set_objective(
            crate::model::constraint_programming::IntegerObjective::minimize(
                IntegerExpression::variable(x.clone()),
            ),
        );
        let minimum = minimum.freeze().expect("minimum snapshot");
        let limited = ConstraintProgrammingSolveOptions::builder()
            .node_limit(Some(0))
            .finish();
        let minimum_report = FakeConstraintProgrammingSolver::new()
            .solve_constraint_programming(&minimum, &limited)
            .expect("minimum report");
        assert_eq!(minimum_report.statistics.best_bound, Some(0));

        let mut maximum =
            crate::model::constraint_programming::ConstraintProgrammingModel::new("maximum");
        maximum
            .register_variable(x.clone(), IntegerDomain::range(0, 10).expect("domain"))
            .expect("variable");
        maximum.set_objective(
            crate::model::constraint_programming::IntegerObjective::maximize(
                IntegerExpression::variable(x),
            ),
        );
        let maximum = maximum.freeze().expect("maximum snapshot");
        let maximum_report = FakeConstraintProgrammingSolver::new()
            .solve_constraint_programming(&maximum, &limited)
            .expect("maximum report");
        assert_eq!(maximum_report.statistics.best_bound, Some(10));
    }

    #[test]
    fn fake_solver_does_not_round_large_integer_objectives_into_progress_or_float_stats() {
        let value = 9_007_199_254_740_993_i64;
        let x = IntegerVariable::new("large");
        let mut model =
            crate::model::constraint_programming::ConstraintProgrammingModel::new("large");
        model
            .register_variable(x.clone(), IntegerDomain::values([value]).expect("domain"))
            .expect("variable");
        model.set_objective(
            crate::model::constraint_programming::IntegerObjective::maximize(
                IntegerExpression::variable(x),
            ),
        );
        let snapshot = model.freeze().expect("snapshot");
        let progress = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&progress);
        let reporter: SolveProgressReporter = Arc::new(move |snapshot| {
            captured
                .lock()
                .expect("progress mutex")
                .push(snapshot.clone());
            Ok(())
        });
        let options = ConstraintProgrammingSolveOptions::builder()
            .progress_reporter(Some(&reporter))
            .finish();
        let report = FakeConstraintProgrammingSolver::new()
            .solve_constraint_programming(&snapshot, &options)
            .expect("large objective report");
        assert_eq!(
            report.solution.as_ref().and_then(|s| s.objective),
            Some(value)
        );
        assert_eq!(report.statistics.best_bound, Some(value));
        assert!(report.statistics.best_bound_value.is_none());
        assert!(report.statistics.absolute_gap.is_none());
        assert!(report.statistics.relative_gap.is_none());
        assert!(
            progress
                .lock()
                .expect("progress mutex")
                .iter()
                .all(|snapshot| snapshot.objective_value.is_none())
        );
    }

    #[test]
    fn fake_solver_reports_infeasible_after_complete_enumeration() {
        let x = IntegerVariable::new("x");
        let mut model =
            crate::model::constraint_programming::ConstraintProgrammingModel::new("infeasible");
        model
            .register_variable(x.clone(), IntegerDomain::boolean())
            .expect("variable");
        model
            .add_constraint_by_id(
                "impossible",
                ConstraintProgrammingConstraint::integer(
                    IntegerExpression::variable(x),
                    IntegerRelation::Equal,
                    2,
                ),
            )
            .expect("constraint");
        let snapshot = model.freeze().expect("snapshot");
        let report = FakeConstraintProgrammingSolver::new()
            .solve_constraint_programming(&snapshot, &ConstraintProgrammingSolveOptions::new())
            .expect("solve");
        assert_eq!(report.problem_status, ProblemStatus::Infeasible);
        assert_eq!(report.solution, None);
        assert_eq!(
            report.proof.as_ref().map(|proof| proof.status),
            Some(ProofStatus::Verified)
        );
    }

    #[test]
    fn cancellation_does_not_produce_a_fake_infeasibility_proof() {
        let snapshot = model();
        let handle = SolveHandle::new();
        handle.cancel(CancellationOrigin::User);
        let options = ConstraintProgrammingSolveOptions::builder()
            .cancellation_handle(Some(&handle))
            .finish();
        let report = FakeConstraintProgrammingSolver::new()
            .solve_constraint_programming(&snapshot, &options)
            .expect("solve");
        assert_eq!(report.termination_reason, TerminationReason::Cancelled);
        assert_eq!(report.proof, None);
        assert_eq!(report.problem_status, ProblemStatus::Unknown);
    }

    #[test]
    fn rebuild_session_applies_assumptions() {
        let snapshot = model();
        let x = snapshot
            .variable(&StableVariableId::from("x"))
            .expect("x")
            .variable
            .clone();
        let mut session = FakeConstraintProgrammingSolver::new()
            .create_session(&snapshot)
            .expect("session");
        let assumptions = [ConstraintProgrammingAssumption::Equal(x, 2)];
        let report = session
            .solve_with_assumptions(&assumptions, &ConstraintProgrammingSolveOptions::new())
            .expect("solve");
        assert_eq!(report.problem_status, ProblemStatus::Feasible);
        assert_eq!(
            report
                .solution
                .as_ref()
                .and_then(|solution| solution.objective),
            None
        );
        assert_eq!(
            report
                .solution
                .as_ref()
                .and_then(|solution| solution.stable_values.get(&StableVariableId::from("x")))
                .copied(),
            Some(2)
        );
    }

    #[test]
    fn assumption_ids_are_stable_and_duplicate_assumptions_are_rejected() {
        let snapshot = model();
        let x = snapshot
            .variable(&StableVariableId::from("x"))
            .expect("x")
            .variable
            .clone();
        let first = ConstraintProgrammingAssumption::Equal(x.clone(), 1);
        let second = ConstraintProgrammingAssumption::Equal(x, 1);
        assert_eq!(first.stable_id(), second.stable_id());
        let options = ConstraintProgrammingSolveOptions::new();
        let error = FakeConstraintProgrammingSolver::new()
            .solve_constraint_programming_with_assumptions_and_conflict(
                &snapshot,
                &[first, second],
                &options,
            )
            .expect_err("duplicate assumptions");
        assert!(
            error
                .to_string()
                .contains("duplicate CP assumption identity")
        );
    }

    #[test]
    fn verified_cp_conflict_is_shrunk_deterministically() {
        let x = IntegerVariable::new("x");
        let mut model =
            crate::model::constraint_programming::ConstraintProgrammingModel::new("conflict");
        model
            .register_variable(x.clone(), IntegerDomain::range(0, 1).expect("domain"))
            .expect("variable");
        let snapshot = model.freeze().expect("snapshot");
        let assumptions = [
            ConstraintProgrammingAssumption::Equal(x.clone(), 0),
            ConstraintProgrammingAssumption::Equal(x, 1),
        ];
        let options = ConstraintProgrammingSolveOptions::builder()
            .request_conflict(true)
            .shrink_conflict(true)
            .finish();
        let report = FakeConstraintProgrammingSolver::new()
            .solve_constraint_programming_with_assumptions_and_conflict(
                &snapshot,
                &assumptions,
                &options,
            )
            .expect("conflict solve");
        assert_eq!(report.problem_status, ProblemStatus::Infeasible);
        let evidence = report
            .diagnostics
            .infeasibility_evidence
            .as_ref()
            .expect("conflict evidence");
        assert_eq!(
            evidence.source,
            crate::solver::InfeasibilityEvidenceSource::RebuildVerification
        );
        assert_eq!(evidence.reliability, ProofReliability::Exact);
        assert_eq!(
            evidence.minimality,
            crate::solver::InfeasibilityMinimality::Irreducible
        );
        assert_eq!(evidence.members.len(), 2);
        report.validate().expect("report is valid");
    }

    #[test]
    fn conflict_shrinking_budget_preserves_verified_partial_seed() {
        let x = IntegerVariable::new("x");
        let mut model =
            crate::model::constraint_programming::ConstraintProgrammingModel::new("conflict");
        model
            .register_variable(x.clone(), IntegerDomain::range(0, 1).expect("domain"))
            .expect("variable");
        let snapshot = model.freeze().expect("snapshot");
        let assumptions = [
            ConstraintProgrammingAssumption::Equal(x.clone(), 0),
            ConstraintProgrammingAssumption::Equal(x, 1),
        ];
        let options = ConstraintProgrammingSolveOptions::builder()
            .request_conflict(true)
            .shrink_conflict(true)
            .max_conflict_resolves(1)
            .finish();
        let report = FakeConstraintProgrammingSolver::new()
            .solve_constraint_programming_with_assumptions_and_conflict(
                &snapshot,
                &assumptions,
                &options,
            )
            .expect("conflict solve");
        let evidence = report
            .diagnostics
            .infeasibility_evidence
            .as_ref()
            .expect("conflict evidence");
        assert_eq!(
            evidence.minimality,
            crate::solver::InfeasibilityMinimality::Partial
        );
        assert_eq!(evidence.completeness, ProofCompleteness::Partial);
        assert_eq!(evidence.members.len(), 2);
        assert!(evidence.unavailable_reason.is_some());
        assert_eq!(report.problem_status, ProblemStatus::Infeasible);
    }

    #[test]
    fn conflict_request_does_not_promote_incomplete_solve() {
        let snapshot = model();
        let options = ConstraintProgrammingSolveOptions::builder()
            .request_conflict(true)
            .node_limit(Some(1))
            .finish();
        let report = FakeConstraintProgrammingSolver::new()
            .solve_constraint_programming_with_conflict(&snapshot, &options)
            .expect("limited solve");
        assert_eq!(report.problem_status, ProblemStatus::Unknown);
        assert_eq!(report.proof, None);
        let evidence = report
            .diagnostics
            .infeasibility_evidence
            .as_ref()
            .expect("unavailable conflict evidence");
        assert_eq!(evidence.source, InfeasibilityEvidenceSource::Unavailable);
        assert!(
            report
                .diagnostics
                .issues
                .iter()
                .any(|issue| issue.code == "CpConflictDiagnosticsUnavailable")
        );
    }
}
