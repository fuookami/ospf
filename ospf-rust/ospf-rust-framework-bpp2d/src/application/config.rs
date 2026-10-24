//! 列生成配置 / Column generation configuration
//!
//! 只描述应用编排策略，不持有 solver 私有状态。
//! Describes application orchestration policy only, without solver-private state.

/// 列生成配置 / Column generation config
#[derive(Debug, Clone)]
pub struct ColumnGenerationConfig {
    /// 最大迭代次数 / Maximum iteration count
    pub iteration_limit: usize,
    /// 每轮最大候选数 / Maximum candidates per iteration
    pub max_columns_per_iteration: usize,
    /// 时间上限（毫秒，0 表示不限） / Time limit in milliseconds; 0 means unbounded
    pub time_limit_ms: u64,
    /// 是否启用最终 MILP / Whether the final MILP is enabled
    pub final_milp_enabled: bool,
    /// 检验数容差 / Reduced-cost tolerance
    pub reduced_cost_tolerance: f64,
}

impl Default for ColumnGenerationConfig {
    fn default() -> Self {
        Self {
            iteration_limit: 50,
            max_columns_per_iteration: 256,
            time_limit_ms: 0,
            final_milp_enabled: true,
            reduced_cost_tolerance: 1e-6,
        }
    }
}

impl ColumnGenerationConfig {
    /// 创建默认配置 / Create the default config
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置迭代上限 / Set the iteration limit
    pub fn with_iteration_limit(mut self, limit: usize) -> Self {
        self.iteration_limit = limit;
        self
    }

    /// 设置每轮候选上限 / Set the per-iteration candidate limit
    pub fn with_max_columns_per_iteration(mut self, limit: usize) -> Self {
        self.max_columns_per_iteration = limit;
        self
    }

    /// 设置时间上限（毫秒）/ Set the time limit in milliseconds
    pub fn with_time_limit_ms(mut self, limit: u64) -> Self {
        self.time_limit_ms = limit;
        self
    }
}
