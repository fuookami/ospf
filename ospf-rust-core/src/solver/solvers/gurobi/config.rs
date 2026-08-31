//! Gurobi 求解器配置
//! Gurobi solver configuration

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use crate::error::Result;
use crate::model::ObjectiveCategory;
use crate::solver::{SolverConfig, SolverStatus};

/// Gurobi 分阶段回调节点 / Gurobi staged callback points
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GurobiStage {
    /// 建模完成后 / After modeling
    AfterModeling,
    /// 求解前配置完成后 / After configuration before optimize
    Configuration,
    /// 解分析阶段 / Analyzing solution
    AnalyzingSolution,
    /// 失败后阶段 / After failure
    AfterFailure,
}

/// Gurobi 分阶段状态快照 / Gurobi staged status snapshot
#[derive(Debug, Clone)]
pub struct GurobiStageStatus {
    /// 当前阶段 / Current stage
    pub stage: GurobiStage,
    /// 求解器状态（若可用）/ Solver status (when available)
    pub solver_status: Option<SolverStatus>,
    /// 累计耗时 / Elapsed solve time
    pub solve_time: Duration,
    /// 目标值 / Objective value
    pub objective_value: Option<f64>,
    /// 最优下界 / Best bound
    pub best_bound: Option<f64>,
    /// MIP Gap / MIP gap
    pub mip_gap: Option<f64>,
    /// 迭代次数 / Iteration count
    pub iterations: Option<usize>,
    /// 节点数 / Node count
    pub node_count: Option<usize>,
}

/// Gurobi 分阶段回调 / Gurobi staged callback
pub type GurobiStageCallback = Arc<dyn Fn(&GurobiStageStatus) -> Result<()> + Send + Sync>;

/// Gurobi 迭代遥测状态 / Gurobi iterative telemetry snapshot
#[derive(Debug, Clone)]
pub struct GurobiTelemetryStatus {
    /// 累计耗时 / Elapsed solve time
    pub solve_time: Duration,
    /// 目标方向 / Objective direction
    pub objective_category: ObjectiveCategory,
    /// 首个 incumbent 目标值 / First incumbent objective value
    pub initial_objective_value: Option<f64>,
    /// 当前目标值（MIP incumbent）/ Current objective value (MIP incumbent)
    pub objective_value: Option<f64>,
    /// 当前 incumbent 解向量 / Current incumbent solution vector
    pub incumbent_solution: Option<Vec<f64>>,
    /// 当前最优下界 / Current best bound
    pub best_bound: Option<f64>,
    /// 当前 MIP Gap / Current MIP gap
    pub mip_gap: Option<f64>,
    /// 当前迭代次数 / Current iteration count
    pub iterations: Option<usize>,
    /// 当前已探索节点数 / Current explored node count
    pub node_count: Option<usize>,
}

/// Gurobi 迭代遥测回调 / Gurobi iterative telemetry callback
pub type GurobiTelemetryCallback = Arc<dyn Fn(&GurobiTelemetryStatus) -> Result<()> + Send + Sync>;

/// Gurobi 原生回调触发位置快照 / Snapshot for native callback location
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GurobiNativeWhere {
    /// MIP 回调点 / MIP callback point
    Mip,
    /// 其他回调点 / Other callback point
    Other,
}

/// 原生观察回调快照 / Native observer callback snapshot
#[derive(Debug, Clone)]
pub struct GurobiNativeSnapshot {
    /// 回调位置 / Callback location
    pub where_point: GurobiNativeWhere,
    /// 累计耗时 / Elapsed solve time
    pub solve_time: Duration,
    /// 目标方向 / Objective direction
    pub objective_category: ObjectiveCategory,
    /// 首个 incumbent 目标值 / First incumbent objective value
    pub initial_objective_value: Option<f64>,
    /// 当前目标值 / Current objective value
    pub objective_value: Option<f64>,
    /// 当前 incumbent 解向量 / Current incumbent solution vector
    pub incumbent_solution: Option<Vec<f64>>,
    /// 当前最优下界 / Current best bound
    pub best_bound: Option<f64>,
    /// 当前 MIP gap / Current MIP gap
    pub mip_gap: Option<f64>,
    /// 当前迭代次数 / Current iteration count
    pub iterations: Option<usize>,
    /// 当前已探索节点数 / Current explored node count
    pub node_count: Option<usize>,
}

