//! 确定性离线实验与观测 / Deterministic offline experiments and observations.
//!
//! 实验运行器只负责调度和记录，不修改 solver 配置、callback 或生产决策。
//! The runner only schedules and records cases; it never changes solver configuration,
//! callbacks, or production decisions.

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::thread;

use ospf_rust_core::solver::{AuditFingerprint, SolveReport, SolverProvenance};

/// 离线实验 case 元数据 / Offline experiment case metadata.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentCase<I> {
    /// 稳定 case ID / Stable case ID.
    pub case_id: String,
    /// 生成器标识 / Generator identifier.
    pub generator_id: String,
    /// 生成器版本 / Generator version.
    pub generator_version: String,
    /// 随机种子 / Random seed.
    pub seed: Option<u64>,
    /// 输入实例 / Input instance.
    pub input: I,
    /// 模型指纹 / Model fingerprint.
    pub model_fingerprint: Option<AuditFingerprint>,
    /// 配置指纹 / Configuration fingerprint.
    pub configuration_fingerprint: Option<AuditFingerprint>,
    /// solver 环境指纹 / Solver-environment fingerprint.
    pub solver_fingerprint: Option<AuditFingerprint>,
    /// solver 来源快照 / Solver provenance snapshot.
    pub provenance: Option<SolverProvenance>,
}

impl<I> ExperimentCase<I> {
    /// 创建实验 case / Create an experiment case.
    pub fn new(
        case_id: impl Into<String>,
        generator_id: impl Into<String>,
        generator_version: impl Into<String>,
        input: I,
    ) -> Self {
        Self {
            case_id: case_id.into(),
            generator_id: generator_id.into(),
            generator_version: generator_version.into(),
            seed: None,
            input,
            model_fingerprint: None,
            configuration_fingerprint: None,
            solver_fingerprint: None,
            provenance: None,
        }
    }

    /// 设置随机种子 / Set the random seed.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// 设置模型和配置指纹 / Set model and configuration fingerprints.
    pub fn with_fingerprints(
        mut self,
        model_fingerprint: Option<AuditFingerprint>,
        configuration_fingerprint: Option<AuditFingerprint>,
    ) -> Self {
        self.model_fingerprint = model_fingerprint;
        self.configuration_fingerprint = configuration_fingerprint;
        self
    }

    /// 设置 solver 指纹 / Set the solver fingerprint.
    pub fn with_solver_fingerprint(mut self, solver_fingerprint: AuditFingerprint) -> Self {
        self.solver_fingerprint = Some(solver_fingerprint);
        self
    }

    /// 设置 solver 来源 / Set solver provenance.
    pub fn with_provenance(mut self, provenance: SolverProvenance) -> Self {
        self.provenance = Some(provenance);
        self
    }
}

/// 单个 case 的结构化错误 / Structured error for one case.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentError {
    /// 稳定错误消息 / Stable error message.
    pub message: String,
}

impl ExperimentError {
    /// 创建错误 / Create an experiment error.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// 单个 case 的运行结果 / Result of one experiment case.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct ExperimentResult<R> {
    /// case 元数据（不含输入，避免重复持有大对象）/ Case metadata without the input payload.
    pub metadata: ExperimentMetadata,
    /// 求解结果或启动/运行错误 / Solve result or pre-start/runtime error.
    pub outcome: std::result::Result<R, ExperimentError>,
}

/// 结果中保留的 case 元数据 / Case metadata retained in a result.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentMetadata {
    /// 稳定 case ID / Stable case ID.
    pub case_id: String,
    /// 生成器标识 / Generator identifier.
    pub generator_id: String,
    /// 生成器版本 / Generator version.
    pub generator_version: String,
    /// 随机种子 / Random seed.
    pub seed: Option<u64>,
    /// 模型指纹 / Model fingerprint.
    pub model_fingerprint: Option<AuditFingerprint>,
    /// 配置指纹 / Configuration fingerprint.
    pub configuration_fingerprint: Option<AuditFingerprint>,
    /// solver 环境指纹 / Solver-environment fingerprint.
    pub solver_fingerprint: Option<AuditFingerprint>,
    /// solver 来源快照 / Solver provenance snapshot.
    pub provenance: Option<SolverProvenance>,
}

impl<I> From<&ExperimentCase<I>> for ExperimentMetadata {
    fn from(case: &ExperimentCase<I>) -> Self {
        Self {
            case_id: case.case_id.clone(),
            generator_id: case.generator_id.clone(),
            generator_version: case.generator_version.clone(),
            seed: case.seed,
            model_fingerprint: case.model_fingerprint.clone(),
            configuration_fingerprint: case.configuration_fingerprint.clone(),
            solver_fingerprint: case.solver_fingerprint.clone(),
            provenance: case.provenance.clone(),
        }
    }
}

/// 确定性批量运行器 / Deterministic batch runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExperimentRunner {
    concurrency_budget: usize,
}

impl ExperimentRunner {
    /// 创建运行器；预算至少为一个 worker / Create a runner with at least one worker.
    pub fn new(concurrency_budget: usize) -> Self {
        Self {
            concurrency_budget: concurrency_budget.max(1),
        }
    }

