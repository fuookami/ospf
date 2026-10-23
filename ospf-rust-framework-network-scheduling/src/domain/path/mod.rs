//! 有限域资源路径搜索，不假设资源单调或费用可加。 / Finite-domain resource paths without monotonicity or additive-cost assumptions.

use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeSet, BinaryHeap, HashMap};
use std::marker::PhantomData;
use crate::error::{NetworkSchedulingError, Result};
use crate::infrastructure::{NetworkArc, NetworkArcId, NetworkGraphAccess, NetworkNode, NetworkNodeId, NetworkTopology};

/// 完整路径回溯，端点按当前视图解释 / Full path trace with endpoints interpreted through the current view.
pub struct PathTrace<'a, N, A, S> {
    /// 从起点到终点的节点 / Nodes from source to destination.
    pub nodes: Vec<&'a NetworkNode<N>>,
    /// 有序业务弧，身份不因视图变化 / Ordered business arcs retaining their identities.
    pub arcs: Vec<&'a NetworkArc<A>>,
    /// 每个节点上的增量状态 / Incremental state at each node.
    pub states: Vec<&'a S>,
}

/// 领域转移及完整路径策略；回调失败原样传播 / Domain transitions and whole-path policy; callback errors propagate unchanged.
pub trait PathPolicy<N, A, S> {
    /// 构造起点状态，为空表示不允许该起点 / Construct initial state, or reject a source.
    fn start(&self, node: &NetworkNode<N>) -> Result<Option<S>>;

    /// 扩展资源状态，为空表示业务不可行 / Extend resources, or reject an infeasible transition.
    fn extend(
        &self,
        state: &S,
        arc: &NetworkArc<A>,
        destination: &NetworkNode<N>,
    ) -> Result<Option<S>>;

    /// 带稳定前驱索引的扩展，默认使用资源转移 / Extend with a stable predecessor index; delegates to resource transition by default.
    fn extend_from_label(
        &self,
        _index: usize,
        label: &PathLabel<S>,
        arc: &NetworkArc<A>,
        destination: &NetworkNode<N>,
    ) -> Result<Option<S>> { self.extend(&label.state, arc, destination) }

    /// 完整终点贡献或业务筛选，可包含非可加成本 / Terminal contribution or business filter, including non-additive costs.
    fn finish(&self, trace: &PathTrace<'_, N, A, S>) -> Result<Option<S>>;

    /// 独立完整路径校验，拒绝使穷尽声明失效 / Independent validation; rejection invalidates exhaustiveness.
    fn validate(&self, _trace: &PathTrace<'_, N, A, S>, _finished: &S) -> Result<bool> { Ok(true) }

    /// 可选支配，默认不剪枝 / Optional dominance, disabled by default.
    fn dominates(&self, _left: &S, _right: &S) -> bool { false }

    /// 是否启用支配，默认避免无意义的二次扫描 / Enable dominance; disabled by default to avoid quadratic scans.
    fn has_dominance(&self) -> bool { false }

    /// 业务认证支配保留可行性、费用及主问题系数 / Business certification of feasibility, cost and master-coefficient preservation.
    fn dominance_is_exact(&self) -> bool { false }

    /// 返回路径排序，不影响搜索是否继续 / Output ordering, without affecting traversal completion.
    fn compare(&self, _left: &S, _right: &S) -> Ordering { Ordering::Equal }
}

/// 业务路径域与输出选项 / Business path domain and output options.
#[derive(Debug, Clone)]
pub struct PathSearchOptions {
    /// 禁止重复访问节点 / Disallow repeated nodes.
    pub elementary: bool,
    /// 业务最大弧数；循环非 elementary 域必须声明 / Business arc-count bound, required for cyclic non-elementary walks.
    pub maximum_steps: Option<usize>,
    /// 仅限制返回数量，不停止搜索 / Output-only limit, never an early search stop.
    pub maximum_results: Option<usize>,
}

impl Default for PathSearchOptions {
    fn default() -> Self { Self { elementary: true, maximum_steps: None, maximum_results: None } }
}

/// 计算预算，不是业务路径约束 / Computational budget, not a business path constraint.
#[derive(Default)]
pub struct PathSearchBudget<'a> {
    /// 累计生成标签数上限 / Maximum total generated labels.
    pub maximum_labels: Option<usize>,
    /// 单节点累计标签数上限 / Maximum generated labels per node.
    pub maximum_labels_per_node: Option<usize>,
    /// 协作式中断回调 / Cooperative interruption callback.
    pub interrupted: Option<&'a dyn Fn() -> bool>,
}

/// 每请求独立的资源路径问题 / Independent per-request resource path problem.
pub struct PathProblem<'a, N, A, S, G, P> {
    /// 不可变图或视图 / Immutable graph or view.
    pub graph: &'a G,
    /// 起点身份 / Source identities.
    pub starts: Vec<NetworkNodeId>,
    /// 可接受终点身份 / Acceptable terminal identities.
    pub terminals: BTreeSet<NetworkNodeId>,
    /// 业务回调策略 / Domain callback policy.
    pub policy: P,
    /// 路径与输出选项 / Path and output options.
    pub options: PathSearchOptions,
    marker: PhantomData<fn(N, A, S)>,
}

