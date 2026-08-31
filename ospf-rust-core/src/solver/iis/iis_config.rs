//! IIS 配置
//! IIS Configuration

use std::time::Duration;

/// IIS 算法类型 / IIS Algorithm Type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IISAlgorithm {
    /// 弹性过滤算法 / Elastic filtering algorithm
    ///
    /// 通过添加松弛变量来识别 IIS，通常更快但可能找到更大的集合。
    /// Identifies IIS by adding slack variables, usually faster but may find larger sets.
    ElasticFiltering,

    /// 删除过滤算法 / Deletion filtering algorithm
    ///
    /// 逐个删除约束来识别 IIS，更精确但可能较慢。
    /// Identifies IIS by removing constraints one by one, more precise but potentially slower.
    #[default]
    DeletionFiltering,
}

/// IIS 计算配置 / IIS Computation Configuration
#[derive(Debug, Clone)]
pub struct IISConfig {
    /// 算法类型 / Algorithm type
    pub algorithm: IISAlgorithm,

    /// 最大迭代次数 / Maximum iterations
    pub max_iterations: usize,

    /// 时间限制 / Time limit
    pub time_limit: Option<Duration>,

    /// 是否包含变量边界 / Whether to include variable bounds
    pub include_bounds: bool,

    /// 是否输出详细日志 / Whether to output verbose logs
    pub verbose: bool,

    /// 弹性变量惩罚系数（仅弹性过滤）/ Elastic variable penalty (elastic filtering only)
    pub elastic_penalty: f64,

    /// 数值容差 / Numerical tolerance
    pub tolerance: f64,
}

impl Default for IISConfig {
    fn default() -> Self {
        Self {
            algorithm: IISAlgorithm::default(),
            max_iterations: 1000,
            time_limit: None,
            include_bounds: true,
            verbose: false,
            elastic_penalty: 1000.0,
            tolerance: 1e-6,
        }
    }
}

impl IISConfig {
    /// 创建新配置 / Create new configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// 使用弹性过滤算法 / Use elastic filtering algorithm
    pub fn with_elastic_filtering(mut self) -> Self {
        self.algorithm = IISAlgorithm::ElasticFiltering;
        self
    }

    /// 使用删除过滤算法 / Use deletion filtering algorithm
    pub fn with_deletion_filtering(mut self) -> Self {
        self.algorithm = IISAlgorithm::DeletionFiltering;
        self
    }

    /// 设置最大迭代次数 / Set maximum iterations
    pub fn with_max_iterations(mut self, iterations: usize) -> Self {
        self.max_iterations = iterations;
        self
    }

    /// 设置时间限制 / Set time limit
    pub fn with_time_limit(mut self, duration: Duration) -> Self {
        self.time_limit = Some(duration);
        self
    }

    /// 设置是否包含变量边界 / Set whether to include variable bounds
    pub fn with_bounds(mut self, include: bool) -> Self {
        self.include_bounds = include;
        self
    }

    /// 设置详细输出 / Set verbose output
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// 设置弹性惩罚系数 / Set elastic penalty
    pub fn with_elastic_penalty(mut self, penalty: f64) -> Self {
        self.elastic_penalty = penalty;
        self
    }

    /// 设置数值容差 / Set numerical tolerance
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = tolerance;
        self
    }
}

/// 约束来源 / Constraint Source
///
/// 标识约束来自原始模型的哪个部分。
/// Identifies which part of the original model a constraint comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstraintSource {
    /// 约束（非边界）/ Constraint (not a bound)
    Constraint(usize),
    /// 变量下界 / Variable lower bound
    LowerBound(usize),
    /// 变量上界 / Variable upper bound
    UpperBound(usize),
}

impl ConstraintSource {
    /// 是否为约束 / Whether it's a constraint
    pub fn is_constraint(&self) -> bool {
        matches!(self, ConstraintSource::Constraint(_))
    }

    /// 是否为边界 / Whether it's a bound
    pub fn is_bound(&self) -> bool {
        matches!(
            self,
            ConstraintSource::LowerBound(_) | ConstraintSource::UpperBound(_)
        )
    }

    /// 获取索引 / Get index
    pub fn index(&self) -> usize {
        match self {
            ConstraintSource::Constraint(i) => *i,
            ConstraintSource::LowerBound(i) => *i,
            ConstraintSource::UpperBound(i) => *i,
        }
    }
}
