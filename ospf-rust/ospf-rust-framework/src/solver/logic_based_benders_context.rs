//! Logic-Based Benders framework 上下文 / Logic-Based Benders framework context.
//!
//! 该模块只做 stable binding、engine、subproblem 和 cut pipeline 的装配，不重新定义求解
//! 终态。
//! This module only assembles stable bindings, the engine, subproblem, and cut pipelines; it does
//! not define a second solve-terminal hierarchy.

use ospf_rust_core::error::Result;
use ospf_rust_core::solver::AuditFingerprint;
use ospf_rust_core::solver::{SolveReport, StableVariableId};

use super::framework_solve_options::FrameworkSolveOptions;
use super::logic_based_benders::{
    ConstraintProgrammingSubproblemResult, ConstraintProgrammingSubproblemSolver,
    LogicBasedBendersCutOracle, LogicBasedBendersEngine, LogicBasedBendersMaster,
    LogicBasedBendersMode, MasterAssignment, MasterBinding,
};

/// 稳定绑定 pipeline / Stable binding pipeline.
#[derive(Debug, Clone)]
pub struct LogicBasedBendersBindingPipeline {
    /// master 绑定 / Master binding.
    pub binding: MasterBinding,
}

impl LogicBasedBendersBindingPipeline {
    /// 创建绑定 pipeline / Create a binding pipeline.
    pub fn new(binding: MasterBinding) -> Self {
        Self { binding }
    }

    /// 将 master 报告转换为稳定赋值 / Convert a master report to a stable assignment.
    pub fn assignment_from_report(
        &self,
        report: &SolveReport<f64>,
        integrality_tolerance: f64,
    ) -> Result<MasterAssignment> {
        MasterAssignment::from_report(report, &self.binding, integrality_tolerance)
    }
}

/// cut oracle pipeline 标记 / Cut-oracle pipeline marker.
pub trait LogicBasedBendersCutOraclePipeline: LogicBasedBendersCutOracle {}

impl<T> LogicBasedBendersCutOraclePipeline for T where T: LogicBasedBendersCutOracle {}

/// 稳定 ID 解提取器 / Stable-ID solution extractor.
#[derive(Debug, Clone)]
pub struct LogicBasedBendersSolutionExtractor {
    /// master 绑定 / Master binding.
    pub binding: MasterBinding,
}

impl LogicBasedBendersSolutionExtractor {
    /// 创建提取器 / Create an extractor.
    pub fn new(binding: MasterBinding) -> Self {
        Self { binding }
    }

    /// 提取稳定 master 赋值 / Extract the stable master assignment.
    pub fn extract_master_assignment(
        &self,
        report: &SolveReport<f64>,
        integrality_tolerance: f64,
    ) -> Result<MasterAssignment> {
        MasterAssignment::from_report(report, &self.binding, integrality_tolerance)
    }

    /// 按 ID 提取稳定值，不解析 backend 名称 / Extract one stable value without parsing backend names.
    pub fn stable_value(&self, report: &SolveReport<f64>, id: &StableVariableId) -> Result<f64> {
        report.validate()?;
        report
            .solution
            .as_ref()
            .and_then(|solution| solution.stable_values.get(id).copied())
            .ok_or_else(|| {
                ospf_rust_core::error::CoreError::contract_error(format!(
                    "solution is missing stable variable {}",
                    id.0
                ))
            })
    }
}

/// Logic-Based Benders 上下文 / Logic-Based Benders context.
pub struct LogicBasedBendersContext {
    /// 稳定绑定 pipeline / Stable binding pipeline.
    pub binding_pipeline: LogicBasedBendersBindingPipeline,
    /// 求解引擎 / Solver engine.
    pub engine: LogicBasedBendersEngine,
    /// 稳定 ID 解提取器 / Stable-ID solution extractor.
    pub solution_extractor: LogicBasedBendersSolutionExtractor,
}

impl LogicBasedBendersContext {
    /// 创建 Exact 上下文 / Create an exact context.
    pub fn new(binding: MasterBinding) -> Self {
        Self::with_mode(binding, LogicBasedBendersMode::Exact)
    }

    /// 按指定 proof mode 创建上下文 / Create a context with an explicit proof mode.
    pub fn with_mode(binding: MasterBinding, mode: LogicBasedBendersMode) -> Self {
        let binding_pipeline = LogicBasedBendersBindingPipeline::new(binding.clone());
        let solution_extractor = LogicBasedBendersSolutionExtractor::new(binding.clone());
        let engine = LogicBasedBendersEngine::new(binding).with_mode(mode);
        Self {
            binding_pipeline,
            engine,
            solution_extractor,
        }
    }

