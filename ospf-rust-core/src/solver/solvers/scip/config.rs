use std::sync::Arc;
use russcip::EventMask;
use crate::solver::SolverConfig;
use super::{
    SCIPNativeCallback, SCIPNativeObserver, SCIPSnapshotObserver, SCIPStage, SCIPStageCallback,
    SCIPTelemetryCallback,
};

/// SCIP presolving mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresolvingMode {
    /// Off.
    Off,
    /// Fast.
    Fast,
    /// Medium.
    Medium,
    /// Aggressive.
    Aggressive,
}

/// SCIP solver configuration.
#[derive(Clone)]
pub struct SCIPConfig {
    /// Time limit (seconds).
    pub time_limit: Option<f64>,
    /// MIP gap tolerance.
    pub mip_gap: Option<f64>,
    /// Maximum iterations.
    pub max_iterations: Option<i64>,
    /// Output flag.
    pub output_flag: bool,
    /// Number of threads.
    pub threads: Option<i32>,
    /// Log file path.
    pub log_file: Option<String>,
    /// Node limit.
    pub node_limit: Option<i64>,
    /// Memory limit (MB).
    pub mem_limit: Option<f64>,
    /// Display frequency.
    pub display_freq: Option<i32>,
    /// Presolving mode.
    pub presolving: Option<PresolvingMode>,
    /// Heuristics priority.
    pub heuristics_priority: Option<i32>,
    /// No-improvement early-stop threshold (seconds).
    pub no_improvement_time_limit: Option<f64>,
    /// Improvement tolerance threshold.
    pub improvement_tolerance: Option<f64>,
    /// Minimum telemetry emit interval (seconds).
    pub telemetry_min_interval: Option<f64>,
    /// 原生事件掩码 / Native event mask.
    pub native_event_mask: EventMask,
    /// Staged callback.
    pub stage_callback: Option<SCIPStageCallback>,
    /// Telemetry callback.
    pub telemetry_callback: Option<SCIPTelemetryCallback>,
    /// Snapshot observers (multicast).
    pub snapshot_observers: Vec<SCIPSnapshotObserver>,
    /// Native callback (override semantics).
    pub native_callback: Option<SCIPNativeCallback>,
    /// Native observers (multicast).
    pub native_observers: Vec<SCIPNativeObserver>,
}

impl std::fmt::Debug for SCIPConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SCIPConfig")
            .field("time_limit", &self.time_limit)
            .field("mip_gap", &self.mip_gap)
            .field("max_iterations", &self.max_iterations)
            .field("output_flag", &self.output_flag)
            .field("threads", &self.threads)
            .field("log_file", &self.log_file)
            .field("node_limit", &self.node_limit)
            .field("mem_limit", &self.mem_limit)
            .field("display_freq", &self.display_freq)
            .field("presolving", &self.presolving)
            .field("heuristics_priority", &self.heuristics_priority)
            .field("no_improvement_time_limit", &self.no_improvement_time_limit)
            .field("improvement_tolerance", &self.improvement_tolerance)
            .field("telemetry_min_interval", &self.telemetry_min_interval)
            .field("native_event_mask", &self.native_event_mask)
            .field("stage_callback_registered", &self.stage_callback.is_some())
            .field(
                "telemetry_callback_registered",
                &self.telemetry_callback.is_some(),
            )
            .field("snapshot_observers_count", &self.snapshot_observers.len())
            .field(
                "native_callback_registered",
                &self.native_callback.is_some(),
            )
            .field("native_observers_count", &self.native_observers.len())
            .finish()
    }
}

impl Default for SCIPConfig {
    fn default() -> Self {
        Self {
            time_limit: None,
            mip_gap: None,
            max_iterations: None,
            output_flag: true,
            threads: None,
            log_file: None,
            node_limit: None,
            mem_limit: None,
            display_freq: None,
            presolving: None,
            heuristics_priority: None,
            no_improvement_time_limit: None,
            improvement_tolerance: None,
            telemetry_min_interval: None,
            native_event_mask: Self::default_native_event_mask(),
            stage_callback: None,
            telemetry_callback: None,
            snapshot_observers: Vec::new(),
            native_callback: None,
            native_observers: Vec::new(),
        }
    }
}

