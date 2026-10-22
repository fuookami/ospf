//! 搜索配置 / Search configuration

use std::time::Duration;

use crate::solver::heuristic::model::HeuristicValue;
use crate::solver::heuristic::operator::archive::BestArchive;
use crate::solver::heuristic::operator::stopping::{
    IterationLimit, NotBetterLimit, Or, StoppingCriterion, TimeLimit,
};
use crate::solver::heuristic::runner::runner_core::RunnerCore;

/// 搜索配置，集中描述终止条件与档案容量 / Search configuration describing stopping conditions and archive capacity
///
/// 所有算法工厂共用这一份配置，避免每个算法各自重复定义「迭代上限 / 不改进上限 / 时间上限」。
/// Every algorithm factory shares this configuration, so the iteration limit, the stagnation limit, and the time limit are
/// never redefined per algorithm.
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// 迭代上限 / Iteration limit
    pub max_iterations: u64,
    /// 连续不改进迭代上限 / Consecutive non-improving iteration limit
    pub not_better_limit: Option<u64>,
    /// 时间上限 / Time limit
    pub time_limit: Option<Duration>,
    /// 档案容量 / Archive capacity
    pub archive_capacity: usize,
}

impl SearchConfig {
    /// 以迭代上限创建配置 / Create a configuration with an iteration limit
    pub fn new(max_iterations: u64) -> Self {
        Self {
            max_iterations,
            not_better_limit: None,
            time_limit: None,
            archive_capacity: 1,
        }
    }

    /// 设置不改进上限 / Set the stagnation limit
    pub fn with_not_better_limit(mut self, limit: u64) -> Self {
        self.not_better_limit = Some(limit);
        self
    }

    /// 设置时间上限 / Set the time limit
    pub fn with_time_limit(mut self, limit: Duration) -> Self {
        self.time_limit = Some(limit);
        self
    }

    /// 设置档案容量 / Set the archive capacity
    pub fn with_archive_capacity(mut self, capacity: usize) -> Self {
        self.archive_capacity = capacity.max(1);
        self
    }

    /// 构造终止判据 / Build the stopping criterion
    pub fn stopping(&self) -> Box<dyn StoppingCriterion> {
        let mut stopping: Box<dyn StoppingCriterion> =
            Box::new(IterationLimit::new(self.max_iterations));
        if let Some(limit) = self.not_better_limit {
            stopping = Box::new(Or::new(stopping, Box::new(NotBetterLimit::new(limit))));
        }
        if let Some(limit) = self.time_limit {
            stopping = Box::new(Or::new(stopping, Box::new(TimeLimit::new(limit))));
        }
        stopping
    }

    /// 构造搜索核心 / Build the search core
    pub fn core<V>(&self) -> RunnerCore<V, V>
    where
        V: HeuristicValue,
    {
        RunnerCore::new(
            Box::new(BestArchive::new(self.archive_capacity)),
            self.stopping(),
        )
    }
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self::new(1_000)
    }
}
