//! 迭代计数器 / Iteration counter

use std::time::{Duration, Instant};

/// 启发式迭代计数器 / Heuristic iteration counter
///
/// 同时跟踪总迭代次数、连续无改进次数与已用时间，是终止判据与参数演化的共同输入。
/// Tracks total iterations, consecutive non-improving iterations, and elapsed time; shared by stopping criteria and
/// parameter schedules.
#[derive(Debug, Clone)]
pub struct Iteration {
    iteration: u64,
    not_better_iteration: u64,
    begin: Instant,
}

impl Iteration {
    /// 创建从零开始的计数器 / Create a counter starting from zero
    pub fn new() -> Self {
        Self::with_begin(Instant::now())
    }

    /// 以指定起点时间创建计数器 / Create a counter with an explicit begin instant
    pub fn with_begin(begin: Instant) -> Self {
        Self {
            iteration: 0,
            not_better_iteration: 0,
            begin,
        }
    }

    /// 从已保存的计数恢复，用于会话快照 / Restore from saved counters, used by session snapshots
    pub fn with_counts(iteration: u64, not_better_iteration: u64) -> Self {
        Self {
            iteration,
            not_better_iteration,
            begin: Instant::now(),
        }
    }

    /// 当前迭代次数 / Current iteration count
    pub fn iteration(&self) -> u64 {
        self.iteration
    }

    /// 连续无改进迭代次数 / Consecutive non-improving iterations
    pub fn not_better_iteration(&self) -> u64 {
        self.not_better_iteration
    }

    /// 已用时间 / Elapsed time
    pub fn time(&self) -> Duration {
        self.begin.elapsed()
    }

    /// 推进迭代计数器 / Advance the iteration counter
    ///
    /// @param better 本轮是否有改进 / Whether this round improved
    pub fn next(&mut self, better: bool) {
        self.iteration = self.iteration.saturating_add(1);
        if better {
            self.not_better_iteration = 0;
        } else {
            self.not_better_iteration = self.not_better_iteration.saturating_add(1);
        }
    }
}

impl Default for Iteration {
    fn default() -> Self {
        Self::new()
    }
}
