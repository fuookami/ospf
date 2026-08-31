//! 列生成迭代状态管理 / Column generation iteration state management
//!
//! 跟踪迭代次数、收敛状态、上下界和最优比率。
//! Tracks iteration count, convergence status, bounds, and optimal rate.

use std::time::{Duration, Instant};

/// 列生成迭代状态 / Column generation iteration state
///
/// 跟踪列生成算法的迭代状态，包括 LP/MILP 目标值、
/// 上下界、收敛检测和最优比率。
/// Tracks iteration state for column generation algorithms, including
/// LP/MILP objectives, bounds, convergence detection, and optimal rate.
#[derive(Debug, Clone)]
pub struct Iteration {
    /// 当前迭代次数 / Current iteration count
    pub iteration: usize,
    /// 连续未改进迭代次数 / Consecutive non-improving iterations
    pub not_better_iteration: usize,

    /// 最佳 LP 目标值 / Best LP objective value
    pub best_lp_obj: f64,
    /// 最佳 MILP 目标值 / Best MILP objective value
    pub best_ip_obj: f64,
    /// 最佳对偶目标值（下界来源）/ Best dual objective value (lower bound source)
    pub best_dual_obj: f64,

    /// 当前下界 / Current lower bound
    pub lower_bound: f64,
    /// 当前上界 / Current upper bound
    pub upper_bound: Option<f64>,

    /// LP 目标值历史 / LP objective value history
    pub lp_obj_history: Vec<f64>,
    /// MILP 目标值历史 / MILP objective value history
    pub ip_obj_history: Vec<f64>,

    /// 慢改进步长阈值 / Slow improvement step threshold
    pub improvement_slow_step: f64,
    /// 慢改进计数 / Slow improvement count
    pub improvement_slow_count: usize,
    /// 慢改进判定阈值 / Slow improvement threshold
    pub improvement_slow_threshold: usize,

    /// 算法开始时间 / Algorithm start time
    pub begin: Instant,
}

impl Default for Iteration {
    fn default() -> Self {
        Self::new()
    }
}

impl Iteration {
    /// 创建迭代状态 / Create iteration state
    ///
    /// 使用默认参数创建迭代状态。
    /// Creates iteration state with default parameters.
    pub fn new() -> Self {
        Self {
            iteration: 0,
            not_better_iteration: 0,
            best_lp_obj: f64::NEG_INFINITY,
            best_ip_obj: f64::NEG_INFINITY,
            best_dual_obj: f64::NEG_INFINITY,
            lower_bound: f64::NEG_INFINITY,
            upper_bound: None,
            lp_obj_history: Vec::new(),
            ip_obj_history: Vec::new(),
            improvement_slow_step: 100.0,
            improvement_slow_count: 0,
            improvement_slow_threshold: 5,
            begin: Instant::now(),
        }
    }

    /// 创建带自定义收敛参数的迭代状态 / Create iteration state with custom convergence parameters
    ///
    /// # Arguments
    /// * `improvement_slow_step` - 慢改进步长阈值 / Slow improvement step threshold
    /// * `improvement_slow_threshold` - 慢改进判定迭代次数 / Consecutive slow iterations threshold
    pub fn with_convergence_params(
        improvement_slow_step: f64,
        improvement_slow_threshold: usize,
    ) -> Self {
        Self {
            improvement_slow_step,
            improvement_slow_threshold,
            ..Self::new()
        }
    }

    /// 记录 LP 求解结果 / Record LP solve result
    ///
    /// 返回 `true` 如果目标值有改进。
    /// Returns `true` if the objective value improved.
    pub fn record_lp(&mut self, lp_obj: f64) -> bool {
        self.lp_obj_history.push(lp_obj);

        if lp_obj > self.best_lp_obj {
            self.best_lp_obj = lp_obj;
            self.improvement_slow_count = 0;
            true
        } else {
            // 改进幅度小于步长阈值时计入慢改进
            // Count as slow improvement when improvement is below step threshold
            self.improvement_slow_count += 1;
            false
        }
    }

