//! 分支定价树搜索 / Branch-and-price tree search
//!
//! 提供与具体模型解耦的多节点搜索骨架，节点求解由外部策略注入。
//! Provides a model-agnostic multi-node search skeleton with node solving injected by policy.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use ospf_rust_core::error::{CoreError, Result as CoreResult};
use ospf_rust_core::solver::{
    ProblemStatus, ProgressValue, SolveAttemptTrace, SolveHandle, SolveIssue,
    SolveProgressReporter, SolveProgressSnapshot, SolveReport, SolveSolution, SolveStage,
    SolveTrace, TerminationReason,
};

/// 分支方向 / Branch direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchDirection {
    /// 左分支 / Left branch
    Left,
    /// 右分支 / Right branch
    Right,
}

/// 分支决策 / Branch decision
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchDecision {
    /// 变量或列索引 / Variable or column index
    pub target_index: usize,
    /// 方向 / Direction
    pub direction: BranchDirection,
    /// 固定值 / Fixed value
    pub fixed_value: i8,
}

impl BranchDecision {
    /// 创建固定为 0 的决策 / Create a fix-to-zero decision
    pub fn zero(target_index: usize, direction: BranchDirection) -> Self {
        Self {
            target_index,
            direction,
            fixed_value: 0,
        }
    }

    /// 创建固定为 1 的决策 / Create a fix-to-one decision
    pub fn one(target_index: usize, direction: BranchDirection) -> Self {
        Self {
            target_index,
            direction,
            fixed_value: 1,
        }
    }
}

/// 分支节点状态 / Branch node status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchNodeStatus {
    /// 待求解 / Pending
    Pending,
    /// 已求解 / Solved
    Solved,
    /// 已剪枝 / Pruned
    Pruned,
    /// 不可行 / Infeasible
    Infeasible,
    /// 求解未完成 / Incomplete
    Incomplete,
}

/// 分支节点 / Branch node
#[derive(Debug, Clone)]
pub struct BranchNode {
    /// 节点 ID / Node id
    pub id: usize,
    /// 父节点 ID / Parent node id
    pub parent_id: Option<usize>,
    /// 深度 / Depth
    pub depth: usize,
    /// 决策路径 / Decision path
    pub decisions: Vec<BranchDecision>,
    /// 下界 / Lower bound
    pub lower_bound: f64,
    /// 已验证或从父节点继承的下界 / Verified or inherited lower bound
    pub certified_bound: Option<f64>,
    /// 上界 / Upper bound
    pub upper_bound: Option<f64>,
    /// 状态 / Status
    pub status: BranchNodeStatus,
}

impl BranchNode {
    /// 创建根节点 / Create root node
    pub fn root() -> Self {
        Self {
            id: 0,
            parent_id: None,
            depth: 0,
            decisions: Vec::new(),
            lower_bound: f64::NEG_INFINITY,
            certified_bound: None,
            upper_bound: None,
            status: BranchNodeStatus::Pending,
        }
    }

    /// 创建子节点 / Create child node
    pub fn child(id: usize, parent: &BranchNode, decision: BranchDecision) -> Self {
        let mut decisions = parent.decisions.clone();
        decisions.push(decision);
        Self {
            id,
            parent_id: Some(parent.id),
            depth: parent.depth + 1,
            decisions,
            lower_bound: parent.lower_bound,
            certified_bound: parent.certified_bound,
            upper_bound: None,
            status: BranchNodeStatus::Pending,
        }
    }
}

/// 节点结论 / Node conclusion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchNodeConclusion {
    /// 节点已证明不可行 / The node is proven infeasible
    Infeasible,
    /// 节点有 incumbent 但没有完整证明 / The node has an incumbent without a complete proof
    Feasible,
    /// 节点搜索未完成 / The node search is incomplete
    Incomplete,
    /// 节点已完成最优性证明 / The node has a complete optimality proof
    Optimal,
}

/// 节点求解输出 / Node solve output
#[derive(Debug, Clone)]
pub struct BranchNodeSolveOutput<S> {
    /// 统一节点求解报告 / Unified node solve report
    pub report: SolveReport<f64>,
    /// 节点数学结论 / Mathematical node conclusion
    pub conclusion: BranchNodeConclusion,
    /// 定价是否已经精确完成 / Whether pricing completed exactly
    pub pricing_complete: bool,
    /// 可用于全局剪枝的已验证下界 / Verified bound usable for global pruning
    pub certified_bound: Option<f64>,
    /// 节点下界 / Node lower bound
    pub lower_bound: f64,
    /// 可行整数解 / Feasible integer solution
    pub solution: Option<S>,
    /// 可行整数解目标值 / Feasible integer objective
    pub objective: Option<f64>,
    /// 分支目标索引 / Branch target index
    pub branch_target: Option<usize>,
}

impl<S> BranchNodeSolveOutput<S> {
    /// 创建带完整节点证明的测试/适配器输出 / Create a fully certifying test or adapter output
    pub fn certified(
        solution: S,
        objective: f64,
        bound: f64,
        branch_target: Option<usize>,
    ) -> CoreResult<Self> {
        let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(SolveSolution {
                value: None,
                values: Vec::new(),
                stable_values: Default::default(),
                objective: Some(objective),
                objective_value: Some(objective),
                dual_solution: None,
                quadratic_dual_solution: None,
                pool: Vec::new(),
            })
            .proof(ospf_rust_core::solver::SolveProof::optimality())
            .build()?;
        Self::from_report(
            report,
            Some(solution),
            Some(objective),
            branch_target,
            true,
            Some(bound),
        )
    }

