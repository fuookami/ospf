//! 独立求解只编排 context 注册、后端调用与报告投影。 / Standalone solves only orchestrate context registration, backend invocation and report projection.

use ospf_rust_core::model::MetaModel;
use ospf_rust_core::solver::audit::evaluate_linear_solution;
use ospf_rust_core::solver::value::SolveValue;
use ospf_rust_core::solver::{
    LinearSolver, ProofReliability, ProofStatus, SolveDiagnostics, SolveOptions, SolveProof,
    SolveReport, SolveSolution, SolveStatistics, SolveWarning, linear_model_fingerprint,
};
use ospf_rust_quantities::unit::UnitConversionValue;
use crate::domain::allocation::{AllocationContext, AllocationProblem, AllocationSolution};
use crate::domain::design::{NetworkDesignContext, NetworkDesignProblem, NetworkDesignSolution};
use crate::domain::flow::{FlowContext, FlowProblem, FlowSolution};
use crate::domain::generalized_flow::{
    GeneralizedFlowContext, GeneralizedFlowProblem, GeneralizedFlowSolution,
};
use crate::domain::path_compilation::{PathCompilationContext, PathCompilationProblem, PathCompilationSolution};
use crate::domain::resilience::{NetworkResilienceContext, NetworkResilienceProblem, NetworkResilienceSolution};
use crate::domain::structure::{NetworkStructureContext, NetworkStructureProblem, NetworkStructureSolution};
use crate::error::{NetworkSchedulingError, Result};

/// 独立与嵌入式使用相同领域注册和提取。 / Standalone and embedded use share domain registration and extraction.
pub trait NetworkApplicationContext {
    /// 经过生产校验的领域解 / Production-validated domain solution.
    type Solution;

    /// 委托领域 context 注册 / Delegate registration to the domain context.
    fn register(&mut self, model: &mut MetaModel<f64>) -> Result<()>;

    /// 委托领域 context 独立提取及校验 / Delegate independent extraction and validation to the domain context.
    fn extract(&mut self, model: &MetaModel<f64>) -> Result<Self::Solution>;
}

impl<V> NetworkApplicationContext for FlowContext<V>
where
    V: SolveValue + UnitConversionValue,
{
    type Solution = FlowSolution<V>;

    fn register(&mut self, model: &mut MetaModel<f64>) -> Result<()> { self.register(model) }

    fn extract(&mut self, model: &MetaModel<f64>) -> Result<Self::Solution> { self.extract_solution(model) }
}

impl<V> NetworkApplicationContext for GeneralizedFlowContext<V>
where
    V: SolveValue + UnitConversionValue,
{
    type Solution = GeneralizedFlowSolution<V>;

    fn register(&mut self, model: &mut MetaModel<f64>) -> Result<()> { self.register(model, Default::default()) }

    fn extract(&mut self, model: &MetaModel<f64>) -> Result<Self::Solution> { self.extract_solution(model) }
}

impl<V: SolveValue + UnitConversionValue> NetworkApplicationContext for NetworkDesignContext<V> {
    type Solution = NetworkDesignSolution<V>;

    fn register(&mut self, model: &mut MetaModel<f64>) -> Result<()> { self.register(model, Default::default()) }

    fn extract(&mut self, model: &MetaModel<f64>) -> Result<Self::Solution> { self.extract_solution(model) }
}

impl<V: SolveValue + UnitConversionValue> NetworkApplicationContext for NetworkResilienceContext<V> {
    type Solution = NetworkResilienceSolution<V>;

    fn register(&mut self, model: &mut MetaModel<f64>) -> Result<()> { self.register(model) }

    fn extract(&mut self, model: &MetaModel<f64>) -> Result<Self::Solution> { self.extract_solution(model) }
}

impl<V: SolveValue + UnitConversionValue> NetworkApplicationContext for NetworkStructureContext<V> {
    type Solution = NetworkStructureSolution<V>;

    fn register(&mut self, model: &mut MetaModel<f64>) -> Result<()> { self.register(model, Default::default()) }

    fn extract(&mut self, model: &MetaModel<f64>) -> Result<Self::Solution> { self.extract_solution(model) }
}

impl<V: SolveValue + UnitConversionValue> NetworkApplicationContext for AllocationContext<V> {
    type Solution = AllocationSolution<V>;

    fn register(&mut self, model: &mut MetaModel<f64>) -> Result<()> { self.register(model, Default::default()) }