    /// 记录 MILP 求解结果 / Record MILP solve result
    ///
    /// 返回 `true` 如果目标值有改进。
    /// Returns `true` if the objective value improved.
    pub fn record_ip(&mut self, ip_obj: f64) -> bool {
        self.ip_obj_history.push(ip_obj);

        if ip_obj > self.best_ip_obj {
            self.best_ip_obj = ip_obj;
            self.not_better_iteration = 0;
            true
        } else {
            self.not_better_iteration += 1;
            false
        }
    }

    /// 刷新下界 / Refresh lower bound
    ///
    /// 根据新列的 reduced cost 更新下界。
    /// 对偶目标 = LP 目标 + 最优 reduced cost 之和。
    /// Updates lower bound based on reduced costs of new columns.
    /// Dual objective = LP objective + sum of best reduced costs.
    pub fn refresh_lower_bound<C>(
        &mut self,
        new_columns: &[C],
        reduced_cost: impl Fn(&C) -> f64,
        executor_of: impl Fn(&C) -> Option<usize>,
    ) {
        if new_columns.is_empty() {
            return;
        }

        // 找到每个执行器的最佳 reduced cost
        // Find best reduced cost per executor
        let mut best_reduced_cost_per_executor = std::collections::HashMap::<usize, f64>::new();

        for col in new_columns {
            if let Some(executor_idx) = executor_of(col) {
                let rc = reduced_cost(col);
                let entry = best_reduced_cost_per_executor
                    .entry(executor_idx)
                    .or_insert(f64::INFINITY);
                if rc < *entry {
                    *entry = rc;
                }
            }
        }

        // 对偶目标 = LP 目标 + 最优 reduced cost 之和
        // Dual objective = LP objective + sum of best reduced costs
        let sum_best_rc: f64 = best_reduced_cost_per_executor.values().sum();
        let current_dual_obj = self.best_lp_obj + sum_best_rc;

        if current_dual_obj > self.best_dual_obj && current_dual_obj < self.best_ip_obj {
            self.best_dual_obj = current_dual_obj;
            self.lower_bound = self.lower_bound.max(self.best_dual_obj);
        }
    }

    /// 检查是否收敛 / Check if converged
    ///
    /// 当 LP 和 MILP 目标值之差小于给定容差时认为收敛。
    /// Converged when LP and MILP objective difference is within tolerance.
    pub fn is_converged(&self, tolerance: f64) -> bool {
        if self.upper_bound.is_none() || self.best_ip_obj == f64::NEG_INFINITY {
            return false;
        }
        (self.upper_bound.unwrap() - self.lower_bound).abs() < tolerance
    }

    /// 检查改进是否缓慢 / Check if improvement is slow
    ///
    /// 当连续多次未改进 MILP 目标时认为改进缓慢。
    /// Improvement is slow when consecutive non-improving MILP iterations
    /// exceed the threshold.
    pub fn is_improvement_slow(&self) -> bool {
        self.improvement_slow_count >= self.improvement_slow_threshold
    }

    /// 将慢改进步长减半 / Halve the slow improvement step
    ///
    /// 用于加速收敛。每调用一次步长减半。
    /// Used to accelerate convergence. Step is halved on each call.
    pub fn halve_step(&mut self) {
        self.improvement_slow_step /= 2.0;
    }

    /// 计算最优比率 / Calculate optimal rate
    ///
    /// optimal_rate = (lower_bound + 1) / (best_ip_obj + 1) 的平方根，
    /// 但不超过 1.0。如果有上界，则取 max(actual_rate, (upper - best) / upper)。
    /// optimal_rate = sqrt((lower_bound + 1) / (best_ip_obj + 1)),
    /// capped at 1.0. With upper bound, max(actual_rate, (upper - best) / upper).
    pub fn optimal_rate(&self) -> f64 {
        if self.best_ip_obj == f64::NEG_INFINITY {
            return 0.0;
        }

        let denominator = self.best_ip_obj + 1.0;
        if denominator.abs() < f64::EPSILON {
            return 1.0;
        }

        let numerator = self.lower_bound + 1.0;
        let actual_rate = (numerator / denominator).sqrt().min(1.0);

        if let Some(upper) = self.upper_bound {
            if upper.abs() > f64::EPSILON {
                let gap_rate = (upper - self.best_ip_obj) / upper;
                actual_rate.max(gap_rate)
            } else {
                actual_rate
            }
        } else {
            actual_rate
        }
    }

