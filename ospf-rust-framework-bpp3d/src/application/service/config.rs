#[cfg(feature = "serde")]
use crate::application::csv::CsvMaterializedApplicationRequest;

// ============================================================================
// ColumnGenerationConfig - 列生成配置 / Column generation config
// ============================================================================

/// 目标方向 / Objective sense
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectiveSense {
    /// 最小化 / Minimize
    Minimize,
    /// 最大化 / Maximize
    Maximize,
}

impl Default for ObjectiveSense {
    fn default() -> Self {
        Self::Minimize
    }
}

/// 列生成配置 / Column generation config
///
/// 只描述应用编排策略，不持有 solver 私有状态。
/// Describes application orchestration policy only, without solver-private state.
#[derive(Debug, Clone)]
pub struct ColumnGenerationConfig {
    /// 最大迭代次数 / Maximum iterations
    pub max_iterations: usize,
    /// 最大未改进迭代次数 / Maximum non-improving iterations
    pub max_not_better_iterations: usize,
    /// 最大列数量 / Maximum column count
    pub max_column_amount: usize,
    /// 每轮最大候选数 / Maximum candidates per iteration
    pub max_candidates_per_iteration: usize,
    /// 时间限制 / Time limit
    pub time_limit: Duration,
    /// reduced cost 接受阈值 / Reduced-cost acceptance tolerance
    pub reduced_cost_tolerance: f64,
    /// 目标方向 / Objective sense
    pub objective_sense: ObjectiveSense,
}

impl Default for ColumnGenerationConfig {
    fn default() -> Self {
        Self {
            max_iterations: 100,
            max_not_better_iterations: 10,
            max_column_amount: 50000,
            max_candidates_per_iteration: 256,
            time_limit: Duration::from_secs(30000),
            reduced_cost_tolerance: -1e-7,
            objective_sense: ObjectiveSense::Minimize,
        }
    }
}

impl ColumnGenerationConfig {
    /// 创建默认配置 / Create default config
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置最大迭代次数 / Set maximum iterations
    pub fn with_max_iterations(mut self, value: usize) -> Self {
        self.max_iterations = value;
        self
    }

    /// 设置最大未改进迭代次数 / Set maximum non-improving iterations
    pub fn with_max_not_better_iterations(mut self, value: usize) -> Self {
        self.max_not_better_iterations = value;
        self
    }

    /// 设置最大列数量 / Set maximum column count
    pub fn with_max_column_amount(mut self, value: usize) -> Self {
        self.max_column_amount = value;
        self
    }

    /// 设置每轮最大候选数 / Set maximum candidates per iteration
    pub fn with_max_candidates_per_iteration(mut self, value: usize) -> Self {
        self.max_candidates_per_iteration = value;
        self
    }

    /// 设置时间限制 / Set time limit
    pub fn with_time_limit(mut self, value: Duration) -> Self {
        self.time_limit = value;
        self
    }
}