    fn extract(&mut self, model: &MetaModel<f64>) -> Result<Self::Solution> { self.extract_solution(model) }
}

impl<V, C> NetworkApplicationContext for PathCompilationContext<V, C>
where
    V: SolveValue + UnitConversionValue,
    C: Clone + PartialEq,
{
    type Solution = PathCompilationSolution<V, C>;

    fn register(&mut self, model: &mut MetaModel<f64>) -> Result<()> { self.register(model) }

    fn extract(&mut self, model: &MetaModel<f64>) -> Result<Self::Solution> { self.extract_solution(model) }
}

/// 无建模细节的统一独立求解编排。 / Unified standalone orchestration without domain-model details.
pub struct NetworkApplicationService<S> {
    solver: S,
}

impl<S: LinearSolver> NetworkApplicationService<S> {
    /// 注入后端 / Inject a backend.
    pub fn new(solver: S) -> Self { Self { solver } }

    /// 每次从工厂创建独立 context；正常无解终态保留报告。 / Create a fresh context from the factory on every call; preserve normal no-incumbent outcomes as reports.
    pub fn solve<C, F>(
        &self,
        factory: F,
        options: &SolveOptions<'_>,
    ) -> Result<SolveReport<C::Solution>>
    where
        C: NetworkApplicationContext,
        F: FnOnce() -> Result<C>,
    {
        let mut context = factory()?;
        let mut model = MetaModel::new("network-application");
        context.register(&mut model)?;
        let compiled = model.try_to_linear_triad_model()
            .map_err(|error| NetworkSchedulingError::model(error.to_string()))?;
        let report = self.solver.solve_linear_report_with_options(&compiled, options)
            .map_err(|error| NetworkSchedulingError::solver(error.to_string()))?;
        report.validate().map_err(|error| NetworkSchedulingError::contract(error.to_string()))?;
        let expected = linear_model_fingerprint(&compiled)
            .map_err(|error| NetworkSchedulingError::contract(error.to_string()))?;
        let verified = report.proof.as_ref().is_some_and(|proof| proof.status == ProofStatus::Verified
            && matches!(proof.reliability, ProofReliability::Exact | ProofReliability::Reliable));
        if report.fingerprints.model.as_ref().is_some_and(|fingerprint| fingerprint != &expected)
            || verified && report.fingerprints.model.is_none()
        {
            return Err(NetworkSchedulingError::contract("solver 报告模型身份不匹配 / solver report model identity mismatch"));
        }
        let domain = if let Some(solution) = &report.solution {
            let audit = evaluate_linear_solution(&compiled, &solution.values, 1e-7)
                .map_err(|error| NetworkSchedulingError::contract(error.to_string()))?;
            if !audit.issues.is_empty() || compiled.var_types.iter().zip(&solution.values)
                .any(|(kind, value)| kind.is_integer() && (value - value.round()).abs() > 1e-7)
            {
                return Err(NetworkSchedulingError::contract("solver incumbent 未通过独立模型复核 / solver incumbent failed independent model validation"));
            }
            let actual: f64 = compiled.c.iter().zip(&solution.values).map(|(coefficient, value)| coefficient * value).sum();
            let declared = solution.objective_value.or(solution.objective);
            if !actual.is_finite() || declared.is_some_and(|value|
                !value.is_finite() || (actual - value).abs() > 1e-7 * (1.0 + actual.abs().max(value.abs())))
            {
                return Err(NetworkSchedulingError::contract("solver 目标结果不一致 / inconsistent solver objective result"));
            }
            model.set_solution(&solution.values);
            Some((context.extract(&model)?, actual))
        } else { None };
        project(report, domain)
    }
}