/// 原生观察回调控制信号 / Native observer callback control signal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GurobiNativeControl {
    /// 继续求解 / Continue solving
    Continue,
    /// 请求终止 / Request terminate
    Terminate,
}

/// Gurobi 原生观察回调 / Gurobi native observer callback
pub type GurobiNativeObserver =
    Arc<dyn Fn(&GurobiNativeSnapshot) -> Result<GurobiNativeControl> + Send + Sync>;

/// 数值策略模板 / Numeric profile template
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GurobiNumericProfile {
    /// 性能优先 / Performance first
    Performance,
    /// 均衡 / Balanced
    Balanced,
    /// 稳健优先 / Robustness first
    Robust,
}

/// Gurobi 数值诊断结果 / Gurobi numeric diagnostics result
#[derive(Debug, Clone)]
pub struct GurobiNumericDiagnostics {
    /// 非零系数数量 / Non-zero coefficient count
    pub nonzero_coefficient_count: usize,
    /// 非有限系数数量（NaN/Inf）/ Non-finite coefficient count (NaN/Inf)
    pub non_finite_coefficient_count: usize,
    /// 系数绝对值最小值（非零）/ Minimum absolute non-zero coefficient
    pub min_abs_nonzero_coefficient: Option<f64>,
    /// 系数绝对值最大值 / Maximum absolute coefficient
    pub max_abs_coefficient: Option<f64>,
    /// 动态范围（max/min）/ Dynamic range (max/min)
    pub dynamic_range: Option<f64>,
    /// 过小系数数量（<1e-10）/ Tiny coefficient count (<1e-10)
    pub tiny_coefficient_count: usize,
    /// 过大系数数量（>1e6）/ Large coefficient count (>1e6)
    pub large_coefficient_count: usize,
    /// 推荐模板 / Recommended profile
    pub recommended_profile: GurobiNumericProfile,
}

/// Gurobi 数值诊断回调 / Gurobi numeric diagnostics callback
pub type GurobiNumericDiagnosticsCallback =
    Arc<dyn Fn(&GurobiNumericDiagnostics) -> Result<()> + Send + Sync>;

/// Gurobi 原生回调 / Gurobi native callback
pub type GurobiNativeCallback =
    Arc<dyn for<'a> Fn(grb::callback::Where<'a>) -> grb::callback::CbResult + Send + Sync>;

/// Gurobi 环境创建回调 / Gurobi environment creation callback
pub type GurobiEnvCallback = Arc<dyn Fn(&mut grb::Env) -> Result<()> + Send + Sync>;