impl<'a, N, A, S, G, P> PathProblem<'a, N, A, S, G, P> {
    /// 创建问题；调用方设置起点、终点及业务选项 / Create a problem; the caller supplies sources, terminals and options.
    pub fn new(graph: &'a G, policy: P) -> Self {
        Self { graph, starts: Vec::new(), terminals: BTreeSet::new(), policy,
            options: PathSearchOptions::default(), marker: PhantomData }
    }
}

/// 标签前驱保留在结果内，支配后也不删除 / Labels retain predecessors even after dominance pruning.
#[derive(Debug, Clone)]
pub struct PathLabel<S> {
    /// 增量业务状态 / Incremental business state.
    pub state: S,
    /// 当前节点 / Current node.
    pub node: NetworkNodeId,
    /// 前驱标签 / Predecessor label.
    pub predecessor: Option<usize>,
    /// 到达当前标签的弧 / Incoming arc for this label.
    pub arc: Option<NetworkArcId>,
    /// 已走弧数 / Traversed arc count.
    pub steps: usize,
    visited: BTreeSet<NetworkNodeId>,
    active: bool,
}

/// 包含完整终点贡献的候选 / Candidate including complete terminal contributions.
#[derive(Debug, Clone)]
pub struct PathCandidate<S> {
    /// 终点标签索引 / Terminal label index.
    pub label: usize,
    /// 完整业务状态 / Finished domain state.
    pub state: S,
}

/// 搜索真实完成状态 / Actual search completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathSearchCompletion {
    /// 域已穷尽（可含认证支配） / Domain exhausted, possibly with certified dominance.
    Exhausted,
    /// 计算预算耗尽 / Computational budget exhausted.
    BudgetLimit,
    /// 协作中断 / Cooperative interruption.
    Interrupted,
    /// 使用了未经业务认证的支配 / Uncertified dominance was used.
    UncertifiedDominance,
    /// 完整路径校验拒绝增量可行候选 / Full validation rejected an incrementally feasible candidate.
    ValidationRejected,
}

/// 搜索诊断 / Search diagnostics.
#[derive(Debug, Clone, Default)]
pub struct PathSearchDiagnostics {
    /// 累计创建标签 / Total generated labels.
    pub generated_labels: usize,
    /// 业务转移接受数，含随后被支配的状态 / Accepted transitions, including subsequently dominated states.
    pub extensions: usize,
    /// 支配扫描的活跃候选数 / Active candidates inspected for dominance.
    pub dominance_checks: usize,
    /// 支配删除标签 / Dominance-pruned labels.
    pub dominated_labels: usize,
    /// 完整校验拒绝数 / Full-validation rejections.
    pub validation_rejections: usize,
    /// 输出截取前的有效候选数 / Valid candidates before output truncation.
    pub total_results: usize,
}

/// 路径、前驱及完成度必须一起读取 / Paths, predecessors and completion must be consumed together.
#[derive(Debug, Clone)]
pub struct PathSearchResult<S> {
    /// 经排序和输出截取的完整路径 / Sorted and output-limited finished paths.
    pub paths: Vec<PathCandidate<S>>,
    /// 包括被支配标签的前驱池 / Predecessor pool including dominated labels.
    pub labels: Vec<PathLabel<S>>,
    /// 真实完成状态 / Actual completion.
    pub completion: PathSearchCompletion,
    /// 诊断 / Diagnostics.
    pub diagnostics: PathSearchDiagnostics,
}