    /// 获取当前下界 / Get current lower bound
    pub fn current_lower_bound(&self) -> f64 {
        self.lower_bound
    }

    /// 获取当前上界 / Get current upper bound
    pub fn current_upper_bound(&self) -> Option<f64> {
        self.upper_bound
    }

    /// 设置上界 / Set upper bound
    ///
    /// 通常在首次 MILP 求解后设置。
    /// Typically set after the first MILP solve.
    pub fn set_upper_bound(&mut self, upper: f64) {
        match self.upper_bound {
            None => self.upper_bound = Some(upper),
            Some(current) => {
                if upper < current {
                    self.upper_bound = Some(upper);
                }
            }
        }
    }

    /// 已用时长 / Elapsed time since start
    pub fn elapsed(&self) -> Duration {
        self.begin.elapsed()
    }

    /// 进入下一轮迭代 / Move to next iteration
    pub fn next_iteration(&mut self) {
        self.iteration += 1;
    }

    /// 创建快照 / Create snapshot
    ///
    /// 创建当前迭代状态的只读快照。
    /// Creates a read-only snapshot of the current iteration state.
    pub fn snapshot(&self, active_column_count: usize) -> IterationSnapshot {
        IterationSnapshot {
            iteration: self.iteration,
            lp_objective: self.lp_obj_history.last().copied(),
            ip_objective: self.ip_obj_history.last().copied(),
            lower_bound: self.lower_bound,
            upper_bound: self.upper_bound,
            optimal_rate: self.optimal_rate(),
            is_improvement_slow: self.is_improvement_slow(),
            active_column_count,
            elapsed: self.elapsed(),
        }
    }
}

/// 迭代快照 / Iteration snapshot
///
/// 只读快照，用于报告和追踪。
/// Read-only snapshot for reporting and tracing.
#[derive(Debug, Clone)]
pub struct IterationSnapshot {
    /// 迭代次数 / Iteration count
    pub iteration: usize,
    /// LP 目标值 / LP objective value
    pub lp_objective: Option<f64>,
    /// MILP 目标值 / MILP objective value
    pub ip_objective: Option<f64>,
    /// 下界 / Lower bound
    pub lower_bound: f64,
    /// 上界 / Upper bound
    pub upper_bound: Option<f64>,
    /// 最优比率 / Optimal rate
    pub optimal_rate: f64,
    /// 是否改进缓慢 / Whether improvement is slow
    pub is_improvement_slow: bool,
    /// 活跃列数量 / Active column count
    pub active_column_count: usize,
    /// 已用时间 / Elapsed time
    pub elapsed: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iteration_record_lp_improvement() {
        let mut iter = Iteration::new();
        assert!(iter.record_lp(10.0));
        assert_eq!(iter.best_lp_obj, 10.0);
        assert!(!iter.record_lp(9.0));
        assert_eq!(iter.best_lp_obj, 10.0);
    }

    #[test]
    fn test_iteration_record_ip_improvement() {
        let mut iter = Iteration::new();
        assert!(iter.record_ip(15.0));
        assert_eq!(iter.best_ip_obj, 15.0);
        assert!(!iter.record_ip(14.0));
        assert_eq!(iter.best_ip_obj, 15.0);
        assert_eq!(iter.not_better_iteration, 1);
    }

    #[test]
    fn test_iteration_convergence() {
        let mut iter = Iteration::new();
        iter.lower_bound = 10.0;
        iter.best_ip_obj = 10.0001;
        iter.set_upper_bound(10.0001);
        // 10.0001 - 10.0 = 0.0001 < 1e-3
        assert!(iter.is_converged(1e-3));

        // 上界只减不增，上界降到 10.00005
        iter.set_upper_bound(10.00005);
        // 10.00005 - 10.0 = 0.00005 < 1e-3
        assert!(iter.is_converged(1e-3));

        // 下界上升，gap 变大
        iter.lower_bound = 5.0;
        assert!(!iter.is_converged(1e-3));
    }