impl From<&SolverConfig> for SCIPConfig {
    fn from(config: &SolverConfig) -> Self {
        let mut scip_config = SCIPConfig::new();
        scip_config.time_limit = config.time_limit.map(|duration| duration.as_secs_f64());
        scip_config.mip_gap = config.mip_gap;
        scip_config.max_iterations = config
            .iteration_limit
            .map(|limit| limit.min(i64::MAX as usize) as i64);
        scip_config.output_flag = config.verbose;
        scip_config.threads = config
            .threads
            .map(|threads| threads.min(i32::MAX as usize) as i32);
        scip_config.node_limit = config
            .node_limit
            .map(|limit| limit.min(i64::MAX as usize) as i64);
        scip_config.mem_limit = config.memory_limit.map(|limit_mb| limit_mb as f64);
        scip_config.no_improvement_time_limit = config
            .no_improvement_time_limit
            .map(|duration| duration.as_secs_f64());
        scip_config.improvement_tolerance = config.improve_threshold;
        scip_config
    }
}

impl From<SolverConfig> for SCIPConfig {
    fn from(config: SolverConfig) -> Self {
        Self::from(&config)
    }
}

impl SCIPConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// 默认原生事件掩码 / Default native event mask.
    pub fn default_native_event_mask() -> EventMask {
        EventMask::NODE_EVENT | EventMask::LP_EVENT | EventMask::SOL_EVENT
    }

    /// 推荐 LP/子问题配置 / Recommended LP/subproblem configuration.
    ///
    /// 使用单线程、关闭 presolving、关闭 heuristics，便于列生成/Benders 子问题获取稳定对偶。
    /// Uses one thread, disables presolving, and disables heuristics for stable duals in
    /// column-generation/Benders subproblems.
    pub fn recommended_lp_subproblem_defaults() -> Self {
        Self::new().with_lp_subproblem_defaults()
    }

    /// 在当前配置上应用推荐 LP/子问题配置 / Apply recommended LP/subproblem settings.
    ///
    /// 保留未相关字段（例如 time limit、gap、callback），只覆盖线程、presolving 和 heuristics。
    /// Keeps unrelated fields such as time limit, gap, and callbacks, and only overrides threads,
    /// presolving, and heuristics.
    pub fn with_lp_subproblem_defaults(mut self) -> Self {
        self.threads = Some(1);
        self.presolving = Some(PresolvingMode::Off);
        self.heuristics_priority = Some(0);
        self
    }

    pub fn with_time_limit(mut self, seconds: f64) -> Self {
        self.time_limit = Some(seconds);
        self
    }

    pub fn with_mip_gap(mut self, gap: f64) -> Self {
        self.mip_gap = Some(gap);
        self
    }

    /// 设置求解 gap（`with_mip_gap` 的易用别名）/
    /// Set solve gap (ergonomic alias for `with_mip_gap`).
    pub fn with_gap(self, gap: f64) -> Self {
        self.with_mip_gap(gap)
    }

    pub fn with_max_iterations(mut self, iterations: i64) -> Self {
        self.max_iterations = Some(iterations);
        self
    }

    pub fn with_output(mut self, output: bool) -> Self {
        self.output_flag = output;
        self
    }

    pub fn with_threads(mut self, threads: i32) -> Self {
        self.threads = Some(threads);
        self
    }

    pub fn with_log_file(mut self, path: &str) -> Self {
        self.log_file = Some(path.to_string());
        self
    }

    pub fn with_presolving(mut self, mode: PresolvingMode) -> Self {
        self.presolving = Some(mode);
        self
    }

    pub fn with_node_limit(mut self, node_limit: i64) -> Self {
        self.node_limit = Some(node_limit);
        self
    }

    pub fn with_mem_limit(mut self, mem_limit: f64) -> Self {
        self.mem_limit = Some(mem_limit);
        self
    }

    /// 设置内存限制（MB）/ Set memory limit (MB).
    pub fn with_memory_limit_mb(self, memory_limit_mb: f64) -> Self {
        self.with_mem_limit(memory_limit_mb)
    }

    /// 设置内存限制（GB）/ Set memory limit (GB).
    pub fn with_memory_limit_gb(self, memory_limit_gb: f64) -> Self {
        self.with_mem_limit(memory_limit_gb * 1024.0)
    }

    pub fn with_display_freq(mut self, display_freq: i32) -> Self {
        self.display_freq = Some(display_freq);
        self
    }

    pub fn with_heuristics_priority(mut self, heuristics_priority: i32) -> Self {
        self.heuristics_priority = Some(heuristics_priority);
        self
    }

    pub fn with_no_improvement_time_limit(mut self, seconds: f64) -> Self {
        self.no_improvement_time_limit = (seconds > 0.0).then_some(seconds);
        self
    }

    pub fn with_improvement_tolerance(mut self, tolerance: f64) -> Self {
        self.improvement_tolerance = (tolerance > 0.0).then_some(tolerance);
        self
    }

    /// 设置改进判定阈值（`with_improvement_tolerance` 的易用别名）/
    /// Set improvement threshold (ergonomic alias for `with_improvement_tolerance`).
    pub fn with_improve_threshold(self, threshold: f64) -> Self {
        self.with_improvement_tolerance(threshold)
    }

    pub fn with_telemetry_min_interval(mut self, seconds: f64) -> Self {
        self.telemetry_min_interval = (seconds > 0.0).then_some(seconds);
        self
    }

    /// 设置原生事件掩码 / Set native event mask.
    pub fn with_native_event_mask(mut self, event_mask: EventMask) -> Self {
        self.native_event_mask = event_mask;
        self
    }

    pub fn with_stage_callback(mut self, callback: Option<SCIPStageCallback>) -> Self {
        self.stage_callback = callback;
        self
    }

    pub fn add_stage_callback(mut self, callback: SCIPStageCallback) -> Self {
        self.stage_callback = Some(match self.stage_callback {
            Some(existing) => Arc::new(move |status| {
                existing(status)?;
                callback(status)
            }),
            None => callback,
        });
        self
    }

    pub fn add_stage_callback_for(mut self, stage: SCIPStage, callback: SCIPStageCallback) -> Self {
        self = self.add_stage_callback(Arc::new(move |status| {
            if status.stage == stage {
                callback(status)?;
            }
            Ok(())
        }));
        self
    }

    pub fn add_after_modeling_callback(self, callback: SCIPStageCallback) -> Self {
        self.add_stage_callback_for(SCIPStage::AfterModeling, callback)
    }

    pub fn add_configuration_callback(self, callback: SCIPStageCallback) -> Self {
        self.add_stage_callback_for(SCIPStage::Configuration, callback)
    }

    pub fn add_analyzing_solution_callback(self, callback: SCIPStageCallback) -> Self {
        self.add_stage_callback_for(SCIPStage::AnalyzingSolution, callback)
    }

    pub fn add_after_failure_callback(self, callback: SCIPStageCallback) -> Self {
        self.add_stage_callback_for(SCIPStage::AfterFailure, callback)
    }

    pub fn with_telemetry_callback(mut self, callback: Option<SCIPTelemetryCallback>) -> Self {
        self.telemetry_callback = callback;
        self
    }

    pub fn add_telemetry_callback(mut self, callback: SCIPTelemetryCallback) -> Self {
        self.telemetry_callback = Some(match self.telemetry_callback {
            Some(existing) => Arc::new(move |status| {
                existing(status)?;
                callback(status)
            }),
            None => callback,
        });
        self
    }

    pub fn with_snapshot_observers(mut self, observers: Vec<SCIPSnapshotObserver>) -> Self {
        self.snapshot_observers = observers;
        self
    }

    pub fn add_snapshot_observer(mut self, observer: SCIPSnapshotObserver) -> Self {
        self.snapshot_observers.push(observer);
        self
    }

    /// 设置原生回调（覆盖语义）/ Set native callback (override semantics).
    pub fn with_native_callback(mut self, callback: Option<SCIPNativeCallback>) -> Self {
        self.native_callback = callback;
        self
    }

    /// 设置原生回调和事件掩码 / Set native callback and event mask.
    pub fn with_native_callback_with_event_mask(
        mut self,
        event_mask: EventMask,
        callback: SCIPNativeCallback,
    ) -> Self {
        self.native_event_mask = event_mask;
        self.native_callback = Some(callback);
        self
    }

    /// 追加原生回调（覆盖语义）/ Append native callback (override semantics).
    pub fn add_native_callback(mut self, callback: SCIPNativeCallback) -> Self {
        self.native_callback = Some(callback);
        self
    }

    /// 设置原生观察回调集合 / Set native observers.
    pub fn with_native_observers(mut self, observers: Vec<SCIPNativeObserver>) -> Self {
        self.native_observers = observers;
        self
    }

    /// 追加原生观察回调（可多播）/ Append native observer (multicast).
    pub fn add_native_observer(mut self, observer: SCIPNativeObserver) -> Self {
        self.native_observers.push(observer);
        self
    }
}

/// Rust-style alias for [`SCIPConfig`].
/// [`SCIPConfig`] 的 Rust 风格别名。
pub type ScipConfig = SCIPConfig;