    /// 创建不可行输出 / Create infeasible output
    pub fn infeasible() -> Self {
        let report = SolveReport::builder(ProblemStatus::Infeasible, TerminationReason::Completed)
            .build()
            .expect("infeasible node report must satisfy the shared contract");
        Self {
            report,
            conclusion: BranchNodeConclusion::Infeasible,
            pricing_complete: false,
            certified_bound: None,
            lower_bound: f64::INFINITY,
            solution: None,
            objective: None,
            branch_target: None,
        }
    }

    /// 创建带统一报告的节点输出 / Create node output from a unified report
    pub fn from_report(
        report: SolveReport<f64>,
        solution: Option<S>,
        objective: Option<f64>,
        branch_target: Option<usize>,
        pricing_complete: bool,
        certified_bound: Option<f64>,
    ) -> CoreResult<Self> {
        report.validate()?;
        let conclusion = match report.problem_status {
            ProblemStatus::Infeasible => BranchNodeConclusion::Infeasible,
            ProblemStatus::Feasible if report.is_optimal() => BranchNodeConclusion::Optimal,
            ProblemStatus::Feasible => BranchNodeConclusion::Feasible,
            ProblemStatus::Unknown
            | ProblemStatus::Unbounded
            | ProblemStatus::InfeasibleOrUnbounded => BranchNodeConclusion::Incomplete,
        };
        if matches!(conclusion, BranchNodeConclusion::Infeasible)
            && (solution.is_some() || objective.is_some())
        {
            return Err(CoreError::contract_error(
                "infeasible branch node cannot carry an incumbent",
            ));
        }
        if report.has_incumbent() != solution.is_some() {
            return Err(CoreError::contract_error(
                "branch node payload and SolveReport incumbent differ",
            ));
        }
        if certified_bound.is_some_and(|bound| !bound.is_finite()) {
            return Err(CoreError::contract_error(
                "branch node certified bound must be finite",
            ));
        }
        if let Some(objective) = objective {
            if !objective.is_finite() {
                return Err(CoreError::contract_error(
                    "branch node objective must be finite",
                ));
            }
            let report_objective = report
                .solution
                .as_ref()
                .and_then(|value| value.objective_value.or(value.objective));
            if report_objective
                .is_none_or(|report_objective| (report_objective - objective).abs() > 1e-8)
            {
                return Err(CoreError::contract_error(
                    "branch node objective differs from SolveReport",
                ));
            }
        }
        Ok(Self {
            report,
            conclusion,
            pricing_complete,
            lower_bound: certified_bound.unwrap_or(f64::NEG_INFINITY),
            certified_bound,
            solution,
            objective,
            branch_target,
        })
    }

    /// 判断节点是否不可行 / Check whether the node is infeasible
    pub fn is_infeasible(&self) -> bool {
        self.conclusion == BranchNodeConclusion::Infeasible
    }

    /// 判断节点是否可用于继续精确定价 / Check whether exact pricing may continue
    pub fn is_certifying_pricing(&self) -> bool {
        self.pricing_complete && self.report.is_optimal()
    }
}

/// 分支搜索顺序 / Branch search order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchSearchOrder {
    /// 深度优先 / Depth first
    DepthFirst,
    /// 最佳界优先 / Best bound first
    BestBound,
}

/// 分支搜索配置 / Branch search configuration
#[derive(Debug, Clone)]
pub struct BranchSearchConfig {
    /// 搜索顺序 / Search order
    pub order: BranchSearchOrder,
    /// 最大节点数 / Maximum node count
    pub max_nodes: usize,
    /// 最大深度 / Maximum depth
    pub max_depth: usize,
    /// gap 容差 / Gap tolerance
    pub gap_tolerance: f64,
    /// 时间限制 / Time limit
    pub time_limit: Duration,
    /// 外部取消句柄 / External cancellation handle
    pub cancellation_handle: Option<SolveHandle>,
}

impl Default for BranchSearchConfig {
    fn default() -> Self {
        Self {
            order: BranchSearchOrder::BestBound,
            max_nodes: 10_000,
            max_depth: 64,
            gap_tolerance: 1e-6,
            time_limit: Duration::from_secs(300),
            cancellation_handle: None,
        }
    }
}

/// 分支搜索结果 / Branch search result
#[derive(Debug, Clone)]
pub struct BranchSearchResult<S> {
    /// 统一树搜索报告 / Unified tree-search report
    pub report: SolveReport<f64>,
    /// 最优或 incumbent 解 / Best incumbent solution
    pub incumbent: Option<S>,
    /// incumbent 目标值 / Incumbent objective
    pub incumbent_objective: Option<f64>,
    /// 全局下界 / Global lower bound
    pub lower_bound: f64,
    /// 有效的全局下界 / Effective global lower bound
    pub certified_lower_bound: Option<f64>,
    /// 已处理节点数 / Processed node count
    pub processed_nodes: usize,
    /// 是否因 gap 收敛 / Whether converged by gap
    pub converged: bool,
    /// 每个已执行节点的 attempt 记录 / Attempt records for executed nodes
    pub attempts: Vec<SolveAttemptTrace>,
}

/// 强分支候选 / Strong branching candidate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrongBranchCandidate {
    /// 目标索引 / Target index
    pub target_index: usize,
}

/// 强分支策略 / Strong branching strategy
pub trait StrongBranchingStrategy<S>: Send + Sync {
    /// 选择分支目标 / Select branch target
    fn select_branch_target(
        &self,
        node: &BranchNode,
        output: &BranchNodeSolveOutput<S>,
    ) -> Option<usize>;
}

