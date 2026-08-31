//! 分支定价树搜索 / Branch-and-price tree search
//!
//! 提供与具体模型解耦的多节点搜索骨架，节点求解由外部策略注入。
//! Provides a model-agnostic multi-node search skeleton with node solving injected by policy.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

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
            upper_bound: None,
            status: BranchNodeStatus::Pending,
        }
    }
}

/// 节点求解输出 / Node solve output
#[derive(Debug, Clone)]
pub struct BranchNodeSolveOutput<S> {
    /// 节点下界 / Node lower bound
    pub lower_bound: f64,
    /// 可行整数解 / Feasible integer solution
    pub solution: Option<S>,
    /// 可行整数解目标值 / Feasible integer objective
    pub objective: Option<f64>,
    /// 分支目标索引 / Branch target index
    pub branch_target: Option<usize>,
    /// 是否不可行 / Whether infeasible
    pub infeasible: bool,
}

impl<S> BranchNodeSolveOutput<S> {
    /// 创建不可行输出 / Create infeasible output
    pub fn infeasible() -> Self {
        Self {
            lower_bound: f64::INFINITY,
            solution: None,
            objective: None,
            branch_target: None,
            infeasible: true,
        }
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
}

impl Default for BranchSearchConfig {
    fn default() -> Self {
        Self {
            order: BranchSearchOrder::BestBound,
            max_nodes: 10_000,
            max_depth: 64,
            gap_tolerance: 1e-6,
            time_limit: Duration::from_secs(300),
        }
    }
}

/// 分支搜索结果 / Branch search result
#[derive(Debug, Clone)]
pub struct BranchSearchResult<S> {
    /// 最优或 incumbent 解 / Best incumbent solution
    pub incumbent: Option<S>,
    /// incumbent 目标值 / Incumbent objective
    pub incumbent_objective: Option<f64>,
    /// 全局下界 / Global lower bound
    pub lower_bound: f64,
    /// 已处理节点数 / Processed node count
    pub processed_nodes: usize,
    /// 是否因 gap 收敛 / Whether converged by gap
    pub converged: bool,
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
    pub fn search<S, F>(&self, mut solve_node: F) -> BranchSearchResult<S>
    where
        S: Clone,
        F: FnMut(&BranchNode) -> BranchNodeSolveOutput<S>,
    {
        let strong_branching = NoopStrongBranching;
        let cut_callback = NoopBranchCutCallback;
        let node_callback = NoopBranchNodeCallback;
        self.search_with_hooks(
            |node| solve_node(node),
            BranchSearchHooks::new(&strong_branching, &cut_callback, &node_callback),
        )
    }