    /// 获取并发预算 / Get the concurrency budget.
    pub fn concurrency_budget(self) -> usize {
        self.concurrency_budget
    }

    /// 并发运行 case，并按 case ID 稳定排序结果 / Run cases concurrently and sort results by case ID.
    pub fn run<I, R, E, F>(
        self,
        cases: Vec<ExperimentCase<I>>,
        solve: F,
    ) -> Vec<ExperimentResult<R>>
    where
        I: Send + 'static,
        R: Send + 'static,
        E: std::fmt::Display,
        F: Fn(I) -> std::result::Result<R, E> + Send + Sync + 'static,
    {
        let queue = Arc::new(Mutex::new(VecDeque::from(cases)));
        let results = Arc::new(Mutex::new(Vec::new()));
        let solve = Arc::new(solve);
        let mut workers = Vec::new();

        for _ in 0..self.concurrency_budget {
            let queue = Arc::clone(&queue);
            let results = Arc::clone(&results);
            let solve = Arc::clone(&solve);
            workers.push(thread::spawn(move || {
                loop {
                    let case = queue
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .pop_front();
                    let Some(case) = case else {
                        break;
                    };
                    let metadata = ExperimentMetadata::from(&case);
                    let outcome =
                        solve(case.input).map_err(|error| ExperimentError::new(error.to_string()));
                    results
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .push(ExperimentResult { metadata, outcome });
                }
            }));
        }

        for worker in workers {
            let _ = worker.join();
        }
        let mut results = Arc::try_unwrap(results)
            .ok()
            .and_then(|results| results.into_inner().ok())
            .unwrap_or_default();
        results.sort_by(|left, right| left.metadata.case_id.cmp(&right.metadata.case_id));
        results
    }

    /// 运行并校验统一报告 case / Run cases and validate unified solve reports.
    ///
    /// 该入口保证每个成功 case 的结果携带完整 `SolveReport`，从而同时保留
    /// 生成器元数据、指纹和 solver provenance；求解失败则保留结构化错误。
    /// This entry point ensures every successful case retains a complete `SolveReport`,
    /// preserving generator metadata, fingerprints, and solver provenance while retaining
    /// structured errors for failed cases.
    pub fn run_reports<I, E, F>(
        self,
        cases: Vec<ExperimentCase<I>>,
        solve: F,
    ) -> Vec<ExperimentResult<SolveReport<f64>>>
    where
        I: Send + 'static,
        E: std::fmt::Display,
        F: Fn(I) -> std::result::Result<SolveReport<f64>, E> + Send + Sync + 'static,
    {
        let mut results = self.run(cases, move |input| {
            let report = solve(input).map_err(|error| error.to_string())?;
            report.validate().map_err(|error| error.to_string())?;
            Ok::<_, String>(report)
        });
        for result in &mut results {
            let Ok(report) = result.outcome.as_ref() else {
                continue;
            };
            let model_fingerprint = report.fingerprints.model.clone();
            let configuration_fingerprint = report.fingerprints.configuration.clone();
            let solver_fingerprint = report.fingerprints.solver.clone();
            let provenance = report.provenance.clone();
            if result.metadata.model_fingerprint.is_some()
                && result.metadata.model_fingerprint != model_fingerprint
            {
                result.outcome = Err(ExperimentError::new(
                    "experiment model fingerprint does not match the solve report",
                ));
                continue;
            }
            if result.metadata.configuration_fingerprint.is_some()
                && result.metadata.configuration_fingerprint != configuration_fingerprint
            {
                result.outcome = Err(ExperimentError::new(
                    "experiment configuration fingerprint does not match the solve report",
                ));
                continue;
            }
            if result.metadata.solver_fingerprint.is_some()
                && result.metadata.solver_fingerprint != solver_fingerprint
            {
                result.outcome = Err(ExperimentError::new(
                    "experiment solver fingerprint does not match the solve report",
                ));
                continue;
            }
            if result.metadata.provenance.is_some()
                && result.metadata.provenance.as_ref() != Some(&provenance)
            {
                result.outcome = Err(ExperimentError::new(
                    "experiment provenance does not match the solve report",
                ));
                continue;
            }
            result.metadata.model_fingerprint = model_fingerprint;
            result.metadata.configuration_fingerprint = configuration_fingerprint;
            result.metadata.solver_fingerprint = solver_fingerprint;
            result.metadata.provenance = Some(provenance);
        }
        results
    }
}

impl Default for ExperimentRunner {
    fn default() -> Self {
        Self::new(1)
    }
}

/// 只读求解观测 / Read-only solve observation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct SolveObservation {
    /// case ID / Case ID.
    pub case_id: String,
    /// 报告问题结论 / Problem status from the report.
    pub problem_status: ospf_rust_core::solver::ProblemStatus,
    /// 终止原因 / Termination reason.
    pub termination_reason: ospf_rust_core::solver::TerminationReason,
    /// incumbent 目标 / Incumbent objective.
    pub objective_value: Option<f64>,
    /// 最优下界 / Best bound.
    pub best_bound: Option<f64>,
    /// 相对 gap / Relative gap.
    pub relative_gap: Option<f64>,
    /// 搜索节点数 / Search-node count.
    pub nodes: Option<usize>,
    /// Branch-and-Price/算法 trace / Branch-and-Price or algorithm trace.
    pub trace: ospf_rust_core::solver::SolveTrace,
    /// 只读标签 / Read-only labels.
    pub labels: BTreeMap<String, String>,
}