    /// 设置 Exact CP snapshot 预期身份 / Set the expected Exact CP snapshot identity.
    pub fn with_expected_cp_snapshot_fingerprint(mut self, fingerprint: AuditFingerprint) -> Self {
        self.engine = self
            .engine
            .with_expected_cp_snapshot_fingerprint(fingerprint);
        self
    }

    /// 求解已配置的 master 与 CP subproblem / Solve a configured master and CP subproblem.
    pub fn solve<M, S>(
        &self,
        master: &mut M,
        subproblem: &mut S,
        options: &FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>>
    where
        M: LogicBasedBendersMaster,
        S: ConstraintProgrammingSubproblemSolver,
    {
        self.engine.solve(master, subproblem, options)
    }

    /// 使用 cut oracle pipeline 求解 / Solve with a cut-oracle pipeline.
    pub fn solve_with_oracle<M, S, O>(
        &self,
        master: &mut M,
        subproblem: &mut S,
        oracle: &mut O,
        options: &FrameworkSolveOptions,
    ) -> Result<SolveReport<f64>>
    where
        M: LogicBasedBendersMaster,
        S: ConstraintProgrammingSubproblemSolver,
        O: LogicBasedBendersCutOraclePipeline,
    {
        self.engine
            .solve_with_oracle(master, subproblem, oracle, options)
    }
}

/// CP subproblem 工厂 / CP subproblem factory.
pub trait LogicBasedBendersSubproblemFactory {
    /// 创建的 solver 类型 / Created solver type.
    type Solver: ConstraintProgrammingSubproblemSolver;

    /// 为 subproblem attempt 创建新 solver / Create a fresh solver for a subproblem attempt.
    fn create_subproblem_solver(&mut self) -> Result<Self::Solver>;
}

/// 闭包工厂适配器 / A factory adapter for closures.
pub struct ClosureSubproblemFactory<F, S> {
    factory: F,
    marker: std::marker::PhantomData<fn() -> S>,
}

impl<F, S> ClosureSubproblemFactory<F, S> {
    /// 创建闭包驱动的工厂 / Create a closure-backed factory.
    pub fn new(factory: F) -> Self {
        Self {
            factory,
            marker: std::marker::PhantomData,
        }
    }
}

impl<F, S> LogicBasedBendersSubproblemFactory for ClosureSubproblemFactory<F, S>
where
    F: FnMut() -> Result<S>,
    S: ConstraintProgrammingSubproblemSolver,
{
    type Solver = S;

    fn create_subproblem_solver(&mut self) -> Result<Self::Solver> {
        (self.factory)()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_core::solver::{
        ProblemStatus, SolveProof, SolveSolution, SolverProvenance, TerminationReason,
    };
    use std::collections::BTreeMap;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    fn report() -> SolveReport<f64> {
        SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(SolveSolution {
                stable_values: BTreeMap::from([(StableVariableId::from("x"), 1.0)]),
                ..SolveSolution::vector(vec![1.0])
            })
            .proof(SolveProof::optimality())
            .provenance(SolverProvenance::default())
            .build()
            .expect("report")
    }

    #[test]
    fn extractor_uses_stable_ids_and_binding_pipeline_is_shared() {
        let binding = MasterBinding::new([super::super::MasterVariableBinding::binary("x")])
            .expect("binding");
        let context = LogicBasedBendersContext::new(binding);
        let assignment = context
            .solution_extractor
            .extract_master_assignment(&report(), 1e-9)
            .expect("assignment");
        assert_eq!(assignment.get(&StableVariableId::from("x")), Some(1));
        assert_eq!(
            context.binding_pipeline.binding,
            context.solution_extractor.binding
        );
    }

    #[test]
    fn closure_factory_creates_a_fresh_solver() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_for_factory = Arc::clone(&calls);
        let mut factory = ClosureSubproblemFactory::<_, DummySolver>::new(move || {
            calls_for_factory.fetch_add(1, Ordering::SeqCst);
            Ok(DummySolver)
        });
        factory.create_subproblem_solver().expect("solver");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    struct DummySolver;

    impl ConstraintProgrammingSubproblemSolver for DummySolver {
        fn solve_subproblem(
            &mut self,
            _assignment: &MasterAssignment,
            _options: &FrameworkSolveOptions,
        ) -> Result<ConstraintProgrammingSubproblemResult> {
            Ok(ConstraintProgrammingSubproblemResult::failed(
                "dummy", "unused",
            ))
        }
    }
}