fn project<D>(report: SolveReport<f64>, domain: Option<(D, f64)>) -> Result<SolveReport<D>> {
    let statistics = SolveStatistics {
        solve_time: report.statistics.solve_time, iterations: report.statistics.iterations,
        nodes: report.statistics.nodes, best_bound: None,
        best_bound_value: report.statistics.best_bound_value.or(report.statistics.best_bound),
        absolute_gap: report.statistics.absolute_gap, relative_gap: report.statistics.relative_gap,
        solution_count: report.statistics.solution_count, extensions: report.statistics.extensions,
    };
    let mut diagnostics = SolveDiagnostics {
        infeasibility_evidence: report.diagnostics.infeasibility_evidence,
        warnings: report.diagnostics.warnings, issues: report.diagnostics.issues,
        extensions: report.diagnostics.extensions, ..Default::default()
    };
    // 领域 payload 不是数值向量；原始数值审计保留在扩展中，不伪造领域对偶。 / A domain payload is not a numeric vector; preserve numeric audit in extensions rather than inventing domain duals.
    if !report.diagnostics.constraint_evaluations.is_empty() || !report.diagnostics.variable_bound_evaluations.is_empty()
        || report.proof.as_ref().is_some_and(|proof| proof.evidence.is_some())
        || report.solution.as_ref().is_some_and(|solution| !solution.pool.is_empty())
    {
        diagnostics.extensions.insert("solver.constraintEvaluations".into(), format!("{:?}", report.diagnostics.constraint_evaluations));
        diagnostics.extensions.insert("solver.variableBoundEvaluations".into(), format!("{:?}", report.diagnostics.variable_bound_evaluations));
        diagnostics.extensions.insert("solver.proofEvidence".into(), format!("{:?}", report.proof.as_ref().and_then(|proof| proof.evidence.as_ref())));
        diagnostics.extensions.insert("solver.solutionPool".into(), format!("{:?}", report.solution.as_ref().map(|solution| &solution.pool)));
        diagnostics.warnings.push(SolveWarning::new(
            "NumericAuditProjected", "数值审计在 solver.* 扩展中，领域报告不提供定价对偶 / numeric audit is stored in solver.* extensions; domain reports do not provide pricing duals",
        ));
    }
    let mut builder = SolveReport::builder(report.problem_status, report.termination_reason)
        .statistics(statistics).diagnostics(diagnostics).trace(report.trace)
        .provenance(report.provenance).fingerprints(report.fingerprints).model_mapping(report.model_mapping);
    if let Some((domain, objective)) = domain {
        let mut solution = SolveSolution::incumbent(domain);
        solution.objective_value = Some(objective);
        builder = builder.solution(solution);
    }
    if let Some(proof) = report.proof {
        builder = builder.proof(SolveProof {
            kind: proof.kind, status: proof.status, reliability: proof.reliability,
            completeness: proof.completeness, reference: proof.reference, evidence: None,
        });
    }
    for warning in report.warnings { builder = builder.warning(warning); }
    let mut projected = builder.build().map_err(|error| NetworkSchedulingError::contract(error.to_string()))?;
    projected.schema_version = report.schema_version;
    Ok(projected)
}

/// 最小费用 Flow 独立入口。 / Standalone min-cost Flow entry point.
pub struct FlowApplicationService<S> {
    application: NetworkApplicationService<S>,
}

impl<S: LinearSolver> FlowApplicationService<S> {
    /// 注入后端 / Inject a backend.
    pub fn new(solver: S) -> Self { Self { application: NetworkApplicationService::new(solver) } }

    /// 从不可变问题创建新 context 并返回统一领域报告。 / Create a fresh context from an immutable problem and return a unified domain report.
    pub fn solve<V>(&self, problem: &FlowProblem<V>) -> Result<SolveReport<FlowSolution<V>>>
    where
        V: SolveValue + UnitConversionValue,
    {
        self.application.solve(|| Ok(FlowContext::new(problem.clone())), &SolveOptions::default())
    }
}

/// 广义流独立入口。 / Standalone generalized-flow entry point.
pub struct GeneralizedFlowApplicationService<S> {
    application: NetworkApplicationService<S>,
}

impl<S: LinearSolver> GeneralizedFlowApplicationService<S> {
    /// 注入后端 / Inject a backend.
    pub fn new(solver: S) -> Self { Self { application: NetworkApplicationService::new(solver) } }

    /// 每次注册独立 context；非法结果不能形成领域 incumbent。 / Register a fresh context on every call; invalid results cannot become domain incumbents.
    pub fn solve<V>(&self, problem: &GeneralizedFlowProblem<V>) -> Result<SolveReport<GeneralizedFlowSolution<V>>>
    where
        V: SolveValue + UnitConversionValue,
    {
        self.application.solve(|| GeneralizedFlowContext::new(problem.clone(), "generalized-flow"), &SolveOptions::default())
    }
}

/// 建设领域独立入口，与嵌入式共用全部注册路径。 / Standalone design entry point sharing all registration paths with embedded use.
pub struct NetworkDesignApplicationService<S> {
    application: NetworkApplicationService<S>,
}