/// 从报告提取观测，不改变报告 / Extract an observation without changing the report.
pub fn observe_report(
    case_id: impl Into<String>,
    report: &SolveReport<f64>,
    labels: BTreeMap<String, String>,
) -> SolveObservation {
    SolveObservation {
        case_id: case_id.into(),
        problem_status: report.problem_status,
        termination_reason: report.termination_reason,
        objective_value: report
            .solution
            .as_ref()
            .and_then(|solution| solution.objective_value.or(solution.objective)),
        best_bound: report.statistics.best_bound_value,
        relative_gap: report.statistics.relative_gap,
        nodes: report.statistics.nodes,
        trace: report.trace.clone(),
        labels,
    }
}

/// 将实验结果编码为 JSON / Encode experiment results as JSON.
#[cfg(feature = "experiment")]
pub fn results_to_json<R>(results: &[ExperimentResult<R>]) -> Result<String, serde_json::Error>
where
    R: serde::Serialize,
{
    serde_json::to_string_pretty(results)
}

#[cfg(all(test, feature = "experiment"))]
mod json_tests {
    use super::*;

    #[test]
    fn json_export_preserves_case_order() {
        let cases = vec![
            ExperimentCase::new("b", "fixture", "1", 2),
            ExperimentCase::new("a", "fixture", "1", 1),
        ];
        let results = ExperimentRunner::new(2).run(cases, |input| Ok::<_, &str>(input));
        let json = results_to_json(&results).unwrap();
        assert!(
            json.find("\"case_id\": \"a\"").unwrap() < json.find("\"case_id\": \"b\"").unwrap()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_core::solver::{
        AuditFingerprint, ProblemStatus, SolveFingerprints, TerminationReason,
    };

    #[test]
    fn runner_respects_budget_and_sorts_case_ids() {
        let cases = vec![
            ExperimentCase::new("case-b", "fixture", "1", 2),
            ExperimentCase::new("case-a", "fixture", "1", 1),
            ExperimentCase::new("case-c", "fixture", "1", 3),
        ];
        let results = ExperimentRunner::new(2).run(cases, |input| Ok::<_, &str>(input * 2));
        assert_eq!(
            results
                .iter()
                .map(|result| result.metadata.case_id.as_str())
                .collect::<Vec<_>>(),
            vec!["case-a", "case-b", "case-c"]
        );
        assert_eq!(results[0].outcome, Ok(2));
    }

    #[test]
    fn report_runner_retains_generation_metadata_and_provenance() {
        let report = SolveReport::builder(ProblemStatus::Unknown, TerminationReason::TimeLimit)
            .provenance(SolverProvenance {
                solver_id: "fixture/solver".to_owned(),
                backend_name: "fixture".to_owned(),
                ..SolverProvenance::default()
            })
            .fingerprints(SolveFingerprints {
                model: Some(AuditFingerprint {
                    schema_version: "1.0".to_owned(),
                    algorithm: "SHA-256".to_owned(),
                    value: "model".to_owned(),
                }),
                configuration: Some(AuditFingerprint {
                    schema_version: "1.0".to_owned(),
                    algorithm: "SHA-256".to_owned(),
                    value: "configuration".to_owned(),
                }),
                solver: Some(AuditFingerprint {
                    schema_version: "1.0".to_owned(),
                    algorithm: "SHA-256".to_owned(),
                    value: "solver".to_owned(),
                }),
            })
            .build()
            .expect("time-limit report without incumbent should be valid");
        let cases = vec![ExperimentCase::new("case-a", "generator", "v2", 7).with_seed(42)];
        let results =
            ExperimentRunner::default().run_reports(cases, move |_| Ok::<_, &str>(report.clone()));

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.generator_version, "v2");
        assert_eq!(results[0].metadata.seed, Some(42));
        assert_eq!(
            results[0]
                .metadata
                .model_fingerprint
                .as_ref()
                .map(|fingerprint| fingerprint.value.as_str()),
            Some("model")
        );
        assert_eq!(
            results[0]
                .metadata
                .configuration_fingerprint
                .as_ref()
                .map(|fingerprint| fingerprint.value.as_str()),
            Some("configuration")
        );
        assert_eq!(
            results[0]
                .metadata
                .solver_fingerprint
                .as_ref()
                .map(|fingerprint| fingerprint.value.as_str()),
            Some("solver")
        );
        assert_eq!(
            results[0]
                .metadata
                .provenance
                .as_ref()
                .map(|provenance| provenance.solver_id.as_str()),
            Some("fixture/solver")
        );
        let report = results[0]
            .outcome
            .as_ref()
            .expect("report case should succeed");
        assert_eq!(report.termination_reason, TerminationReason::TimeLimit);
        assert_eq!(report.provenance.solver_id, "fixture/solver");
    }
}
