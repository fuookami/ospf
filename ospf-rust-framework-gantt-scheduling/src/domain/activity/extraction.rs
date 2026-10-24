//! 完整编译模型审计、源变量映射与证据域隔离。 / Complete compiled-model audit, source-variable mapping and evidence-domain isolation.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use ospf_rust_quantities::unit::UnitConversionValue;
use ospf_rust_core::model::{MetaModel, Objective};
use ospf_rust_core::model::intermediate::LinearTriadModel;
use ospf_rust_core::solver::audit::linear_model_mapping;
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_core::solver::{ProblemStatus, ProofReliability, ProofStatus, SolutionPresence, SolveDiagnostics, SolveProof,
    SolveReport, SolveSolution, SolveStatistics, SolveTrace, SolveWarning, linear_model_fingerprint};
use ospf_rust_framework::solver::{ColumnGenerationSolver, FrameworkSolveOptions};
use crate::domain::bunch_compilation::validation::validate_bunch_incumbent;
use crate::GanttResult;
use super::aggregation::{lifecycle, model_error};
use super::context::ActivitySchedulingContext;
use super::model::ActivitySchedule;
use super::numeric::sum_activity_values;
use super::pipeline::ActivityDecision;

/// 私有构造的完整编译记录；源 token 与后端列通过 VariableId 关联。
/// Privately constructed complete compilation record; source tokens map to backend columns by VariableId.
#[derive(Debug)]
pub struct ActivityCompiledModel {
    owner: u64,
    model_identity: u64,
    source: String,
    compiled: LinearTriadModel,
    source_columns: BTreeMap<usize, usize>,
    offset: f64,
    objective: Objective<f64>,
    execution: Arc<AtomicBool>,
}

/// 只由本次受控后端调用创建的执行记录，不能从裸外部报告提升证明。
/// Execution record created only by the controlled backend invocation; raw external reports cannot promote proofs.
#[derive(Debug)]
pub struct ActivitySolveExecution {
    execution: Arc<AtomicBool>,
    report: SolveReport<f64>,
}

impl ActivitySolveExecution {
    /// 原始 SolverModel 报告的只读审计视图。 / Read-only audit view of the original SolverModel report.
    pub fn report(&self) -> &SolveReport<f64> { &self.report }
}

impl ActivityCompiledModel {
    /// 真正传给单次 MILP 的完整模型，包含扩展辅助列和展开行。
    /// Complete model passed to the single MILP solve, including extension auxiliaries and expanded rows.
    pub fn model(&self) -> &LinearTriadModel { &self.compiled }

    /// 已按全局方向和子目标权重归一化的声明目标常数；后端目标不含它。
    /// Declared objective constant normalized by global direction and sub-objective weights; excluded from backend objectives.
    pub fn objective_offset(&self) -> f64 { self.offset }

    /// 保留方向、权重和常数的完整声明目标，不替换后端 c^T x 报告口径。
    /// Complete declared objective retaining directions, weights and constants, without replacing backend c^T x report semantics.
    pub fn declared_objective(&self) -> &Objective<f64> { &self.objective }

    /// 源 token 索引到后端列；不得把整个后端向量回填源 token 表。
    /// Source-token index to backend column; never assign the complete backend vector to the source token table.
    pub fn source_columns(&self) -> &BTreeMap<usize, usize> { &self.source_columns }

    fn begin_execution(&self) -> GanttResult<()> {
        self.execution.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| lifecycle("activity compilation already consumed its single solve attempt; compile again to retry"))?;
        Ok(())
    }

    /// 在受控记录的完整矩阵上恰调用一次 MILP；失败后须重新编译再尝试。
    /// Invoke MILP exactly once on the recorded complete matrix; recompile before retrying a failed execution.
    #[cfg(not(feature = "async"))]
    pub fn solve<S: ColumnGenerationSolver>(&self, solver: &S, options: FrameworkSolveOptions) -> GanttResult<ActivitySolveExecution> {
        self.begin_execution()?;
        let report = solver.solve_milp_report_with_options(&self.compiled, options).map_err(model_error)?;
        Ok(ActivitySolveExecution { execution: self.execution.clone(), report })
    }

    /// 异步受控单次 MILP，与同步入口使用相同执行归属合同。
    /// Asynchronous controlled single MILP, sharing execution ownership with the synchronous entry.
    #[cfg(feature = "async")]
    pub async fn solve<S: ColumnGenerationSolver>(&self, solver: &S, options: FrameworkSolveOptions) -> GanttResult<ActivitySolveExecution> {
        self.begin_execution()?;
        let report = solver.solve_milp_report_with_options(&self.compiled, options).await.map_err(model_error)?;
        Ok(ActivitySolveExecution { execution: self.execution.clone(), report })
    }

    #[cfg(test)]
    pub(crate) fn fixture_execution(&self, report: SolveReport<f64>) -> ActivitySolveExecution {
        ActivitySolveExecution { execution: self.execution.clone(), report }
    }
}