impl<S> PathSearchResult<S> {
    /// 是否完整搜索当前业务域，不等价于无改进或整数最优证明 / Exhaustiveness, not a no-improvement or integer-optimality certificate.
    pub fn complete(&self) -> bool { self.completion == PathSearchCompletion::Exhausted }

    /// 按标签索引回溯稳定弧序列 / Trace stable arc identities from a label index.
    pub fn trace(&self, label: usize) -> Result<Vec<NetworkArcId>> {
        let mut result = Vec::new();
        let mut current = Some(label);
        while let Some(index) = current {
            let item = self.labels.get(index).ok_or_else(|| NetworkSchedulingError::contract(
                "路径标签索引无效 / invalid path label index"))?;
            if let Some(arc) = &item.arc { result.push(arc.clone()); }
            if item.predecessor.is_some_and(|parent| parent >= index) {
                return Err(NetworkSchedulingError::contract("路径前驱无效 / invalid path predecessor"));
            }
            current = item.predecessor;
        }
        result.reverse();
        Ok(result)
    }
}

/// 无求解器、无跨请求可变状态的路径入口 / Solver-free path facade without cross-request mutable state.
#[derive(Debug, Default, Clone, Copy)]
pub struct PathSearchContext;

fn make_trace<'a, N, A, S, G>(
    graph: &'a G,
    labels: &'a [PathLabel<S>],
    index: usize,
) -> Result<PathTrace<'a, N, A, S>>
where
    G: NetworkGraphAccess<N, A>,
{
    let mut chain = Vec::new();
    let mut current = Some(index);
    while let Some(index) = current {
        let label = &labels[index];
        chain.push(label);
        current = label.predecessor;
    }
    chain.reverse();
    let nodes = chain.iter().map(|label| graph.node(&label.node).ok_or_else(||
        NetworkSchedulingError::contract("回溯节点缺失 / missing trace node"))).collect::<Result<_>>()?;
    let arcs = chain.iter().filter_map(|label| label.arc.as_ref()).map(|id| graph.arc(id)
        .ok_or_else(|| NetworkSchedulingError::contract("回溯弧缺失 / missing trace arc"))).collect::<Result<_>>()?;
    Ok(PathTrace { nodes, arcs, states: chain.iter().map(|label| &label.state).collect() })
}

fn admit_terminal<N, A, S, G, P>(
    problem: &PathProblem<'_, N, A, S, G, P>,
    result: &mut PathSearchResult<S>,
    index: usize,
) -> Result<()>
where
    G: NetworkGraphAccess<N, A>,
    P: PathPolicy<N, A, S>,
{
    if problem.terminals.contains(&result.labels[index].node) {
        let trace = make_trace(problem.graph, &result.labels, index)?;
        if let Some(state) = problem.policy.finish(&trace)? {
            if problem.policy.validate(&trace, &state)? {
                result.paths.push(PathCandidate { label: index, state });
            } else {
                result.diagnostics.validation_rejections += 1;
                if result.completion == PathSearchCompletion::Exhausted {
                    result.completion = PathSearchCompletion::ValidationRejected;
                }
            }
        }
    }
    Ok(())
}

