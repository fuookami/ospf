//! 启发式迭代状态
//! Heuristic Iteration State

use std::time::{Duration, Instant};

/// 迭代状态 / Iteration state
#[derive(Debug, Clone)]
pub struct Iteration {
    /// 当前迭代次数 / Current iteration count
    pub iteration: usize,
    /// 未改进迭代次数 / Consecutive non-improving iterations
    pub not_better_iteration: usize,
    begin: Instant,
}

impl Iteration {
    /// 创建迭代状态 / Create iteration state
    pub fn new() -> Self {
        Self {
            iteration: 0,
            not_better_iteration: 0,
            begin: Instant::now(),
        }
    }

    /// 下一轮迭代 / Move to next iteration
    pub fn next(&mut self, better: bool) {
        self.iteration = self.iteration.saturating_add(1);
        if better {
            self.not_better_iteration = 0;
        } else {
            self.not_better_iteration = self.not_better_iteration.saturating_add(1);
        }
    }

    /// 已用时长 / Elapsed time
    pub fn elapsed(&self) -> Duration {
        self.begin.elapsed()
    }
}

impl Default for Iteration {
    fn default() -> Self {
        Self::new()
    }
}