/// Gurobi 求解器配置 / Gurobi Solver Configuration
#[derive(Clone)]
pub struct GurobiConfig {
    /// 时间限制（秒）/ Time limit (seconds)
    pub time_limit: Option<f64>,
    /// MIP Gap 容差 / MIP Gap tolerance
    pub mip_gap: Option<f64>,
    /// 最大迭代次数 / Maximum iterations
    pub max_iterations: Option<i32>,
    /// 输出标志 / Output flag
    pub output_flag: bool,
    /// 线程数 / Number of threads
    pub threads: Option<i32>,
    /// 零系数容差 / Zero-coefficient tolerance
    pub coefficient_zero_tolerance: f64,
    /// 数值稳定性关注级别（0-3）/ Numeric focus level (0-3)
    pub numeric_focus: Option<i32>,
    /// 缩放策略（-1..3）/ Scaling strategy (-1..3)
    pub scale_flag: Option<i32>,
    /// 启用数值诊断 / Enable numeric diagnostics
    pub enable_numeric_diagnostics: bool,
    /// 自动应用诊断推荐模板 / Auto apply recommended profile from diagnostics
    pub auto_apply_numeric_profile: bool,
    /// 日志文件 / Log file
    pub log_file: Option<String>,
    /// 节点限制 / Node limit
    pub node_limit: Option<i32>,
    /// 内存限制（GB）/ Memory limit (GB)
    pub mem_limit: Option<f64>,
    /// Compute Server 地址 / Compute Server endpoint
    pub compute_server: Option<String>,
    /// Compute Server 密码 / Compute Server password
    pub server_password: Option<String>,
    /// Compute Server 连接超时（秒）/ Compute Server timeout (seconds)
    pub server_timeout: Option<i32>,
    /// Compute Server 排队超时（秒）/ Compute Server queue timeout (seconds)
    pub cs_queue_timeout: Option<f64>,
    /// 无改进提前终止阈值（秒）/ No-improvement early-stop threshold (seconds)
    pub no_improvement_time_limit: Option<f64>,
    /// 改进判定阈值 / Improvement tolerance threshold
    pub improve_threshold: Option<f64>,
    /// 遥测最小上报间隔（秒）/ Minimum telemetry emit interval (seconds)
    pub telemetry_min_interval: Option<f64>,
    /// 分阶段回调 / Staged callback
    pub stage_callback: Option<GurobiStageCallback>,
    /// 迭代遥测回调 / Iterative telemetry callback
    pub telemetry_callback: Option<GurobiTelemetryCallback>,
    /// 数值诊断回调 / Numeric diagnostics callback
    pub numeric_diagnostics_callback: Option<GurobiNumericDiagnosticsCallback>,
    /// 原生回调 / Native callback
    pub native_callback: Option<GurobiNativeCallback>,
    /// 原生观察回调（可多播）/ Native observer callbacks (multicast)
    pub native_observers: Vec<GurobiNativeObserver>,
    /// 环境创建回调 / Environment creation callback
    pub env_callback: Option<GurobiEnvCallback>,
}

impl fmt::Debug for GurobiConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GurobiConfig")
            .field("time_limit", &self.time_limit)
            .field("mip_gap", &self.mip_gap)
            .field("max_iterations", &self.max_iterations)
            .field("output_flag", &self.output_flag)
            .field("threads", &self.threads)
            .field(
                "coefficient_zero_tolerance",
                &self.coefficient_zero_tolerance,
            )
            .field("numeric_focus", &self.numeric_focus)
            .field("scale_flag", &self.scale_flag)
            .field(
                "enable_numeric_diagnostics",
                &self.enable_numeric_diagnostics,
            )
            .field(
                "auto_apply_numeric_profile",
                &self.auto_apply_numeric_profile,
            )
            .field("log_file", &self.log_file)
            .field("node_limit", &self.node_limit)
            .field("mem_limit", &self.mem_limit)
            .field("compute_server", &self.compute_server)
            .field(
                "server_password_registered",
                &self.server_password.is_some(),
            )
            .field("server_timeout", &self.server_timeout)
            .field("cs_queue_timeout", &self.cs_queue_timeout)
            .field("no_improvement_time_limit", &self.no_improvement_time_limit)
            .field("improve_threshold", &self.improve_threshold)
            .field("telemetry_min_interval", &self.telemetry_min_interval)
            .field("stage_callback_registered", &self.stage_callback.is_some())
            .field(
                "telemetry_callback_registered",
                &self.telemetry_callback.is_some(),
            )
            .field(
                "numeric_diagnostics_callback_registered",
                &self.numeric_diagnostics_callback.is_some(),
            )
            .field(
                "native_callback_registered",
                &self.native_callback.is_some(),
            )
            .field("native_observers_count", &self.native_observers.len())
            .field("env_callback_registered", &self.env_callback.is_some())
            .finish()
    }
}

impl Default for GurobiConfig {
    fn default() -> Self {
        Self {
            time_limit: None,
            mip_gap: None,
            max_iterations: None,
            output_flag: true,
            threads: None,
            coefficient_zero_tolerance: 1e-13,
            numeric_focus: None,
            scale_flag: None,
            enable_numeric_diagnostics: true,
            auto_apply_numeric_profile: false,
            log_file: None,
            node_limit: None,
            mem_limit: None,
            compute_server: None,
            server_password: None,
            server_timeout: None,
            cs_queue_timeout: None,
            no_improvement_time_limit: None,
            improve_threshold: None,
            telemetry_min_interval: None,
            stage_callback: None,
            telemetry_callback: None,
            numeric_diagnostics_callback: None,
            native_callback: None,
            native_observers: Vec::new(),
            env_callback: None,
        }
    }
}