fn compiled_state(model: &LinearTriadModel) -> String {
    // fallback 辅助变量每次可有新 ID；只比较数学结构，源身份另行检查。
    // Fallback auxiliaries may receive fresh IDs; compare mathematical structure with source identity checked separately.
    format!("{:?}", ((&model.A, &model.b, &model.c, &model.lb, &model.ub, &model.var_types),
        &model.constraint_names, &model.constraint_group_ids, &model.constraint_lazy_flags,
        &model.constraint_priorities, &model.constraint_args, &model.constraint_source_symbol_ids, model.objective_category))
}

impl<V: SolveValue + UnitConversionValue> ActivitySchedulingContext<V> {
    /// 在全部下游扩展之后编译；记录不允许调用方替换或修改实际矩阵。
    /// Compile after all downstream extensions; callers cannot replace or mutate the recorded matrix.
    pub fn compile(&self, model: &MetaModel<f64>) -> GanttResult<ActivityCompiledModel> {
        self.aggregation.ensure_model(model)?;
        let source = model.linear_source_snapshot();
        let compiled = model.try_to_linear_triad_model().map_err(model_error)?;
        linear_model_fingerprint(&compiled).map_err(model_error)?;
        let source_columns = model.tokens().iter().enumerate().map(|(index, token)| {
            compiled.find_variable_index(token.id()).map(|column| (index, column))
                .ok_or_else(|| lifecycle("source variable is absent from the compiled model"))
        }).collect::<GanttResult<_>>()?;
        let objective = model.objective_snapshot();
        let offset = sum_activity_values(objective.sub_objectives.iter().map(|part| {
            let sign = if part.category == objective.category { 1.0 } else { -1.0 };
            sign * part.weight * part.polynomial.constant_term()
        }))?;
        Ok(ActivityCompiledModel { owner: self.owner, model_identity: model.model_identity(), source, compiled, source_columns, offset, objective,
            execution: Arc::new(AtomicBool::new(false)) })
    }

    /// 生产入口自身审计完整向量和所有辅助变量，再以原 V 复核选择、库存及资源。
    /// Production entry audits the complete vector and all auxiliaries, then validates choices, inventory and resources in native V.
    pub fn extract_solution(
        &self,
        model: &MetaModel<f64>,
        compilation: &ActivityCompiledModel,
        report: &SolveReport<f64>,
    ) -> GanttResult<Option<ActivitySchedule<V>>> {
        self.aggregation.ensure_model(model)?;
        if compilation.owner != self.owner || compilation.model_identity != model.model_identity()
            || compilation.source != model.linear_source_snapshot()
        { return Err(lifecycle("activity compilation belongs to another context or stale source model")); }
        let current = model.try_to_linear_triad_model().map_err(model_error)?;
        if compiled_state(&current) != compiled_state(&compilation.compiled) {
            return Err(lifecycle("compiled activity model changed since compilation"));
        }
        report.validate().map_err(model_error)?;
        let fingerprint = linear_model_fingerprint(&compilation.compiled).map_err(model_error)?;
        let verified = report.proof.as_ref().is_some_and(|proof| proof.status == ProofStatus::Verified
            && matches!(proof.reliability, ProofReliability::Exact | ProofReliability::Reliable));
        if report.fingerprints.model.as_ref().is_some_and(|actual| *actual != fingerprint)
            || verified && report.fingerprints.model.is_none()
        { return Err(lifecycle("activity report does not bind the actual complete compiled model")); }
        if let Some(mapping) = &report.model_mapping {
            let expected = linear_model_mapping(&compilation.compiled).map_err(model_error)?;
            if *mapping != expected {
                return Err(lifecycle("activity report mapping does not match the complete compiled model"));
            }
        }
        let Some(solution) = &report.solution else { return Ok(None); };
        validate_bunch_incumbent(&compilation.compiled, &solution.values, solution.objective_value)?;
        if let Some(objective) = solution.objective {
            validate_bunch_incumbent(&compilation.compiled, &solution.values, Some(objective))?;
        }
        let mut selected = Vec::new();
        let mut cancelled = BTreeSet::new();
        for (decision, index) in self.aggregation.compilation().variables() {
            let column = compilation.source_columns.get(index).ok_or_else(|| lifecycle("missing activity source-column mapping"))?;
            if solution.values[*column].round() == 1.0 {
                match decision { ActivityDecision::Execute(key) => selected.push(key.clone()), ActivityDecision::Cancel(task) => { cancelled.insert(task.clone()); } }
            }
        }
        self.aggregation.prepared().schedule(&selected, &cancelled).map(Some)
    }