    #[test]
    fn test_iteration_optimal_rate() {
        let mut iter = Iteration::new();

        // 无上界时
        iter.lower_bound = 8.0;
        iter.best_ip_obj = 10.0;
        let rate = iter.optimal_rate();
        assert!(rate > 0.0 && rate <= 1.0);

        // 有上界时
        iter.set_upper_bound(11.0);
        let rate_with_upper = iter.optimal_rate();
        assert!(rate_with_upper >= rate);
    }

    #[test]
    fn test_iteration_optimal_rate_no_ip() {
        let iter = Iteration::new();
        assert_eq!(iter.optimal_rate(), 0.0);
    }

    #[test]
    fn test_iteration_improvement_slow() {
        let mut iter = Iteration::with_convergence_params(100.0, 3);

        // 首次记录有改进
        iter.record_lp(10.0);
        assert!(!iter.is_improvement_slow());

        // 连续 3 次未改进
        iter.record_lp(9.0);
        iter.record_lp(9.5);
        iter.record_lp(8.0);
        assert!(iter.is_improvement_slow());
    }

    #[test]
    fn test_iteration_halve_step() {
        let mut iter = Iteration::with_convergence_params(100.0, 5);
        iter.halve_step();
        assert!((iter.improvement_slow_step - 50.0).abs() < f64::EPSILON);
        iter.halve_step();
        assert!((iter.improvement_slow_step - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_iteration_refresh_lower_bound() {
        let mut iter = Iteration::new();
        iter.best_lp_obj = 10.0;
        iter.best_ip_obj = 20.0;

        struct TestCol {
            reduced_cost: f64,
            executor: Option<usize>,
        }

        let new_cols = vec![
            TestCol {
                reduced_cost: -2.0,
                executor: Some(0),
            },
            TestCol {
                reduced_cost: -3.0,
                executor: Some(1),
            },
            TestCol {
                reduced_cost: -1.0,
                executor: Some(0),
            },
        ];

        iter.refresh_lower_bound(&new_cols, |c| c.reduced_cost, |c| c.executor);

        // 对偶目标 = 10.0 + (-2.0) + (-3.0) = 5.0
        // lower_bound = max(-inf, 5.0) = 5.0
        assert!((iter.best_dual_obj - 5.0).abs() < 1e-9);
        assert!((iter.lower_bound - 5.0).abs() < 1e-9);
    }

    #[test]
    fn test_iteration_set_upper_bound() {
        let mut iter = Iteration::new();
        iter.set_upper_bound(20.0);
        assert_eq!(iter.upper_bound, Some(20.0));

        // 更小的上界才更新
        iter.set_upper_bound(18.0);
        assert_eq!(iter.upper_bound, Some(18.0));

        // 更大的上界不更新
        iter.set_upper_bound(25.0);
        assert_eq!(iter.upper_bound, Some(18.0));
    }

    #[test]
    fn test_iteration_snapshot() {
        let mut iter = Iteration::new();
        iter.record_lp(10.0);
        iter.record_ip(15.0);
        iter.lower_bound = 8.0;
        iter.set_upper_bound(16.0);

        let snapshot = iter.snapshot(42);
        assert_eq!(snapshot.iteration, 0);
        assert_eq!(snapshot.lp_objective, Some(10.0));
        assert_eq!(snapshot.ip_objective, Some(15.0));
        assert_eq!(snapshot.active_column_count, 42);
        assert!(!snapshot.is_improvement_slow);
    }

    #[test]
    fn test_iteration_next_iteration() {
        let mut iter = Iteration::new();
        assert_eq!(iter.iteration, 0);
        iter.next_iteration();
        assert_eq!(iter.iteration, 1);
        iter.next_iteration();
        assert_eq!(iter.iteration, 2);
    }
}