/// 默认强分支策略 / Default strong branching strategy
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopStrongBranching;

impl<S> StrongBranchingStrategy<S> for NoopStrongBranching {
    fn select_branch_target(
        &self,
        _node: &BranchNode,
        output: &BranchNodeSolveOutput<S>,
    ) -> Option<usize> {
        output.branch_target
    }
}

/// 分支切割动作 / Branch cut action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchCutAction {
    /// 继续节点处理 / Continue node processing
    Continue,
    /// 剪枝当前节点 / Prune current node
    Prune,
}

/// cut callback / Cut callback
pub trait BranchCutCallback<S>: Send + Sync {
    /// 节点求解后调用 / Called after node solve
    fn after_solve(&self, node: &BranchNode, output: &BranchNodeSolveOutput<S>) -> BranchCutAction;
}

/// 默认 cut callback / Default cut callback
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopBranchCutCallback;

impl<S> BranchCutCallback<S> for NoopBranchCutCallback {
    fn after_solve(
        &self,
        _node: &BranchNode,
        _output: &BranchNodeSolveOutput<S>,
    ) -> BranchCutAction {
        BranchCutAction::Continue
    }
}

/// 节点回调 / Node callback
pub trait BranchNodeCallback<S>: Send + Sync {
    /// 节点开始前 / Before node processing
    fn on_node_begin(&self, _node: &BranchNode) {}

    /// 节点完成后 / After node processing
    fn on_node_end(
        &self,
        _node: &BranchNode,
        _output: &BranchNodeSolveOutput<S>,
        _status: BranchNodeStatus,
    ) {
    }

    /// incumbent 更新后 / After incumbent update
    fn on_incumbent(&self, _node: &BranchNode, _solution: &S, _objective: f64) {}
}

/// 默认节点回调 / Default node callback
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopBranchNodeCallback;

impl<S> BranchNodeCallback<S> for NoopBranchNodeCallback {}

/// 分支搜索 hooks / Branch search hooks
pub struct BranchSearchHooks<'a, S> {
    /// 强分支策略 / Strong branching strategy
    pub strong_branching: &'a dyn StrongBranchingStrategy<S>,
    /// cut callback / Cut callback
    pub cut_callback: &'a dyn BranchCutCallback<S>,
    /// 节点回调 / Node callback
    pub node_callback: &'a dyn BranchNodeCallback<S>,
}

impl<'a, S> BranchSearchHooks<'a, S> {
    /// 创建 hooks / Create hooks
    pub fn new(
        strong_branching: &'a dyn StrongBranchingStrategy<S>,
        cut_callback: &'a dyn BranchCutCallback<S>,
        node_callback: &'a dyn BranchNodeCallback<S>,
    ) -> Self {
        Self {
            strong_branching,
            cut_callback,
            node_callback,
        }
    }
}

/// 分支定价树搜索器 / Branch-and-price tree searcher
#[derive(Debug, Clone)]
pub struct BranchAndPriceTreeSearch {
    /// 配置 / Configuration
    pub config: BranchSearchConfig,
}

impl BranchAndPriceTreeSearch {
    /// 创建搜索器 / Create searcher
    pub fn new(config: BranchSearchConfig) -> Self {
        Self { config }
    }

    /// 执行搜索 / Run search
    pub fn search<S, F, E>(&self, mut solve_node: F) -> Result<BranchSearchResult<S>, E>
    where
        S: Clone,
        F: FnMut(&BranchNode) -> Result<BranchNodeSolveOutput<S>, E>,
    {
        let strong_branching = NoopStrongBranching;
        let cut_callback = NoopBranchCutCallback;
        let node_callback = NoopBranchNodeCallback;
        self.search_with_hooks(
            |node| solve_node(node),
            BranchSearchHooks::new(&strong_branching, &cut_callback, &node_callback),
        )
    }

    /// 执行带统一 progress reporter 的搜索 / Run the search with a unified progress reporter
    ///
    /// reporter 错误会形成结构化 `BackendFailure` 报告，而不是被忽略。
    /// Reporter failures become a structured `BackendFailure` report instead of being ignored.
    pub fn search_with_progress<S, F, E>(
        &self,
        mut solve_node: F,
        attempt_id: impl Into<String>,
        reporter: SolveProgressReporter,
    ) -> Result<BranchSearchResult<S>, E>
    where
        S: Clone,
        F: FnMut(&BranchNode) -> Result<BranchNodeSolveOutput<S>, E>,
    {
        let strong_branching = NoopStrongBranching;
        let cut_callback = NoopBranchCutCallback;
        let node_callback = NoopBranchNodeCallback;
        self.search_with_hooks_and_progress(
            |node| solve_node(node),
            BranchSearchHooks::new(&strong_branching, &cut_callback, &node_callback),
            attempt_id,
            reporter,
        )
    }

    /// 执行带扩展点的搜索 / Run search with extension points
    pub fn search_with_hooks<S, F, E>(
        &self,
        solve_node: F,
        hooks: BranchSearchHooks<'_, S>,
    ) -> Result<BranchSearchResult<S>, E>
    where
        S: Clone,
        F: FnMut(&BranchNode) -> Result<BranchNodeSolveOutput<S>, E>,
    {
        self.search_with_hooks_internal(solve_node, hooks, None)
    }