impl From<&SolverConfig> for GurobiConfig {
    fn from(config: &SolverConfig) -> Self {
        let mut gurobi_config = GurobiConfig::new();
        gurobi_config.time_limit = config.time_limit.map(|duration| duration.as_secs_f64());
        gurobi_config.mip_gap = config.mip_gap;
        gurobi_config.max_iterations = config
            .iteration_limit
            .map(|limit| limit.min(i32::MAX as usize) as i32);
        gurobi_config.output_flag = config.verbose;
        gurobi_config.threads = config
            .threads
            .map(|threads| threads.min(i32::MAX as usize) as i32);
        gurobi_config.node_limit = config
            .node_limit
            .map(|limit| limit.min(i32::MAX as usize) as i32);
        gurobi_config.mem_limit = config.memory_limit.map(|limit_mb| limit_mb as f64 / 1024.0);
        gurobi_config.no_improvement_time_limit = config
            .no_improvement_time_limit
            .map(|duration| duration.as_secs_f64());
        gurobi_config.improve_threshold = config.improve_threshold;
        gurobi_config
    }
}

impl From<SolverConfig> for GurobiConfig {
    fn from(config: SolverConfig) -> Self {
        Self::from(&config)
    }
}

impl GurobiConfig {
    fn kotlin_style_thread_count() -> i32 {
        let cores = std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(1);
        if cores <= 16 {
            cores as i32
        } else if cores < 24 {
            16
        } else if cores < 32 {
            24
        } else {
            32
        }
    }

    /// 创建新配置 / Create new configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Kotlin 风格推荐默认配置 / Kotlin-style recommended defaults
    ///
    /// - `time_limit = 30s`
    /// - `mip_gap = 0.0`
    /// - `threads =` Kotlin 同款自适应策略
    pub fn recommended_defaults() -> Self {
        Self::default()
            .with_time_limit(30.0)
            .with_mip_gap(0.0)
            .with_threads(Self::kotlin_style_thread_count())
    }

    /// 数值稳健优先推荐配置 / Numeric-robustness-first recommended defaults
    ///
    /// 建议场景 / Suggested scenarios:
    /// - 系数跨度大且日志出现 `large matrix coefficients`、`small coefficients ignored`
    /// - 更关注可行性稳定性而非极限求解速度
    pub fn robust_defaults() -> Self {
        Self::recommended_defaults()
            .with_coefficient_zero_tolerance(1e-10)
            .with_numeric_focus(3)
            .with_scale_flag(2)
    }

    /// 性能优先推荐配置 / Performance-first recommended defaults
    ///
    /// 建议场景 / Suggested scenarios:
    /// - 数据尺度较健康，批量求解关注吞吐
    /// - 更关注平均时延与速度
    pub fn performance_defaults() -> Self {
        Self::recommended_defaults()
            .with_coefficient_zero_tolerance(1e-13)
            .with_numeric_focus(0)
            .with_scale_flag(-1)
    }

    /// 均衡推荐配置 / Balanced recommended defaults
    ///
    /// 建议场景 / Suggested scenarios:
    /// - 既希望提升数值稳定性，又不希望过度牺牲性能
    /// - 作为稳健优先与性能优先之间的中间档
    pub fn balanced_defaults() -> Self {
        Self::recommended_defaults()
            .with_coefficient_zero_tolerance(1e-12)
            .with_numeric_focus(1)
            .with_scale_flag(1)
    }

    /// 在当前配置基础上应用推荐默认值 / Apply recommended defaults on top of current config
    pub fn with_recommended_defaults(mut self) -> Self {
        if self.time_limit.is_none() {
            self.time_limit = Some(30.0);
        }
        if self.mip_gap.is_none() {
            self.mip_gap = Some(0.0);
        }
        if self.threads.is_none() {
            self.threads = Some(Self::kotlin_style_thread_count());
        }
        self
    }