    /// 执行带扩展点的搜索 / Run search with extension points
    pub fn search_with_hooks<S, F>(
        &self,
        mut solve_node: F,
        hooks: BranchSearchHooks<'_, S>,
    ) -> BranchSearchResult<S>
    where
        S: Clone,
        F: FnMut(&BranchNode) -> BranchNodeSolveOutput<S>,
    {
        let begin = Instant::now();
        let mut queue = VecDeque::from([BranchNode::root()]);
        let mut next_node_id = 1usize;
        let mut processed_nodes = 0usize;
        let mut incumbent: Option<S> = None;
        let mut incumbent_objective: Option<f64> = None;
        let mut global_lower_bound = f64::NEG_INFINITY;
        let mut converged = false;

        while let Some(mut node) = self.pop_node(&mut queue) {
            if processed_nodes >= self.config.max_nodes || begin.elapsed() >= self.config.time_limit
            {
                break;
            }

            processed_nodes += 1;
            hooks.node_callback.on_node_begin(&node);
            let output = solve_node(&node);

            if output.infeasible {
                node.status = BranchNodeStatus::Infeasible;
                hooks.node_callback.on_node_end(&node, &output, node.status);
                continue;
            }

            node.lower_bound = output.lower_bound;
            global_lower_bound = global_lower_bound.max(output.lower_bound);

            if hooks.cut_callback.after_solve(&node, &output) == BranchCutAction::Prune {
                node.status = BranchNodeStatus::Pruned;
                hooks.node_callback.on_node_end(&node, &output, node.status);
                continue;
            }

            if let (Some(solution), Some(objective)) = (output.solution.as_ref(), output.objective)
            {
                if incumbent_objective
                    .map(|value| objective < value)
                    .unwrap_or(true)
                {
                    hooks.node_callback.on_incumbent(&node, solution, objective);
                    incumbent = Some(solution.clone());
                    incumbent_objective = Some(objective);
                }
            }

            if self.can_prune(output.lower_bound, incumbent_objective) {
                node.status = BranchNodeStatus::Pruned;
                hooks.node_callback.on_node_end(&node, &output, node.status);
                continue;
            }

            if self.gap_closed(global_lower_bound, incumbent_objective) {
                converged = true;
                node.status = BranchNodeStatus::Solved;
                hooks.node_callback.on_node_end(&node, &output, node.status);
                break;
            }

            if node.depth >= self.config.max_depth {
                node.status = BranchNodeStatus::Pruned;
                hooks.node_callback.on_node_end(&node, &output, node.status);
                continue;
            }

            if let Some(target) = hooks.strong_branching.select_branch_target(&node, &output) {
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

            node.status = BranchNodeStatus::Solved;
            hooks.node_callback.on_node_end(&node, &output, node.status);
        }

        BranchSearchResult {
            incumbent,
            incumbent_objective,
            lower_bound: global_lower_bound,
            processed_nodes,
            converged,
        }
    }

    fn pop_node(&self, queue: &mut VecDeque<BranchNode>) -> Option<BranchNode> {
        match self.config.order {
            BranchSearchOrder::DepthFirst => queue.pop_back(),
            BranchSearchOrder::BestBound => {
                let (index, _) = queue
                    .iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.lower_bound.total_cmp(&b.lower_bound))?;
                queue.remove(index)
            }
        }
    }

    fn can_prune(&self, lower_bound: f64, incumbent_objective: Option<f64>) -> bool {
        incumbent_objective
            .map(|objective| lower_bound >= objective - self.config.gap_tolerance)
            .unwrap_or(false)
    }

    fn gap_closed(&self, lower_bound: f64, incumbent_objective: Option<f64>) -> bool {
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
        });

        let result = search.search(|node| {
            if node.depth == 0 {
                BranchNodeSolveOutput {
                    lower_bound: 0.0,
                    solution: None,
                    objective: None,
                    branch_target: Some(0),
                    infeasible: false,
                }
            } else {
                BranchNodeSolveOutput {
                    lower_bound: 1.0,
                    solution: Some(node.id),
                    objective: Some(10.0 + node.id as f64),
                    branch_target: None,
                    infeasible: false,
                }
            }
        });

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
        });
        let strong_branching = TestStrongBranching;
        let cut_callback = TestCutCallback;
        let node_callback = TraceNodeCallback::default();
        let trace = node_callback.events.clone();

        let result = search.search_with_hooks(
            |node| {
                if node.depth == 0 {
                    BranchNodeSolveOutput {
                        lower_bound: 0.0,
                        solution: None,
                        objective: None,
                        branch_target: Some(1),
                        infeasible: false,
                    }
                } else {
                    BranchNodeSolveOutput {
                        lower_bound: 1.0,
                        solution: Some(node.id),
                        objective: Some(node.id as f64),
                        branch_target: None,
                        infeasible: false,
                    }
                }
            },
            BranchSearchHooks::new(&strong_branching, &cut_callback, &node_callback),
        );

        let trace = trace.lock().unwrap().clone();
        assert!(result.processed_nodes >= 2);
        assert_eq!(result.incumbent, Some(2));
        assert!(trace.iter().any(|event| event == "begin:0"));
        assert!(trace.iter().any(|event| event == "end:1:Pruned"));
        assert!(trace.iter().any(|event| event.starts_with("incumbent:")));
    }
}
