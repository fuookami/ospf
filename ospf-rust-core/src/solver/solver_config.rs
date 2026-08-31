//! 求解器配置
//! Solver Configuration

use std::time::Duration;

/// 求解器配置 / Solver Configuration
#[derive(Debug, Clone)]
pub struct SolverConfig {
    /// 求解器名称 / Solver name
    pub name: String,
    /// 时间限制 / Time limit
    pub time_limit: Option<Duration>,
    /// 迭代限制 / Iteration limit
    pub iteration_limit: Option<usize>,
    /// 节点限制（MIP）/ Node limit (MIP)
    pub node_limit: Option<usize>,
    /// MIP Gap 容差 / MIP Gap tolerance
    pub mip_gap: Option<f64>,
    /// 最优容差 / Optimality tolerance
    pub optimality_tolerance: Option<f64>,
    /// 可行性容差 / Feasibility tolerance
    pub feasibility_tolerance: Option<f64>,
    /// 是否输出日志 / Whether to output log
    pub verbose: bool,
    /// 线程数 / Number of threads
    pub threads: Option<usize>,
    /// 内存限制（MB）/ Memory limit (MB)
    pub memory_limit: Option<usize>,
    /// 随机种子 / Random seed
    pub seed: Option<u64>,
}

impl SolverConfig {
    /// 创建默认配置 / Create default configuration
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            time_limit: None,
            iteration_limit: None,
            node_limit: None,
            mip_gap: None,
            optimality_tolerance: None,
            feasibility_tolerance: None,
            verbose: false,
            threads: None,
            memory_limit: None,
            seed: None,
        }
    }

    /// 设置时间限制 / Set time limit
    pub fn with_time_limit(mut self, limit: Duration) -> Self {
        self.time_limit = Some(limit);
        self
    }

    /// 设置迭代限制 / Set iteration limit
    pub fn with_iteration_limit(mut self, limit: usize) -> Self {
        self.iteration_limit = Some(limit);
        self
    }

    /// 设置节点限制 / Set node limit
    pub fn with_node_limit(mut self, limit: usize) -> Self {
        self.node_limit = Some(limit);
        self
    }

    /// 设置 MIP Gap / Set MIP gap
    pub fn with_mip_gap(mut self, gap: f64) -> Self {
        self.mip_gap = Some(gap);
        self
    }

    /// 设置最优容差 / Set optimality tolerance
    pub fn with_optimality_tolerance(mut self, tol: f64) -> Self {
        self.optimality_tolerance = Some(tol);
        self
    }

    /// 设置可行性容差 / Set feasibility tolerance
    pub fn with_feasibility_tolerance(mut self, tol: f64) -> Self {
        self.feasibility_tolerance = Some(tol);
        self
    }

    /// 设置日志输出 / Set verbose
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// 设置线程数 / Set thread count
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.threads = Some(threads);
        self
    }

    /// 设置内存限制 / Set memory limit
    pub fn with_memory_limit(mut self, limit: usize) -> Self {
        self.memory_limit = Some(limit);
        self
    }

    /// 设置随机种子 / Set random seed
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self::new("default")
    }
}