    /// 在当前配置基础上应用数值稳健优先模板 / Apply robust profile on top of current config
    pub fn with_robust_defaults(mut self) -> Self {
        self = self.with_recommended_defaults();
        if self.coefficient_zero_tolerance <= 0.0 {
            self.coefficient_zero_tolerance = 1e-10;
        } else {
            self.coefficient_zero_tolerance = self.coefficient_zero_tolerance.max(1e-10);
        }
        if self.numeric_focus.is_none() {
            self.numeric_focus = Some(3);
        }
        if self.scale_flag.is_none() {
            self.scale_flag = Some(2);
        }
        self
    }

    /// 在当前配置基础上应用性能优先模板 / Apply performance profile on top of current config
    pub fn with_performance_defaults(mut self) -> Self {
        self = self.with_recommended_defaults();
        if self.numeric_focus.is_none() {
            self.numeric_focus = Some(0);
        }
        if self.scale_flag.is_none() {
            self.scale_flag = Some(-1);
        }
        self
    }

    /// 在当前配置基础上应用均衡模板 / Apply balanced profile on top of current config
    pub fn with_balanced_defaults(mut self) -> Self {
        self = self.with_recommended_defaults();
        if self.coefficient_zero_tolerance <= 0.0 {
            self.coefficient_zero_tolerance = 1e-12;
        } else {
            self.coefficient_zero_tolerance = self.coefficient_zero_tolerance.max(1e-12);
        }
        if self.numeric_focus.is_none() {
            self.numeric_focus = Some(1);
        }
        if self.scale_flag.is_none() {
            self.scale_flag = Some(1);
        }
        self
    }

    /// 设置时间限制 / Set time limit
    pub fn with_time_limit(mut self, seconds: f64) -> Self {
        self.time_limit = Some(seconds);
        self
    }

    /// 设置 MIP Gap / Set MIP Gap
    pub fn with_mip_gap(mut self, gap: f64) -> Self {
        self.mip_gap = Some(gap);
        self
    }

    /// 设置求解 gap（`with_mip_gap` 的易用别名）/
    /// Set solve gap (ergonomic alias for `with_mip_gap`)
    pub fn with_gap(self, gap: f64) -> Self {
        self.with_mip_gap(gap)
    }

    /// 设置最大迭代次数 / Set max iterations
    pub fn with_max_iterations(mut self, iterations: i32) -> Self {
        self.max_iterations = Some(iterations);
        self
    }

    /// 设置输出标志 / Set output flag
    pub fn with_output(mut self, output: bool) -> Self {
        self.output_flag = output;
        self
    }

    /// 设置线程数 / Set threads
    pub fn with_threads(mut self, threads: i32) -> Self {
        self.threads = Some(threads);
        self
    }

    /// 设置零系数容差 / Set zero-coefficient tolerance
    pub fn with_coefficient_zero_tolerance(mut self, tolerance: f64) -> Self {
        if tolerance.is_finite() && tolerance > 0.0 {
            self.coefficient_zero_tolerance = tolerance;
        }
        self
    }

    /// 设置数值稳定性关注级别 / Set numeric focus level
    pub fn with_numeric_focus(mut self, level: i32) -> Self {
        if (0..=3).contains(&level) {
            self.numeric_focus = Some(level);
        }
        self
    }

    /// 设置缩放策略 / Set scaling strategy
    pub fn with_scale_flag(mut self, flag: i32) -> Self {
        if (-1..=3).contains(&flag) {
            self.scale_flag = Some(flag);
        }
        self
    }

    /// 设置是否启用数值诊断 / Set whether numeric diagnostics are enabled
    pub fn with_numeric_diagnostics(mut self, enabled: bool) -> Self {
        self.enable_numeric_diagnostics = enabled;
        self
    }

