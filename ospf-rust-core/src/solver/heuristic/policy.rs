//! 启发式策略接口
//! Heuristic Policy Interface

use std::time::Duration;
use super::Iteration;

/// 启发式策略抽象 / Abstract heuristic policy
pub trait AbstractHeuristicPolicy: Send + Sync {
    /// 迭代回调 / Iteration hook
    fn update(&mut self, _iteration: &Iteration, _better: bool) {}

    /// 是否结束 / Whether finished
    fn finished(&self, iteration: &Iteration) -> bool;
}

/// 通用启发式策略 / Generic heuristic policy
#[derive(Debug, Clone)]
pub struct HeuristicPolicy {
    /// 最大迭代次数 / Maximum iterations
    pub iteration_limit: Option<usize>,
    /// 最大连续未改进次数 / Maximum consecutive non-improving iterations
    pub not_better_iteration_limit: Option<usize>,
    /// 最大运行时长 / Maximum runtime
    pub time_limit: Duration,
}

impl HeuristicPolicy {
    /// 创建策略 / Create policy
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置最大迭代次数 / Set iteration limit
    pub fn with_iteration_limit(mut self, limit: usize) -> Self {
        self.iteration_limit = Some(limit);
        self
    }

    /// 设置连续未改进限制 / Set non-improving iteration limit
    pub fn with_not_better_iteration_limit(mut self, limit: usize) -> Self {
        self.not_better_iteration_limit = Some(limit);
        self
    }

    /// 设置时长限制 / Set time limit
    pub fn with_time_limit(mut self, limit: Duration) -> Self {
        self.time_limit = limit;
        self
    }
}

impl Default for HeuristicPolicy {
    fn default() -> Self {
        Self {
            iteration_limit: None,
            not_better_iteration_limit: None,
            time_limit: Duration::from_secs(30 * 60),
        }
    }
}

impl AbstractHeuristicPolicy for HeuristicPolicy {
    fn finished(&self, iteration: &Iteration) -> bool {
        if self
            .iteration_limit
            .is_some_and(|limit| iteration.iteration > limit)
        {
            return true;
        }
        if self
            .not_better_iteration_limit
            .is_some_and(|limit| iteration.not_better_iteration > limit)
        {
            return true;
        }
        iteration.elapsed() > self.time_limit
    }
}