    /// 报告后端目标/界/gap保持编译口径；领域费用独立，有损模型证据仅存审计扩展。
    /// Backend objective/bound/gap retain compiled semantics; domain costs are separate and lossy evidence is audit-only.
    pub fn extract_report(
        &self,
        model: &MetaModel<f64>,
        compilation: &ActivityCompiledModel,
        execution: ActivitySolveExecution,
    ) -> GanttResult<SolveReport<ActivitySchedule<V>>> {
        if !Arc::ptr_eq(&compilation.execution, &execution.execution) {
            return Err(lifecycle("activity report belongs to another compilation or solve execution"));
        }
        let report = execution.report;
        let domain = self.extract_solution(model, compilation, &report)?;
        let solver_only = self.aggregation.prepared().solver_model_only();
        let mut result = SolveReport::builder(ProblemStatus::Unknown, report.termination_reason).build().map_err(model_error)?;
        result.problem_status = if solver_only { if domain.is_some() { ProblemStatus::Feasible } else { ProblemStatus::Unknown } } else { report.problem_status };
        if let Some(domain) = domain {
            let mut solution = SolveSolution::incumbent(domain);
            solution.objective_value = (!solver_only).then(|| report.solution.as_ref().and_then(|value| value.objective_value.or(value.objective))).flatten();
            result.solution = Some(solution);
            result.solution_presence = if solver_only { SolutionPresence::Incumbent } else { report.solution_presence };
        }
        if !solver_only {
            result.proof = report.proof.as_ref().map(|proof| SolveProof { kind: proof.kind, status: proof.status, reliability: proof.reliability,
                completeness: proof.completeness, reference: proof.reference.clone(), evidence: None });
        }
        result.statistics = SolveStatistics {
            solve_time: report.statistics.solve_time, iterations: report.statistics.iterations, nodes: report.statistics.nodes,
            solution_count: if solver_only { result.solution.as_ref().map(|_| 1) } else { report.statistics.solution_count },
            ..SolveStatistics::default()
        };
        if !solver_only {
            result.statistics.best_bound_value = report.statistics.best_bound_value.or(report.statistics.best_bound);
            result.statistics.absolute_gap = report.statistics.absolute_gap;
            result.statistics.relative_gap = report.statistics.relative_gap;
            result.trace = report.trace.clone();
            result.fingerprints = report.fingerprints.clone();
            result.model_mapping = report.model_mapping.clone();
            result.statistics.extensions = report.statistics.extensions.clone();
            result.warnings = report.warnings.clone();
            // 数字型求值不能伪装成领域 payload；无损路径保留其余结构化诊断。
            // Numeric evaluations cannot become domain payloads; retain the other structured diagnostics on lossless paths.
            result.diagnostics = SolveDiagnostics {
                infeasibility_evidence: report.diagnostics.infeasibility_evidence.clone(),
                warnings: report.diagnostics.warnings.clone(), issues: report.diagnostics.issues.clone(),
                extensions: report.diagnostics.extensions.clone(), ..SolveDiagnostics::default()
            };
        } else {
            result.trace = SolveTrace { nodes_explored: report.trace.nodes_explored, nodes_pruned: report.trace.nodes_pruned,
                active_nodes: report.trace.active_nodes, total_iterations: report.trace.total_iterations, elapsed: report.trace.elapsed, ..SolveTrace::default() };
            result.warnings.push(SolveWarning::new("activity.solverModelOnly", "原域只确认可行 incumbent，编译模型证明不转移 / native domain confirms feasible incumbents only; compiled proofs do not transfer"));
        }
        result.provenance = report.provenance.clone();
        result.diagnostics.extensions.insert("activity.solverModelReport.debug".into(), format!("{report:?}"));
        result.diagnostics.extensions.insert("activity.objectiveOffset".into(), compilation.offset.to_string());
        result.diagnostics.extensions.insert("activity.declaredObjective".into(), format!("{:?}", compilation.declared_objective()));
        result.diagnostics.extensions.insert("activity.proofScope".into(), "finite declared candidate domain only; no continuous/global bunch optimality".into());
        result.validate().map_err(model_error)?;
        Ok(result)
    }
}