    /// 设置是否自动应用推荐模板 / Set whether to auto-apply recommended numeric profile
    pub fn with_auto_apply_numeric_profile(mut self, enabled: bool) -> Self {
        self.auto_apply_numeric_profile = enabled;
        self
    }

    /// 设置日志文件 / Set log file
    pub fn with_log_file(mut self, path: &str) -> Self {
        self.log_file = Some(path.to_string());
        self
    }

    /// 设置节点限制 / Set node limit
    pub fn with_node_limit(mut self, node_limit: i32) -> Self {
        self.node_limit = Some(node_limit);
        self
    }

    /// 设置内存限制（GB）/ Set memory limit (GB)
    pub fn with_mem_limit(mut self, mem_limit: f64) -> Self {
        self.mem_limit = Some(mem_limit);
        self
    }

    /// 设置内存限制（MB）/ Set memory limit (MB)
    pub fn with_memory_limit_mb(self, memory_limit_mb: f64) -> Self {
        self.with_mem_limit(memory_limit_mb / 1024.0)
    }

    /// 设置内存限制（GB）/ Set memory limit (GB)
    pub fn with_memory_limit_gb(self, memory_limit_gb: f64) -> Self {
        self.with_mem_limit(memory_limit_gb)
    }

    /// 设置 Compute Server 地址 / Set Compute Server endpoint
    pub fn with_compute_server(mut self, compute_server: &str) -> Self {
        self.compute_server = Some(compute_server.to_string());
        self
    }

    /// 设置 Compute Server 密码 / Set Compute Server password
    pub fn with_server_password(mut self, server_password: &str) -> Self {
        self.server_password = Some(server_password.to_string());
        self
    }

    /// 设置 Compute Server 连接超时（秒）/ Set Compute Server timeout (seconds)
    pub fn with_server_timeout(mut self, server_timeout: i32) -> Self {
        self.server_timeout = Some(server_timeout);
        self
    }

    /// 设置 Compute Server 排队超时（秒）/ Set Compute Server queue timeout (seconds)
    pub fn with_cs_queue_timeout(mut self, cs_queue_timeout: f64) -> Self {
        self.cs_queue_timeout = Some(cs_queue_timeout);
        self
    }

    /// 设置无改进提前终止阈值（秒）/ Set no-improvement early-stop threshold (seconds)
    pub fn with_no_improvement_time_limit(mut self, seconds: f64) -> Self {
        self.no_improvement_time_limit = (seconds > 0.0).then_some(seconds);
        self
    }

    /// 设置改进判定阈值 / Set improvement tolerance threshold
    pub fn with_improve_threshold(mut self, threshold: f64) -> Self {
        self.improve_threshold = (threshold > 0.0).then_some(threshold);
        self
    }

    /// 设置遥测最小上报间隔（秒）/ Set minimum telemetry emit interval (seconds)
    pub fn with_telemetry_min_interval(mut self, seconds: f64) -> Self {
        self.telemetry_min_interval = (seconds > 0.0).then_some(seconds);
        self
    }

    /// 设置分阶段回调 / Set staged callback
    pub fn with_stage_callback(mut self, callback: Option<GurobiStageCallback>) -> Self {
        self.stage_callback = callback;
        self
    }

    /// 追加分阶段回调 / Append staged callback
    pub fn add_stage_callback(mut self, callback: GurobiStageCallback) -> Self {
        self.stage_callback = Some(match self.stage_callback {
            Some(existing) => Arc::new(move |status| {
                existing(status)?;
                callback(status)
            }),
            None => callback,
        });
        self
    }

    /// 追加指定阶段回调 / Append staged callback for a specific stage
    pub fn add_stage_callback_for(
        mut self,
        stage: GurobiStage,
        callback: GurobiStageCallback,
    ) -> Self {
        self = self.add_stage_callback(Arc::new(move |status| {
            if status.stage == stage {
                callback(status)?;
            }
            Ok(())
        }));
        self
    }

    /// 追加建模后回调 / Append callback for `AfterModeling`
    pub fn add_after_modeling_callback(self, callback: GurobiStageCallback) -> Self {
        self.add_stage_callback_for(GurobiStage::AfterModeling, callback)
    }