    /// 执行带 hooks 和统一 progress reporter 的搜索 / Run search with hooks and progress reporter
    pub fn search_with_hooks_and_progress<S, F, E>(
        &self,
        solve_node: F,
        hooks: BranchSearchHooks<'_, S>,
        attempt_id: impl Into<String>,
        reporter: SolveProgressReporter,
    ) -> Result<BranchSearchResult<S>, E>
    where
        S: Clone,
        F: FnMut(&BranchNode) -> Result<BranchNodeSolveOutput<S>, E>,
    {
        self.search_with_hooks_internal(solve_node, hooks, Some((attempt_id.into(), reporter)))
    }

    fn search_with_hooks_internal<S, F, E>(
        &self,
        mut solve_node: F,
        hooks: BranchSearchHooks<'_, S>,
        progress: Option<(String, SolveProgressReporter)>,
    ) -> Result<BranchSearchResult<S>, E>
    where
        S: Clone,
        F: FnMut(&BranchNode) -> Result<BranchNodeSolveOutput<S>, E>,
    {
        let begin = Instant::now();
        let mut queue = VecDeque::from([BranchNode::root()]);
        let mut next_node_id = 1usize;
        let mut processed_nodes = 0usize;
        let mut incumbent: Option<S> = None;
        let mut incumbent_objective: Option<f64> = None;
        let mut converged = false;
        let mut all_nodes_certified_infeasible = true;
        let mut all_bounds_certified = true;
        let mut termination_reason = TerminationReason::Completed;
        let mut pruned_nodes = 0usize;
        let mut attempts = Vec::new();
        let mut progress_issue: Option<SolveIssue> = None;

        while let Some(mut node) = self.pop_node(&mut queue) {
            if self
                .config
                .cancellation_handle
                .as_ref()
                .is_some_and(SolveHandle::is_cancelled)
            {
                termination_reason = TerminationReason::Cancelled;
                queue.push_front(node);
                break;
            }
            if processed_nodes >= self.config.max_nodes {
                termination_reason = TerminationReason::NodeLimit;
                queue.push_front(node);
                break;
            }
            if begin.elapsed() >= self.config.time_limit {
                termination_reason = TerminationReason::TimeLimit;
                queue.push_front(node);
                break;
            }

            if let Some(issue) = Self::publish_progress(
                progress.as_ref(),
                &node,
                processed_nodes,
                self.config.max_nodes,
                begin.elapsed(),
                incumbent_objective,
                self.active_lower_bound(&queue),
                false,
                ProgressValue::known(0.0).expect("constant progress is valid"),
            ) {
                progress_issue = Some(issue);
                termination_reason = TerminationReason::BackendFailure;
                queue.push_front(node);
                break;
            }

            processed_nodes += 1;
            hooks.node_callback.on_node_begin(&node);
            let output = solve_node(&node)?;
            attempts.push(SolveAttemptTrace::from_report_with_parent(
                format!("branch-and-price/node/{}", node.id),
                Some("branch-and-price".to_owned()),
                output.report.provenance.clone(),
                &output.report,
                output.report.statistics.solve_time,
            ));

            if output.report.termination_reason != TerminationReason::Completed {
                termination_reason = output.report.termination_reason;
            }

            if output.is_infeasible() {
                node.status = BranchNodeStatus::Infeasible;
                all_nodes_certified_infeasible &=
                    ospf_rust_core::solver::require_infeasibility_certificate(&output.report)
                        .is_ok();
                hooks.node_callback.on_node_end(&node, &output, node.status);
                if let Some(issue) = Self::publish_progress(
                    progress.as_ref(),
                    &node,
                    processed_nodes,
                    self.config.max_nodes,
                    begin.elapsed(),
                    incumbent_objective,
                    self.active_lower_bound(&queue),
                    false,
                    ProgressValue::known(100.0).expect("constant progress is valid"),
                ) {
                    progress_issue = Some(issue);
                    termination_reason = TerminationReason::BackendFailure;
                    break;
                }
                continue;
            }

            all_nodes_certified_infeasible = false;
            node.certified_bound = output.certified_bound.or(node.certified_bound);
            if let Some(bound) = node.certified_bound {
                node.lower_bound = bound;
            } else {
                all_bounds_certified = false;
            }

            if hooks.cut_callback.after_solve(&node, &output) == BranchCutAction::Prune {
                node.status = BranchNodeStatus::Pruned;
                pruned_nodes += 1;
                hooks.node_callback.on_node_end(&node, &output, node.status);
                if let Some(issue) = Self::publish_progress(
                    progress.as_ref(),
                    &node,
                    processed_nodes,
                    self.config.max_nodes,
                    begin.elapsed(),
                    incumbent_objective,
                    self.active_lower_bound(&queue),
                    false,
                    ProgressValue::known(100.0).expect("constant progress is valid"),
                ) {
                    progress_issue = Some(issue);
                    termination_reason = TerminationReason::BackendFailure;
                    break;
                }
                continue;
            }

            if let (Some(solution), Some(objective)) = (output.solution.as_ref(), output.objective)
                && incumbent_objective
                    .map(|value| objective < value)
                    .unwrap_or(true)
            {
                hooks.node_callback.on_incumbent(&node, solution, objective);
                incumbent = Some(solution.clone());
                incumbent_objective = Some(objective);
            }

            if node
                .certified_bound
                .is_some_and(|bound| self.can_prune(bound, incumbent_objective))
            {
                node.status = BranchNodeStatus::Pruned;
                pruned_nodes += 1;
                hooks.node_callback.on_node_end(&node, &output, node.status);
                if let Some(issue) = Self::publish_progress(
                    progress.as_ref(),
                    &node,
                    processed_nodes,
                    self.config.max_nodes,
                    begin.elapsed(),
                    incumbent_objective,
                    self.active_lower_bound(&queue),
                    false,
                    ProgressValue::known(100.0).expect("constant progress is valid"),
                ) {
                    progress_issue = Some(issue);
                    termination_reason = TerminationReason::BackendFailure;
                    break;
                }
                continue;
            }

            let global_lower_bound = node
                .certified_bound
                .into_iter()
                .chain(self.active_lower_bound(&queue))
                .min_by(f64::total_cmp);
            if self.gap_closed(global_lower_bound, incumbent_objective)
                && output.is_certifying_pricing()
            {
                converged = true;
                node.status = BranchNodeStatus::Solved;
                hooks.node_callback.on_node_end(&node, &output, node.status);
                break;
            }

            if node.depth >= self.config.max_depth {
                node.status = BranchNodeStatus::Pruned;
                pruned_nodes += 1;
                hooks.node_callback.on_node_end(&node, &output, node.status);
                continue;
            }

            if output.is_certifying_pricing()
                && let Some(target) = hooks.strong_branching.select_branch_target(&node, &output)
            {
                let left = BranchNode::child(
                    next_node_id,
                    &node,
                    BranchDecision::zero(target, BranchDirection::Left),
                );
                next_node_id += 1;
                let right = BranchNode::child(
                    next_node_id,
                    &node,
                    BranchDecision::one(target, BranchDirection::Right),
                );
                next_node_id += 1;
                queue.push_back(left);
                queue.push_back(right);
            }

            node.status = if output.conclusion == BranchNodeConclusion::Incomplete {
                BranchNodeStatus::Incomplete
            } else {
                BranchNodeStatus::Solved
            };
            hooks.node_callback.on_node_end(&node, &output, node.status);
            if let Some(issue) = Self::publish_progress(
                progress.as_ref(),
                &node,
                processed_nodes,
                self.config.max_nodes,
                begin.elapsed(),
                incumbent_objective,
                self.active_lower_bound(&queue),
                false,
                ProgressValue::known(100.0).expect("constant progress is valid"),
            ) {
                progress_issue = Some(issue);
                termination_reason = TerminationReason::BackendFailure;
                break;
            }
        }

        let global_lower_bound = self.active_lower_bound(&queue);
        let mut globally_proven =
            queue.is_empty() && incumbent.is_some() && all_bounds_certified && converged;
        if progress_issue.is_none()
            && queue.is_empty()
            && incumbent.is_none()
            && all_nodes_certified_infeasible
        {
            termination_reason = TerminationReason::Completed;
        }
        let mut report_status = if incumbent.is_some() {
            ProblemStatus::Feasible
        } else if queue.is_empty() && all_nodes_certified_infeasible {
            ProblemStatus::Infeasible
        } else {
            ProblemStatus::Unknown
        };
        if let Some(issue) = Self::publish_progress(
            progress.as_ref(),
            &BranchNode::root(),
            processed_nodes,
            self.config.max_nodes,
            begin.elapsed(),
            incumbent_objective,
            global_lower_bound,
            true,
            ProgressValue::known(100.0).expect("constant progress is valid"),
        ) {
            progress_issue = Some(issue);
            termination_reason = TerminationReason::BackendFailure;
            globally_proven = false;
            report_status = if incumbent.is_some() {
                ProblemStatus::Feasible
            } else {
                ProblemStatus::Unknown
            };
        }
        if progress_issue.is_some() {
            globally_proven = false;
            if report_status == ProblemStatus::Infeasible {
                report_status = ProblemStatus::Unknown;
            }
        }
        let mut report_builder = SolveReport::builder(report_status, termination_reason);
        let mut diagnostics = ospf_rust_core::solver::SolveDiagnostics::default();
        if let Some(issue) = progress_issue {
            diagnostics.issues.push(issue);
        }
        diagnostics.extensions.insert(
            "branch-and-price.attemptCount".to_owned(),
            attempts.len().to_string(),
        );
        if termination_reason == TerminationReason::Cancelled
            && let Some(cancellation) = self
                .config
                .cancellation_handle
                .as_ref()
                .and_then(SolveHandle::cancellation)
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
        report_builder = report_builder.diagnostics(diagnostics);
        if let Some(objective) = incumbent_objective {
            report_builder = report_builder.solution(SolveSolution {
                value: None,
                values: Vec::new(),
                stable_values: Default::default(),
                objective: Some(objective),
                objective_value: Some(objective),
                dual_solution: None,
                quadratic_dual_solution: None,
                pool: Vec::new(),
            });
        }
        if globally_proven {
            report_builder = report_builder.proof(ospf_rust_core::solver::SolveProof::optimality());
        }
        let relative_gap = match (incumbent_objective, global_lower_bound) {
            (Some(objective), Some(bound)) => {
                Some((objective - bound).abs() / objective.abs().max(1.0))
            }
            _ => None,
        };
        report_builder = report_builder.trace(SolveTrace {
            nodes_explored: processed_nodes,
            nodes_pruned: pruned_nodes,
            active_nodes: queue.len(),
            global_lower_bound,
            upper_bound: incumbent_objective,
            relative_gap,
            total_iterations: processed_nodes,
            pricing_calls: attempts.len(),
            generated_columns: 0,
            elapsed: begin.elapsed(),
            iteration_snapshots: Vec::new(),
        });
        let report = report_builder
            .build()
            .expect("branch search report must satisfy the shared contract");

        Ok(BranchSearchResult {
            report,
            incumbent,
            incumbent_objective,
            lower_bound: global_lower_bound.unwrap_or(f64::NEG_INFINITY),
            certified_lower_bound: global_lower_bound,
            processed_nodes,
            converged,
            attempts,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn publish_progress(
        progress: Option<&(String, SolveProgressReporter)>,
        node: &BranchNode,
        processed_nodes: usize,
        max_nodes: usize,
        elapsed: Duration,
        objective: Option<f64>,
        best_bound: Option<f64>,
        terminal: bool,
        stage_progress: ProgressValue,
    ) -> Option<SolveIssue> {
        let (attempt_id, reporter) = progress?;
        let overall_progress = ProgressValue::known(
            (processed_nodes as f64 / max_nodes.max(1) as f64 * 100.0).min(100.0),
        )
        .expect("bounded progress is valid");
        let relative_gap = match (objective, best_bound) {
            (Some(objective), Some(bound)) => {
                Some((objective - bound).abs() / objective.abs().max(1.0))
            }
            _ => None,
        };
        let stage = if terminal {
            SolveStage::Completed
        } else {
            SolveStage::Combinatorial
        };
        let path = if terminal {
            vec!["branch-and-price".to_owned(), "completed".to_owned()]
        } else {
            vec![
                "branch-and-price".to_owned(),
                format!("node/{}", node.id),
                "solve".to_owned(),
            ]
        };
        let snapshot = SolveProgressSnapshot::new(
            attempt_id.as_str(),
            stage,
            path,
            stage_progress,
            overall_progress,
            elapsed,
            objective,
            best_bound,
            relative_gap,
            terminal,
        )
        .map_err(|error| SolveIssue::new("ProgressContract", error.to_string()));
        match snapshot {
            Ok(snapshot) => reporter(&snapshot)
                .err()
                .map(|error| SolveIssue::new("ProgressReporterFailed", error.to_string())),
            Err(issue) => Some(issue),
        }
    }

    fn pop_node(&self, queue: &mut VecDeque<BranchNode>) -> Option<BranchNode> {
        match self.config.order {
            BranchSearchOrder::DepthFirst => queue.pop_back(),
            BranchSearchOrder::BestBound => {
                let (index, _) = queue.iter().enumerate().min_by(|(_, a), (_, b)| {
                    a.certified_bound
                        .unwrap_or(f64::INFINITY)
                        .total_cmp(&b.certified_bound.unwrap_or(f64::INFINITY))
                })?;
                queue.remove(index)
            }
        }
    }

    fn active_lower_bound(&self, queue: &VecDeque<BranchNode>) -> Option<f64> {
        queue
            .iter()
            .filter_map(|node| node.certified_bound)
            .min_by(f64::total_cmp)
    }

    fn can_prune(&self, lower_bound: f64, incumbent_objective: Option<f64>) -> bool {
        incumbent_objective
            .map(|objective| lower_bound >= objective - self.config.gap_tolerance)
            .unwrap_or(false)
    }

    fn gap_closed(&self, lower_bound: Option<f64>, incumbent_objective: Option<f64>) -> bool {
        let Some(lower_bound) = lower_bound else {
            return false;
        };
        let Some(objective) = incumbent_objective else {
            return false;
        };
        if objective.abs() < f64::EPSILON {
            (objective - lower_bound).abs() <= self.config.gap_tolerance
        } else {
            ((objective - lower_bound).abs() / objective.abs()) <= self.config.gap_tolerance
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn optimal_output<S>(
        solution: S,
        objective: f64,
        bound: f64,
        branch_target: Option<usize>,
    ) -> BranchNodeSolveOutput<S> {
        let report = SolveReport::builder(ProblemStatus::Feasible, TerminationReason::Completed)
            .solution(SolveSolution {
                value: None,
                values: Vec::new(),
                stable_values: Default::default(),
                objective: Some(objective),
                objective_value: Some(objective),
                dual_solution: None,
                quadratic_dual_solution: None,
                pool: Vec::new(),
            })
            .proof(ospf_rust_core::solver::SolveProof::optimality())
            .build()
            .expect("test report should be valid");
        BranchNodeSolveOutput::from_report(
            report,
            Some(solution),
            Some(objective),
            branch_target,
            true,
            Some(bound),
        )
        .expect("test node output should be valid")
    }

    #[test]
    fn test_branch_node_child_keeps_decision_path() {
        let root = BranchNode::root();
        let child = BranchNode::child(1, &root, BranchDecision::one(3, BranchDirection::Left));

        assert_eq!(child.parent_id, Some(0));
        assert_eq!(child.depth, 1);
        assert_eq!(child.decisions.len(), 1);
        assert_eq!(child.decisions[0].target_index, 3);
    }

    #[test]
    fn test_tree_search_processes_multiple_nodes_and_updates_incumbent() {
        let search = BranchAndPriceTreeSearch::new(BranchSearchConfig {
            order: BranchSearchOrder::DepthFirst,
            max_nodes: 8,
            max_depth: 2,
            gap_tolerance: 1e-9,
            time_limit: Duration::from_secs(1),
            cancellation_handle: None,
        });

        let result = search
            .search(|node| {
                if node.depth == 0 {
                    Ok::<_, std::convert::Infallible>(optimal_output(0usize, 100.0, 0.0, Some(0)))
                } else {
                    Ok::<_, std::convert::Infallible>(optimal_output(
                        node.id,
                        10.0 + node.id as f64,
                        1.0,
                        None,
                    ))
                }
            })
            .expect("tree search should succeed");

        assert!(result.processed_nodes >= 2);
        assert!(result.incumbent.is_some());
        assert!(result.incumbent_objective.is_some());
    }

    #[derive(Debug, Clone, Default)]
    struct TestStrongBranching;

    impl StrongBranchingStrategy<usize> for TestStrongBranching {
        fn select_branch_target(
            &self,
            node: &BranchNode,
            output: &BranchNodeSolveOutput<usize>,
        ) -> Option<usize> {
            if node.depth == 0 {
                Some(7)
            } else {
                output.branch_target
            }
        }
    }

    #[derive(Debug, Clone, Default)]
    struct TestCutCallback;

    impl BranchCutCallback<usize> for TestCutCallback {
        fn after_solve(
            &self,
            node: &BranchNode,
            _output: &BranchNodeSolveOutput<usize>,
        ) -> BranchCutAction {
            if node
                .decisions
                .iter()
                .any(|decision| decision.target_index == 7 && decision.fixed_value == 0)
            {
                BranchCutAction::Prune
            } else {
                BranchCutAction::Continue
            }
        }
    }

    #[derive(Debug, Clone, Default)]
    struct TraceNodeCallback {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl BranchNodeCallback<usize> for TraceNodeCallback {
        fn on_node_begin(&self, node: &BranchNode) {
            self.events
                .lock()
                .unwrap()
                .push(format!("begin:{}", node.id));
        }

        fn on_node_end(
            &self,
            node: &BranchNode,
            _output: &BranchNodeSolveOutput<usize>,
            status: BranchNodeStatus,
        ) {
            self.events
                .lock()
                .unwrap()
                .push(format!("end:{}:{:?}", node.id, status));
        }

        fn on_incumbent(&self, node: &BranchNode, _solution: &usize, objective: f64) {
            self.events
                .lock()
                .unwrap()
                .push(format!("incumbent:{}:{:.1}", node.id, objective));
        }
    }

    #[test]
    fn test_tree_search_hooks_drive_branching_cut_and_node_trace() {
        let search = BranchAndPriceTreeSearch::new(BranchSearchConfig {
            order: BranchSearchOrder::DepthFirst,
            max_nodes: 8,
            max_depth: 2,
            gap_tolerance: 1e-9,
            time_limit: Duration::from_secs(1),
            cancellation_handle: None,
        });
        let strong_branching = TestStrongBranching;
        let cut_callback = TestCutCallback;
        let node_callback = TraceNodeCallback::default();
        let trace = node_callback.events.clone();

        let result = search
            .search_with_hooks(
                |node| {
                    if node.depth == 0 {
                        Ok::<_, std::convert::Infallible>(optimal_output(
                            0usize,
                            100.0,
                            0.0,
                            Some(1),
                        ))
                    } else {
                        Ok::<_, std::convert::Infallible>(optimal_output(
                            node.id,
                            node.id as f64,
                            1.0,
                            None,
                        ))
                    }
                },
                BranchSearchHooks::new(&strong_branching, &cut_callback, &node_callback),
            )
            .expect("hooked tree search should succeed");

        let trace = trace.lock().unwrap().clone();
        assert!(result.processed_nodes >= 2);
        assert_eq!(result.incumbent, Some(2));
        assert!(trace.iter().any(|event| event == "begin:0"));
        assert!(trace.iter().any(|event| event == "end:1:Pruned"));
        assert!(trace.iter().any(|event| event.starts_with("incumbent:")));
    }

    #[test]
    fn non_optimal_node_does_not_branch_or_claim_optimality() {
        let search = BranchAndPriceTreeSearch::new(BranchSearchConfig {
            max_nodes: 8,
            max_depth: 2,
            time_limit: Duration::from_secs(1),
            ..BranchSearchConfig::default()
        });
        let calls = Arc::new(Mutex::new(0usize));
        let calls_for_solver = Arc::clone(&calls);
        let result = search
            .search(|_node| {
                *calls_for_solver.lock().unwrap() += 1;
                let report =
                    SolveReport::builder(ProblemStatus::Feasible, TerminationReason::TimeLimit)
                        .solution(SolveSolution {
                            value: None,
                            values: Vec::new(),
                            stable_values: Default::default(),
                            objective: Some(5.0),
                            objective_value: Some(5.0),
                            dual_solution: None,
                            quadratic_dual_solution: None,
                            pool: Vec::new(),
                        })
                        .build()
                        .expect("time-limited report should be valid");
                Ok::<_, std::convert::Infallible>(
                    BranchNodeSolveOutput::from_report(
                        report,
                        Some(0usize),
                        Some(5.0),
                        Some(1),
                        false,
                        None,
                    )
                    .expect("time-limited node should be valid"),
                )
            })
            .expect("tree search should succeed");

        assert_eq!(*calls.lock().unwrap(), 1);
        assert!(!result.converged);
        assert_eq!(result.report.problem_status, ProblemStatus::Feasible);
        assert_eq!(
            result.report.termination_reason,
            TerminationReason::TimeLimit
        );
        assert!(result.report.proof.is_none());
    }

    #[test]
    fn unverified_infeasible_node_does_not_become_global_infeasibility_proof() {
        let search = BranchAndPriceTreeSearch::new(BranchSearchConfig {
            max_nodes: 1,
            time_limit: Duration::from_secs(1),
            ..BranchSearchConfig::default()
        });
        let result = search
            .search(|_node| {
                Ok::<_, std::convert::Infallible>(BranchNodeSolveOutput::<usize>::infeasible())
            })
            .expect("tree search should succeed");

        assert_eq!(result.report.problem_status, ProblemStatus::Unknown);
        assert!(result.report.proof.is_none());
        assert_eq!(result.processed_nodes, 1);
    }

    #[test]
    fn cancellation_stops_before_node_solver_and_preserves_origin() {
        let handle = SolveHandle::new();
        assert!(handle.cancel(ospf_rust_core::solver::CancellationOrigin::User));
        let search = BranchAndPriceTreeSearch::new(BranchSearchConfig {
            cancellation_handle: Some(handle),
            ..BranchSearchConfig::default()
        });
        let calls = Arc::new(Mutex::new(0usize));
        let calls_for_solver = Arc::clone(&calls);
        let result = search
            .search(|_node| {
                *calls_for_solver.lock().unwrap() += 1;
                Ok::<_, std::convert::Infallible>(BranchNodeSolveOutput::<usize>::infeasible())
            })
            .expect("cancelled search should return a report");

        assert_eq!(*calls.lock().unwrap(), 0);
        assert_eq!(result.processed_nodes, 0);
        assert_eq!(
            result.report.termination_reason,
            TerminationReason::Cancelled
        );
        assert_eq!(
            result
                .report
                .diagnostics
                .extensions
                .get("cancellation.origin")
                .map(String::as_str),
            Some("USER")
        );
    }

    #[test]
    fn node_limit_keeps_incumbent_without_inventing_optimality_proof() {
        let search = BranchAndPriceTreeSearch::new(BranchSearchConfig {
            max_nodes: 1,
            max_depth: 3,
            time_limit: Duration::from_secs(1),
            ..BranchSearchConfig::default()
        });
        let result = search
            .search(|_node| {
                Ok::<_, std::convert::Infallible>(optimal_output(1usize, 3.0, 0.0, Some(1)))
            })
            .expect("node-limited search should return a report");

        assert_eq!(result.report.problem_status, ProblemStatus::Feasible);
        assert_eq!(
            result.report.termination_reason,
            TerminationReason::NodeLimit
        );
        assert!(result.report.has_incumbent());
        assert!(result.report.proof.is_none());
    }

    #[test]
    fn node_limit_without_incumbent_remains_unknown() {
        let search = BranchAndPriceTreeSearch::new(BranchSearchConfig {
            max_nodes: 0,
            time_limit: Duration::from_secs(1),
            ..BranchSearchConfig::default()
        });
        let result =
            search
                .search(
                    |_node| -> std::result::Result<
                        BranchNodeSolveOutput<usize>,
                        std::convert::Infallible,
                    > {
                        panic!("a zero-node limit must not start the node solver")
                    },
                )
                .expect("node-limited search should return a report");

        assert_eq!(result.report.problem_status, ProblemStatus::Unknown);
        assert_eq!(
            result.report.termination_reason,
            TerminationReason::NodeLimit
        );
        assert!(!result.report.has_incumbent());
        assert!(result.report.proof.is_none());
    }

    #[test]
    fn progress_and_node_attempt_trace_are_reported() {
        let snapshots = Arc::new(Mutex::new(Vec::<SolveProgressSnapshot>::new()));
        let snapshots_for_reporter = Arc::clone(&snapshots);
        let reporter: SolveProgressReporter = Arc::new(move |snapshot| {
            snapshots_for_reporter
                .lock()
                .expect("progress snapshot lock")
                .push(snapshot.clone());
            Ok(())
        });
        let search = BranchAndPriceTreeSearch::new(BranchSearchConfig::default());
        let result = search
            .search_with_progress(
                |_node| Ok::<_, std::convert::Infallible>(optimal_output(1usize, 1.0, 1.0, None)),
                "gantt-attempt-1",
                reporter,
            )
            .expect("progress-enabled search should succeed");

        let snapshots = snapshots.lock().expect("progress snapshot lock");
        assert!(snapshots.iter().any(|snapshot| {
            snapshot.attempt_id == "gantt-attempt-1"
                && snapshot.stage == SolveStage::Combinatorial
                && snapshot.stage_path == ["branch-and-price", "node/0", "solve"]
        }));
        assert!(
            snapshots
                .iter()
                .any(|snapshot| { snapshot.terminal && snapshot.stage == SolveStage::Completed })
        );
        assert_eq!(result.attempts.len(), 1);
        assert_eq!(result.attempts[0].attempt_id, "branch-and-price/node/0");
        assert_eq!(result.report.trace.nodes_explored, 1);
        assert_eq!(result.report.trace.pricing_calls, 1);
    }

    #[test]
    fn progress_reporter_failure_is_structured_and_cannot_claim_optimality() {
        let reporter: SolveProgressReporter =
            Arc::new(|_| Err(CoreError::contract_error("progress sink unavailable")));
        let search = BranchAndPriceTreeSearch::new(BranchSearchConfig::default());
        let result = search
            .search_with_progress(
                |_node| Ok::<_, std::convert::Infallible>(optimal_output(1usize, 1.0, 1.0, None)),
                "gantt-attempt-error",
                reporter,
            )
            .expect("progress failure should be represented in the report");

        assert_eq!(
            result.report.termination_reason,
            TerminationReason::BackendFailure
        );
        assert!(!result.report.is_optimal());
        assert!(
            result
                .report
                .diagnostics
                .issues
                .iter()
                .any(|issue| issue.code == "ProgressReporterFailed")
        );
    }
}