impl<S: LinearSolver> NetworkDesignApplicationService<S> {
    /// 注入后端 / Inject a backend.
    pub fn new(solver: S) -> Self { Self { application: NetworkApplicationService::new(solver) } }

    /// 每次使用新 context 并返回统一领域报告。 / Use a fresh context on every call and return a unified domain report.
    pub fn solve<V>(&self, problem: &NetworkDesignProblem<V>) -> Result<SolveReport<NetworkDesignSolution<V>>>
    where
        V: SolveValue + UnitConversionValue,
    {
        self.application.solve(|| NetworkDesignContext::new(problem.clone(), "network-design"), &SolveOptions::default())
    }
}

/// 显式韧性领域独立入口。 / Standalone explicit-resilience entry point.
pub struct NetworkResilienceApplicationService<S> {
    application: NetworkApplicationService<S>,
}

impl<S: LinearSolver> NetworkResilienceApplicationService<S> {
    /// 注入后端 / Inject a backend.
    pub fn new(solver: S) -> Self { Self { application: NetworkApplicationService::new(solver) } }

    /// 注册独立场景 context；不外推显式场景之外的保障。 / Register independent scenario contexts without extrapolating guarantees beyond explicit scenarios.
    pub fn solve<V>(&self, problem: &NetworkResilienceProblem<V>) -> Result<SolveReport<NetworkResilienceSolution<V>>>
    where
        V: SolveValue + UnitConversionValue,
    {
        self.application.solve(|| NetworkResilienceContext::new(problem.clone(), "network-resilience"), &SolveOptions::default())
    }
}

/// 纯结构领域独立入口，不启动隐式名义流求解。 / Standalone structure entry point without implicit nominal-flow solves.
pub struct NetworkStructureApplicationService<S> {
    application: NetworkApplicationService<S>,
}

impl<S: LinearSolver> NetworkStructureApplicationService<S> {
    /// 注入后端 / Inject a backend.
    pub fn new(solver: S) -> Self { Self { application: NetworkApplicationService::new(solver) } }

    /// 每次创建独立 context；合法断联等不可行状态保留统一报告。 / Create a fresh context on every call; preserve infeasibility such as valid disconnection in unified reports.
    pub fn solve<V>(&self, problem: &NetworkStructureProblem<V>) -> Result<SolveReport<NetworkStructureSolution<V>>>
    where
        V: SolveValue + UnitConversionValue,
    {
        self.application.solve(|| NetworkStructureContext::new(problem.clone(), "network-structure"), &SolveOptions::default())
    }
}

/// 运输分配独立入口，复用同一 Flow/Design 注册路径。 / Standalone allocation entry point reusing the same Flow/Design registration path.
pub struct AllocationApplicationService<S> {
    application: NetworkApplicationService<S>,
}

impl<S: LinearSolver> AllocationApplicationService<S> {
    /// 注入后端 / Inject a backend.
    pub fn new(solver: S) -> Self { Self { application: NetworkApplicationService::new(solver) } }

    /// 每次创建独立 context，并保留真实求解终态。 / Create a fresh context on every call and preserve actual solve outcomes.
    pub fn solve<V>(&self, problem: &AllocationProblem<V>) -> Result<SolveReport<AllocationSolution<V>>>
    where
        V: SolveValue + UnitConversionValue,
    {
        self.application.solve(|| AllocationContext::new(problem.clone(), "allocation"), &SolveOptions::default())
    }
}

/// 完整路径有限主问题独立入口，最优性只覆盖返回的 candidate_ids。 / Standalone whole-path finite-master entry point; optimality covers only returned candidate_ids.
pub struct PathCompilationApplicationService<S> {
    application: NetworkApplicationService<S>,
}

impl<S: LinearSolver> PathCompilationApplicationService<S> {
    /// 注入后端 / Inject a backend.
    pub fn new(solver: S) -> Self { Self { application: NetworkApplicationService::new(solver) } }

    /// 每次创建独立 context，保留全部业务载荷及统一求解终态。 / Create a fresh context each time, retaining complete business payloads and unified solve outcomes.
    pub fn solve<V, C>(&self, problem: &PathCompilationProblem<V, C>) -> Result<SolveReport<PathCompilationSolution<V, C>>>
    where
        V: SolveValue + UnitConversionValue,
        C: Clone + PartialEq,
    {
        self.application.solve(|| PathCompilationContext::new(problem.clone(), "path-compilation"), &SolveOptions::default())
    }
}

#[cfg(test)]
mod tests;