    /// 追加配置后回调 / Append callback for `Configuration`
    pub fn add_configuration_callback(self, callback: GurobiStageCallback) -> Self {
        self.add_stage_callback_for(GurobiStage::Configuration, callback)
    }

    /// 追加解分析回调 / Append callback for `AnalyzingSolution`
    pub fn add_analyzing_solution_callback(self, callback: GurobiStageCallback) -> Self {
        self.add_stage_callback_for(GurobiStage::AnalyzingSolution, callback)
    }

    /// 追加失败后回调 / Append callback for `AfterFailure`
    pub fn add_after_failure_callback(self, callback: GurobiStageCallback) -> Self {
        self.add_stage_callback_for(GurobiStage::AfterFailure, callback)
    }

    /// 设置迭代遥测回调 / Set iterative telemetry callback
    pub fn with_telemetry_callback(mut self, callback: Option<GurobiTelemetryCallback>) -> Self {
        self.telemetry_callback = callback;
        self
    }

    /// 追加迭代遥测回调 / Append iterative telemetry callback
    pub fn add_telemetry_callback(mut self, callback: GurobiTelemetryCallback) -> Self {
        self.telemetry_callback = Some(match self.telemetry_callback {
            Some(existing) => Arc::new(move |status| {
                existing(status)?;
                callback(status)
            }),
            None => callback,
        });
        self
    }

    /// 设置数值诊断回调 / Set numeric diagnostics callback
    pub fn with_numeric_diagnostics_callback(
        mut self,
        callback: Option<GurobiNumericDiagnosticsCallback>,
    ) -> Self {
        self.numeric_diagnostics_callback = callback;
        self
    }

    /// 追加数值诊断回调 / Append numeric diagnostics callback
    pub fn add_numeric_diagnostics_callback(
        mut self,
        callback: GurobiNumericDiagnosticsCallback,
    ) -> Self {
        self.numeric_diagnostics_callback = Some(match self.numeric_diagnostics_callback {
            Some(existing) => Arc::new(move |diagnostics| {
                existing(diagnostics)?;
                callback(diagnostics)
            }),
            None => callback,
        });
        self
    }

    /// 设置原生回调（覆盖语义）/ Set native callback (override semantics)
    pub fn with_native_callback(mut self, callback: Option<GurobiNativeCallback>) -> Self {
        self.native_callback = callback;
        self
    }

    /// 设置原生观察回调集合 / Set native observer callback list
    pub fn with_native_observers(mut self, observers: Vec<GurobiNativeObserver>) -> Self {
        self.native_observers = observers;
        self
    }

    /// 追加原生观察回调（可多播）/ Append native observer callback (multicast)
    pub fn add_native_observer(mut self, observer: GurobiNativeObserver) -> Self {
        self.native_observers.push(observer);
        self
    }

    /// 追加原生回调（覆盖语义）/ Append native callback (override semantics)
    ///
    /// 说明 / Note:
    /// `grb::callback::Where<'_>` 按值消费且不可克隆，不能安全地把同一上下文分发给多个回调。
    /// 因此本接口采用“最后一次设置生效”语义。
    pub fn add_native_callback(mut self, callback: GurobiNativeCallback) -> Self {
        // `grb::callback::Where<'_>` 为按值消费且不可克隆，无法像其他回调一样安全链式复用。
        // `grb::callback::Where<'_>` is consumed by value and not clonable, so we cannot
        // safely chain multiple native callbacks like other callback types.
        self.native_callback = Some(callback);
        self
    }

    /// 设置环境创建回调 / Set environment creation callback
    pub fn with_env_callback(mut self, callback: Option<GurobiEnvCallback>) -> Self {
        self.env_callback = callback;
        self
    }

    /// 追加环境创建回调 / Append environment creation callback
    pub fn add_env_callback(mut self, callback: GurobiEnvCallback) -> Self {
        self.env_callback = Some(match self.env_callback {
            Some(existing) => Arc::new(move |env| {
                existing(env)?;
                callback(env)
            }),
            None => callback,
        });
        self
    }
}