impl PathSearchContext {
    /// 搜索有限业务域；预算终态正常返回，业务回调错误传播 / Search a finite business domain; budgets are normal outcomes, callback failures propagate.
    pub fn search<N, A, S, G, P>(
        &self,
        problem: &PathProblem<'_, N, A, S, G, P>,
        budget: &PathSearchBudget<'_>,
    ) -> Result<PathSearchResult<S>>
    where
        S: Clone,
        G: NetworkGraphAccess<N, A>,
        P: PathPolicy<N, A, S>,
    {
        let order = NetworkTopology::new(problem.graph).topological_order()?;
        if !problem.options.elementary && problem.options.maximum_steps.is_none() && order.is_none() {
            return Err(NetworkSchedulingError::validation("循环游走必须声明步数上限 / cyclic walks require a step bound"));
        }
        let mut starts = BTreeSet::new();
        for id in &problem.starts {
            if !starts.insert(id) || problem.graph.node(id).is_none() {
                return Err(NetworkSchedulingError::validation("起点重复或缺失 / duplicate or missing source"));
            }
        }
        if problem.terminals.iter().any(|id| problem.graph.node(id).is_none()) {
            return Err(NetworkSchedulingError::validation("终点不存在 / missing terminal"));
        }
        let ranks: HashMap<_, _> = order.unwrap_or_default().into_iter().enumerate().map(|(i, id)| (id, i)).collect();
        let nodes: HashMap<_, _> = problem.graph.nodes().map(|node| (node.id.clone(), node)).collect();
        let mut outgoing: HashMap<NetworkNodeId, Vec<&NetworkArc<A>>> = HashMap::new();
        for arc in problem.graph.arcs() {
            outgoing.entry(problem.graph.from(arc).clone()).or_default().push(arc);
        }
        let mut result = PathSearchResult { paths: Vec::new(), labels: Vec::new(),
            completion: PathSearchCompletion::Exhausted, diagnostics: PathSearchDiagnostics::default() };
        let mut at_node: HashMap<NetworkNodeId, Vec<usize>> = HashMap::new();
        let mut active_at_node: HashMap<NetworkNodeId, Vec<usize>> = HashMap::new();
        let mut queue = BinaryHeap::new();
        let stopped = |result: &mut PathSearchResult<S>, node: &NetworkNodeId,
            at_node: &HashMap<NetworkNodeId, Vec<usize>>| -> bool {
            if budget.interrupted.is_some_and(|interrupt| interrupt()) {
                result.completion = PathSearchCompletion::Interrupted;
                return true;
            }
            if budget.maximum_labels.is_some_and(|limit| result.labels.len() >= limit)
                || budget.maximum_labels_per_node.is_some_and(|limit| at_node.get(node).map_or(0, Vec::len) >= limit)
            {
                result.completion = PathSearchCompletion::BudgetLimit;
                return true;
            }
            false
        };
        'search: {
            for start in &problem.starts {
                if stopped(&mut result, start, &at_node) { break 'search; }
                let node = nodes.get(start).copied().ok_or_else(|| NetworkSchedulingError::contract("起点缺失 / missing source"))?;
                if let Some(state) = problem.policy.start(node)? {
                    let index = result.labels.len();
                    result.labels.push(PathLabel { state, node: start.clone(), predecessor: None, arc: None,
                        steps: 0, visited: [start.clone()].into(), active: true });
                    at_node.entry(start.clone()).or_default().push(index);
                    if problem.policy.has_dominance() { active_at_node.entry(start.clone()).or_default().push(index); }
                    queue.push(Reverse((*ranks.get(start).unwrap_or(&0), index)));
                    result.diagnostics.generated_labels += 1;
                    admit_terminal(problem, &mut result, index)?;
                }
            }
            while let Some(Reverse((_, index))) = queue.pop() {
                if budget.interrupted.is_some_and(|interrupt| interrupt()) {
                    result.completion = PathSearchCompletion::Interrupted;
                    break;
                }
                if !result.labels[index].active { continue; }
                if problem.options.maximum_steps.is_some_and(|limit| result.labels[index].steps >= limit) { continue; }
                let current_node = result.labels[index].node.clone();
                for arc in outgoing.get(&current_node).into_iter().flatten().copied() {
                    if budget.interrupted.is_some_and(|interrupt| interrupt()) {
                        result.completion = PathSearchCompletion::Interrupted;
                        break 'search;
                    }
                    let next = problem.graph.to(arc);
                    if problem.options.elementary && result.labels[index].visited.contains(next) { continue; }
                    let destination = nodes.get(next).copied().ok_or_else(|| NetworkSchedulingError::contract("弧终点缺失 / missing arc destination"))?;
                    let Some(state) = problem.policy.extend_from_label(index, &result.labels[index], arc, destination)? else { continue; };
                    result.diagnostics.extensions += 1;
                    let mut visited = result.labels[index].visited.clone();
                    visited.insert(next.clone());
                    let steps = result.labels[index].steps + 1;
                    let can_dominate = |left: &PathLabel<S>, right_state: &S,
                        right_visited: &BTreeSet<NetworkNodeId>, right_steps: usize| {
                        left.active && (!problem.options.elementary || left.visited.is_subset(right_visited))
                            && (problem.options.maximum_steps.is_none() || left.steps <= right_steps)
                            && problem.policy.dominates(&left.state, right_state)
                    };
                    let existing = if problem.policy.has_dominance() {
                        active_at_node.get(next).cloned().unwrap_or_default()
                    } else { Vec::new() };
                    result.diagnostics.dominance_checks += existing.len();
                    if existing.iter().any(|&old| can_dominate(&result.labels[old], &state, &visited, steps)) {
                        result.diagnostics.dominated_labels += 1;
                        if !problem.policy.dominance_is_exact() { result.completion = PathSearchCompletion::UncertifiedDominance; }
                        continue;
                    }
                    if stopped(&mut result, next, &at_node) { break 'search; }
                    let candidate = PathLabel { state, node: next.clone(), predecessor: Some(index),
                        arc: Some(arc.id.clone()), steps, visited, active: true };
                    for old in existing {
                        if can_dominate(&candidate, &result.labels[old].state, &result.labels[old].visited, result.labels[old].steps) {
                            result.labels[old].active = false;
                            result.diagnostics.dominated_labels += 1;
                            if !problem.policy.dominance_is_exact() { result.completion = PathSearchCompletion::UncertifiedDominance; }
                        }
                    }
                    let label = result.labels.len();
                    result.labels.push(candidate);
                    at_node.entry(next.clone()).or_default().push(label);
                    if problem.policy.has_dominance() {
                        let active = active_at_node.entry(next.clone()).or_default();
                        active.retain(|&old| result.labels[old].active);
                        active.push(label);
                    }
                    queue.push(Reverse((*ranks.get(next).unwrap_or(&0), label)));
                    result.diagnostics.generated_labels += 1;
                    // 先接纳完整路径，后续预算中断不能抹掉已发现候选。 / Admit complete paths before later budgets can interrupt traversal.
                    admit_terminal(problem, &mut result, label)?;
                }
            }
        }
        result.paths.sort_by(|left, right| problem.policy.compare(&left.state, &right.state));
        result.diagnostics.total_results = result.paths.len();
        if let Some(limit) = problem.options.maximum_results { result.paths.truncate(limit); }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::NetworkGraph;

    struct Costs;
    impl PathPolicy<(), i32, i32> for Costs {
        fn start(&self, _: &NetworkNode<()>) -> Result<Option<i32>> { Ok(Some(0)) }
        fn extend(&self, state: &i32, arc: &NetworkArc<i32>, _: &NetworkNode<()>) -> Result<Option<i32>> { Ok(Some(state + arc.payload)) }
        fn finish(&self, trace: &PathTrace<'_, (), i32, i32>) -> Result<Option<i32>> { Ok(trace.states.last().map(|value| **value + 7)) }
        fn compare(&self, left: &i32, right: &i32) -> Ordering { left.cmp(right) }
    }

    fn graph() -> NetworkGraph<(), i32> {
        NetworkGraph::new(["s", "a", "t"].map(|id| NetworkNode::new(id, ())).to_vec(),
            vec![NetworkArc::new("direct", "s", "t", 4), NetworkArc::new("first", "s", "a", 10),
                NetworkArc::new("credit", "a", "t", -12), NetworkArc::new("parallel", "s", "a", 1)]).unwrap()
    }

    #[test]
    fn terminal_contributions_non_monotone_state_and_output_limit() {
        let graph = graph();
        let mut problem = PathProblem::new(&graph, Costs);
        problem.starts = vec!["s".into()];
        problem.terminals = ["t".into()].into();
        problem.options.maximum_results = Some(1);
        let result = PathSearchContext.search(&problem, &PathSearchBudget::default()).unwrap();
        assert!(result.complete());
        assert_eq!(result.diagnostics.total_results, 3);
        assert_eq!(result.paths[0].state, -4);
        assert_eq!(result.trace(result.paths[0].label).unwrap(), vec!["parallel".into(), "credit".into()]);
        problem.options.maximum_results = Some(0);
        let result = PathSearchContext.search(&problem, &PathSearchBudget::default()).unwrap();
        assert!(result.complete());
        assert!(result.paths.is_empty());
        assert_eq!(result.diagnostics.total_results, 3);
    }

    #[test]
    fn admitted_terminals_survive_budgets_and_source_results_are_finalized() {
        let graph = graph();
        let mut problem = PathProblem::new(&graph, Costs);
        problem.starts = vec!["s".into()];
        problem.terminals = ["t".into()].into();
        let budget = PathSearchBudget { maximum_labels: Some(2), ..Default::default() };
        let result = PathSearchContext.search(&problem, &budget).unwrap();
        assert_eq!(result.completion, PathSearchCompletion::BudgetLimit);
        assert_eq!(result.paths.len(), 1);
        assert_eq!(result.paths[0].state, 11);
        assert_eq!(result.trace(result.paths[0].label).unwrap(), vec!["direct".into()]);

        problem.starts = vec!["s".into(), "a".into()];
        problem.terminals = ["s".into(), "a".into()].into();
        problem.options.maximum_results = Some(0);
        let budget = PathSearchBudget { maximum_labels: Some(1), ..Default::default() };
        let result = PathSearchContext.search(&problem, &budget).unwrap();
        assert_eq!(result.completion, PathSearchCompletion::BudgetLimit);
        assert!(result.paths.is_empty());
        assert_eq!(result.diagnostics.total_results, 1);
        let budget = PathSearchBudget { maximum_labels: Some(0), ..Default::default() };
        let result = PathSearchContext.search(&problem, &budget).unwrap();
        assert!(result.labels.is_empty());
        assert_eq!(result.completion, PathSearchCompletion::BudgetLimit);
    }

    #[test]
    fn interruption_is_checked_between_rejected_transitions() {
        struct Reject(std::cell::Cell<usize>);
        impl PathPolicy<(), i32, i32> for Reject {
            fn start(&self, _: &NetworkNode<()>) -> Result<Option<i32>> { Ok(Some(0)) }
            fn extend(&self, _: &i32, _: &NetworkArc<i32>, _: &NetworkNode<()>) -> Result<Option<i32>> {
                self.0.set(self.0.get() + 1);
                Ok(None)
            }
            fn finish(&self, _: &PathTrace<'_, (), i32, i32>) -> Result<Option<i32>> { Ok(None) }
        }
        let graph = graph();
        let mut problem = PathProblem::new(&graph, Reject(std::cell::Cell::new(0)));
        problem.starts = vec!["s".into()];
        let interrupt = || problem.policy.0.get() > 0;
        let result = PathSearchContext.search(&problem, &PathSearchBudget {
            interrupted: Some(&interrupt), ..Default::default()
        }).unwrap();
        assert_eq!(problem.policy.0.get(), 1);
        assert_eq!(result.completion, PathSearchCompletion::Interrupted);
        assert_eq!(result.diagnostics.extensions, 0);
    }

    #[test]
    fn budgets_interruptions_and_cyclic_domain_are_distinct() {
        let graph = graph();
        let mut problem = PathProblem::new(&graph, Costs);
        problem.starts = vec!["s".into()];
        problem.terminals = ["t".into()].into();
        for budget in [PathSearchBudget { maximum_labels: Some(1), ..Default::default() },
            PathSearchBudget { maximum_labels_per_node: Some(1), ..Default::default() }]
        {
            let result = PathSearchContext.search(&problem, &budget).unwrap();
            assert_eq!(result.completion, PathSearchCompletion::BudgetLimit);
        }
        let interrupt = || true;
        let budget = PathSearchBudget { interrupted: Some(&interrupt), ..Default::default() };
        assert_eq!(PathSearchContext.search(&problem, &budget).unwrap().completion, PathSearchCompletion::Interrupted);
        let cyclic = NetworkGraph::new(vec![NetworkNode::new("s", ())], vec![NetworkArc::new("loop", "s", "s", -1)]).unwrap();
        let mut problem = PathProblem::new(&cyclic, Costs);
        problem.starts = vec!["s".into()];
        problem.terminals = ["s".into()].into();
        problem.options.elementary = false;
        assert!(PathSearchContext.search(&problem, &PathSearchBudget::default()).is_err());
        problem.options.maximum_steps = Some(3);
        let result = PathSearchContext.search(&problem, &PathSearchBudget::default()).unwrap();
        assert!(result.complete());
        assert_eq!(result.paths.len(), 4);
    }

    #[test]
    fn all_small_dags_match_an_independent_recursive_oracle() {
        fn enumerate(graph: &NetworkGraph<(), i32>, node: &NetworkNodeId, cost: i32,
            path: Vec<NetworkArcId>, paths: &mut Vec<(i32, Vec<NetworkArcId>)>)
        {
            if node.as_str() == "t" { paths.push((cost + 7, path.clone())); }
            for arc in graph.outgoing(node) {
                let mut next = path.clone();
                next.push(arc.id.clone());
                enumerate(graph, &arc.to, cost + arc.payload, next, paths);
            }
        }
        let nodes = ["s", "a", "b", "t"];
        let edges = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
        for mask in 0_u32..64 {
            let graph = NetworkGraph::new(nodes.map(|id| NetworkNode::new(id, ())).to_vec(),
                edges.iter().enumerate().filter(|(i, _)| mask & (1 << i) != 0)
                    .map(|(i, &(from, to))| NetworkArc::new(format!("e{i}"), nodes[from], nodes[to], i as i32 - 3)).collect()).unwrap();
            let mut problem = PathProblem::new(&graph, Costs);
            problem.starts = vec!["s".into()];
            problem.terminals = ["t".into()].into();
            let result = PathSearchContext.search(&problem, &PathSearchBudget::default()).unwrap();
            assert!(result.complete());
            let mut actual: Vec<_> = result.paths.iter().map(|path| (path.state, result.trace(path.label).unwrap())).collect();
            let mut expected = Vec::new();
            enumerate(&graph, &"s".into(), 0, Vec::new(), &mut expected);
            actual.sort();
            expected.sort();
            assert_eq!(actual, expected, "mask {mask}");
        }
    }

    struct Dominance { exact: bool, valid: bool }

    impl PathPolicy<(), i32, i32> for Dominance {
        fn start(&self, node: &NetworkNode<()>) -> Result<Option<i32>> { Costs.start(node) }
        fn extend(&self, state: &i32, arc: &NetworkArc<i32>, node: &NetworkNode<()>) -> Result<Option<i32>> { Costs.extend(state, arc, node) }
        fn finish(&self, trace: &PathTrace<'_, (), i32, i32>) -> Result<Option<i32>> { Costs.finish(trace) }
        fn dominates(&self, left: &i32, right: &i32) -> bool { left <= right }
        fn has_dominance(&self) -> bool { true }
        fn dominance_is_exact(&self) -> bool { self.exact }
        fn validate(&self, _: &PathTrace<'_, (), i32, i32>, _: &i32) -> Result<bool> { Ok(self.valid) }
    }

    #[test]
    fn per_node_budget_counts_retired_labels_not_only_the_active_dominance_pool() {
        let graph = NetworkGraph::new(["s", "a", "t"].map(|id| NetworkNode::new(id, ())).to_vec(),
            vec![NetworkArc::new("high", "s", "a", 10), NetworkArc::new("low", "s", "a", 1),
                NetworkArc::new("lowest", "s", "a", 0), NetworkArc::new("end", "a", "t", 0)]).unwrap();
        let mut problem = PathProblem::new(&graph, Dominance { exact: true, valid: true });
        problem.starts = vec!["s".into()];
        problem.terminals = ["t".into()].into();
        let result = PathSearchContext.search(&problem, &PathSearchBudget {
            maximum_labels_per_node: Some(2), ..Default::default()
        }).unwrap();
        assert_eq!(result.completion, PathSearchCompletion::BudgetLimit);
        assert_eq!(result.diagnostics.generated_labels, 3);
        assert_eq!(result.diagnostics.dominated_labels, 1);
        assert!(!result.labels[1].active);
        assert!(result.labels[2].active);
    }

    #[test]
    fn uncertified_pruning_validation_and_remaining_steps_are_honest() {
        let graph = graph();
        let mut problem = PathProblem::new(&graph, Dominance { exact: false, valid: true });
        problem.starts = vec!["s".into()];
        problem.terminals = ["t".into()].into();
        let result = PathSearchContext.search(&problem, &PathSearchBudget::default()).unwrap();
        assert_eq!(result.completion, PathSearchCompletion::UncertifiedDominance);
        problem.policy = Dominance { exact: true, valid: false };
        let result = PathSearchContext.search(&problem, &PathSearchBudget::default()).unwrap();
        assert_eq!(result.completion, PathSearchCompletion::ValidationRejected);
        assert!(result.paths.is_empty());

        let graph = NetworkGraph::new(["s", "a", "b", "t"].map(|id| NetworkNode::new(id, ())).to_vec(),
            vec![NetworkArc::new("short", "s", "a", 10), NetworkArc::new("detour", "s", "b", 0),
                NetworkArc::new("cheap", "b", "a", 0), NetworkArc::new("end", "a", "t", 0)]).unwrap();
        let mut problem = PathProblem::new(&graph, Dominance { exact: true, valid: true });
        problem.starts = vec!["s".into()];
        problem.terminals = ["t".into()].into();
        problem.options.elementary = false;
        problem.options.maximum_steps = Some(2);
        let result = PathSearchContext.search(&problem, &PathSearchBudget::default()).unwrap();
        assert!(result.complete());
        assert_eq!(result.paths.len(), 1);
        assert_eq!(result.paths[0].state, 17);
    }
}
