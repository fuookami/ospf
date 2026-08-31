//! 列生成策略配置 / Column generation policy configuration
//!
//! 定义列生成算法的配置参数和策略接口。
//! Defines configuration parameters and policy interfaces for column generation algorithms.

use std::time::Duration;

/// 列生成策略 / Column generation policy
///
/// 配置列生成算法的参数。
/// Configures column generation algorithm parameters.
#[derive(Debug, Clone)]
pub struct ColumnGenerationPolicy {
    /// 最大迭代次数 / Maximum iterations
    pub max_iterations: usize,
    /// 最大未改进迭代次数 / Maximum consecutive non-improving iterations
    pub max_not_better_iterations: usize,
    /// 最大列数量 / Maximum column amount
    pub max_column_amount: usize,
    /// 每个执行器最小列数量 / Minimum column amount per executor
    pub min_column_amount_per_executor: usize,
    /// 时间限制 / Time limit
    pub time_limit: Duration,
    /// 坏 reduced cost 列数量阈值 / Bad reduced cost column amount threshold
    pub bad_reduced_amount: usize,
    /// 局部固定阈值 / Local fix threshold
    pub local_fix_threshold: f64,
    /// 最优 Gap 阈值 / Optimal gap threshold
    pub optimal_gap_threshold: f64,
}

impl Default for ColumnGenerationPolicy {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            max_not_better_iterations: 10,
            max_column_amount: 50000,
            min_column_amount_per_executor: 0,
            time_limit: Duration::from_secs(30000),
            bad_reduced_amount: 20,
            local_fix_threshold: 0.9,
            optimal_gap_threshold: 1e-6,
        }
    }
}

impl ColumnGenerationPolicy {
    /// 创建默认策略 / Create default policy
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置最大迭代次数 / Set max iterations
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    /// 设置时间限制 / Set time limit
    pub fn with_time_limit(mut self, limit: Duration) -> Self {
        self.time_limit = limit;
        self
    }

    /// 设置最大列数量 / Set max column amount
    pub fn with_max_column_amount(mut self, amount: usize) -> Self {
        self.max_column_amount = amount;
        self
    }

    /// 设置局部固定阈值 / Set local fix threshold
    pub fn with_local_fix_threshold(mut self, threshold: f64) -> Self {
        self.local_fix_threshold = threshold;
        self
    }
}
