//! Gantt 可选网络路径桥接，不拥有主模型或启动求解。 / Optional Gantt network-path bridge without master-model ownership or nested solves.

use ospf_rust_framework::model::{ColumnGenerationBatch, ColumnGenerationCompletion, ColumnGenerationScope};
use ospf_rust_framework_network_scheduling::Result;
use ospf_rust_framework_network_scheduling::domain::path::{
    PathCandidate, PathPolicy, PathProblem, PathSearchBudget, PathSearchCompletion,
    PathSearchContext, PathSearchResult,
};
use ospf_rust_framework_network_scheduling::infrastructure::NetworkGraphAccess;

/// 搜索结果与业务列共同返回，空列不能掩盖搜索状态。 / Return search and business columns together; empty columns cannot hide search status.
#[derive(Debug, Clone)]
pub struct NetworkBunchGenerationResult<C, S> {
    /// 业务列及完整作用域 / Business columns and their complete scope.
    pub batch: ColumnGenerationBatch<C>,
    /// 原始标签、前驱与诊断 / Original labels, predecessors and diagnostics.
    pub search: PathSearchResult<S>,
}

/// 网络路径到业务任务束的转换入口 / Network-path to business-bunch conversion facade.
#[derive(Debug, Clone, Copy, Default)]
pub struct NetworkBunchGenerationContext {
    search: PathSearchContext,
}

impl NetworkBunchGenerationContext {
    /// 保留完整路径状态，由调用方映射为 Gantt 业务束。 / Preserve full path state and let the caller construct Gantt business bunches.
    ///
    /// 无改进回调必须独立检查完整搜索域，不能只检查截取后的候选为空。
    /// 完成度证据不代替 Gantt 对当前 LP 对偶证书的来源验证。
    /// The no-improvement callback must independently inspect the whole search domain,
    /// not merely an empty output-limited candidate list. Completion evidence never
    /// replaces Gantt's validation of the current LP dual certificate.
    pub fn generate<N, A, S, G, P, C, E, F>(
        &self,
        problem: &PathProblem<'_, N, A, S, G, P>,
        budget: &PathSearchBudget<'_>,
        scope: ColumnGenerationScope,
        extract: E,
        no_improvement_evidence: F,
    ) -> Result<NetworkBunchGenerationResult<C, S>>
    where
        S: Clone,
        G: NetworkGraphAccess<N, A>,
        P: PathPolicy<N, A, S>,
        E: Fn(&PathCandidate<S>, &PathSearchResult<S>) -> Result<Option<C>>,
        F: Fn(&PathSearchResult<S>) -> Result<bool>,
    {
        let search = self.search.search(problem, budget)?;
        let mut columns = Vec::new();
        let mut extraction_rejected = false;
        for path in &search.paths {
            match extract(path, &search)? {
                Some(column) => columns.push(column),
                None => extraction_rejected = true,
            }
        }
        let completion = match search.completion {
            PathSearchCompletion::Exhausted if !extraction_rejected => ColumnGenerationCompletion::Exhausted,
            PathSearchCompletion::BudgetLimit => ColumnGenerationCompletion::BudgetLimit,
            PathSearchCompletion::Interrupted => ColumnGenerationCompletion::Interrupted,
            _ => ColumnGenerationCompletion::Heuristic,
        };
        let no_improving_columns = if completion == ColumnGenerationCompletion::Exhausted {
            Some(no_improvement_evidence(&search)?)
        } else { None };
        Ok(NetworkBunchGenerationResult {
            batch: ColumnGenerationBatch { columns, completion, scope: Some(scope),
                no_improving_columns, children: Vec::new() },
            search,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ospf_rust_framework_network_scheduling::domain::path::PathTrace;
    use ospf_rust_framework_network_scheduling::infrastructure::{NetworkArc, NetworkGraph, NetworkNode};

    struct Policy;
    impl PathPolicy<(), i32, i32> for Policy {
        fn start(&self, _: &NetworkNode<()>) -> Result<Option<i32>> { Ok(Some(0)) }
        fn extend(&self, state: &i32, arc: &NetworkArc<i32>, _: &NetworkNode<()>) -> Result<Option<i32>> { Ok(Some(state + arc.payload)) }
        fn finish(&self, trace: &PathTrace<'_, (), i32, i32>) -> Result<Option<i32>> { Ok(trace.states.last().map(|value| **value + 5)) }
    }

    fn scope() -> ColumnGenerationScope {
        ColumnGenerationScope { request: "request".into(), model: "model".into(), dual: "dual".into(),
            branch: "root".into(), groups: ["aircraft".into()].into(), executor_slots: ["aircraft:0".into()].into(), global: true }
    }

    #[test]
    fn preserves_full_state_trace_and_budget_status() {
        let graph = NetworkGraph::new(vec![NetworkNode::new("s", ()), NetworkNode::new("t", ())],
            vec![NetworkArc::new("flight", "s", "t", 2)]).unwrap();
        let mut problem = PathProblem::new(&graph, Policy);
        problem.starts = vec!["s".into()];
        problem.terminals = ["t".into()].into();
        let context = NetworkBunchGenerationContext::default();
        let generated = context.generate(&problem, &PathSearchBudget::default(), scope(),
            |path, search| Ok(Some((path.state, search.trace(path.label)?))), |_| Ok(false)).unwrap();
        assert_eq!(generated.batch.columns, vec![(7, vec!["flight".into()])]);
        assert!(generated.batch.certifies(&scope()));
        assert!(!generated.batch.certifies_no_improving_columns(&scope()));
        let budget = PathSearchBudget { maximum_labels: Some(1), ..Default::default() };
        let generated = context.generate(&problem, &budget, scope(), |path, _| Ok(Some(path.state)),
            |_| panic!("不应为截断搜索调用证据回调 / no evidence callback for truncated search")).unwrap();
        assert!(generated.batch.columns.is_empty());
        assert_eq!(generated.batch.completion, ColumnGenerationCompletion::BudgetLimit);
        assert!(!generated.batch.certifies(&scope()));
    }

    #[test]
    fn output_limit_and_extraction_rejection_do_not_create_proof() {
        let graph = NetworkGraph::new(vec![NetworkNode::new("s", ()), NetworkNode::new("t", ())],
            vec![NetworkArc::new("flight", "s", "t", 2)]).unwrap();
        let mut problem = PathProblem::new(&graph, Policy);
        problem.starts = vec!["s".into()];
        problem.terminals = ["t".into()].into();
        problem.options.maximum_results = Some(0);
        let context = NetworkBunchGenerationContext::default();
        let generated = context.generate(&problem, &PathSearchBudget::default(), scope(),
            |path, _| Ok(Some(path.state)), |search| Ok(search.diagnostics.total_results == 0)).unwrap();
        assert!(generated.search.complete());
        assert!(generated.batch.columns.is_empty());
        assert!(!generated.batch.certifies_no_improving_columns(&scope()));
        problem.options.maximum_results = None;
        let generated = context.generate(&problem, &PathSearchBudget::default(), scope(),
            |_, _| Ok(None::<()>), |_| Ok(true)).unwrap();
        assert_eq!(generated.batch.completion, ColumnGenerationCompletion::Heuristic);
    }
}
