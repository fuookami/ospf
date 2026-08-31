use std::sync::Arc;
use std::time::Duration;

use crate::error::Result;
use crate::model::ObjectiveCategory;
use crate::solver::SolverStatus;

/// SCIP staged callback points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SCIPStage {
    /// After modeling.
    AfterModeling,
    /// After configuration.
    Configuration,
    /// While analyzing solution.
    AnalyzingSolution,
    /// After failure.
    AfterFailure,
}

/// SCIP staged status snapshot.
#[derive(Debug, Clone)]
pub struct SCIPStageStatus {
    /// Current stage.
    pub stage: SCIPStage,
    /// Solver status if available.
    pub solver_status: Option<SolverStatus>,
    /// Elapsed solve time.
    pub solve_time: Duration,
    /// Current objective value.
    pub objective_value: Option<f64>,
    /// Current best bound.
    pub best_bound: Option<f64>,
    /// Current MIP gap.
    pub mip_gap: Option<f64>,
    /// LP iteration count.
    pub iterations: Option<usize>,
    /// Node count.
    pub node_count: Option<usize>,
}

/// SCIP staged callback.
pub type SCIPStageCallback = Arc<dyn Fn(&SCIPStageStatus) -> Result<()> + Send + Sync>;

/// SCIP telemetry snapshot.
#[derive(Debug, Clone)]
pub struct SCIPTelemetryStatus {
    /// Elapsed solve time.
    pub solve_time: Duration,
    /// 目标方向 / Objective direction.
    pub objective_category: ObjectiveCategory,
    /// 首个 incumbent 目标值 / First incumbent objective value.
    pub initial_objective_value: Option<f64>,
    /// Current objective value.
    pub objective_value: Option<f64>,
    /// 当前 incumbent 解向量 / Current incumbent solution vector.
    pub incumbent_solution: Option<Vec<f64>>,
    /// Current best bound.
    pub best_bound: Option<f64>,
    /// Current MIP gap.
    pub mip_gap: Option<f64>,
    /// LP iteration count.
    pub iterations: Option<usize>,
    /// Node count.
    pub node_count: Option<usize>,
}

/// SCIP telemetry callback.
pub type SCIPTelemetryCallback = Arc<dyn Fn(&SCIPTelemetryStatus) -> Result<()> + Send + Sync>;

/// SCIP snapshot observer control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SCIPSnapshotControl {
    /// Continue solving.
    Continue,
    /// Request interrupt.
    Interrupt,
}

/// SCIP snapshot observer callback.
pub type SCIPSnapshotObserver =
    Arc<dyn Fn(&SCIPTelemetryStatus) -> Result<SCIPSnapshotControl> + Send + Sync>;

/// SCIP 原生事件快照 / SCIP native event snapshot.
#[derive(Debug, Clone)]
pub struct SCIPNativeSnapshot {
    /// 语义化事件位置 / Semantic event location.
    pub where_point: SCIPNativeWhere,
    /// 事件掩码位 / Event mask bits.
    pub event_mask_bits: u64,
    /// 事件处理器名称 / Event handler name.
    pub handler_name: String,
    /// 累计耗时 / Elapsed solve time.
    pub solve_time: Duration,
    /// 目标方向 / Objective direction.
    pub objective_category: ObjectiveCategory,
    /// 首个 incumbent 目标值 / First incumbent objective value.
    pub initial_objective_value: Option<f64>,
    /// 当前目标值 / Current objective value.
    pub objective_value: Option<f64>,
    /// 当前 incumbent 解向量 / Current incumbent solution vector.
    pub incumbent_solution: Option<Vec<f64>>,
    /// 当前最优界 / Current best bound.
    pub best_bound: Option<f64>,
    /// 当前 MIP gap / Current MIP gap.
    pub mip_gap: Option<f64>,
    /// 当前迭代次数 / Current iteration count.
    pub iterations: Option<usize>,
    /// 当前已探索节点数 / Current explored node count.
    pub node_count: Option<usize>,
}

/// SCIP 原生回调语义化事件位置 / SCIP native callback semantic location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SCIPNativeWhere {
    /// 节点相关事件 / Node-related event.
    Node,
    /// LP 相关事件 / LP-related event.
    Lp,
    /// 解相关事件 / Solution-related event.
    Solution,
    /// 变量相关事件 / Variable-related event.
    Variable,
    /// 行相关事件 / Row-related event.
    Row,
    /// 预处理轮次事件 / Presolve-round event.
    PresolveRound,
    /// 同步事件 / Sync event.
    Sync,
    /// 其他 / Other.
    Other,
}

impl SCIPNativeSnapshot {
    /// 判断是否命中事件掩码 / Check whether snapshot matches event mask bits.
    pub fn matches_mask_bits(&self, mask_bits: u64) -> bool {
        self.event_mask_bits & mask_bits != 0
    }
}

/// SCIP 原生观察回调控制信号 / SCIP native observer callback control signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SCIPNativeControl {
    /// 继续求解 / Continue solving.
    Continue,
    /// 请求中断 / Request interrupt.
    Interrupt,
}

/// SCIP 原生观察回调（可多播）/ SCIP native observer callback (multicast).
pub type SCIPNativeObserver =
    Arc<dyn Fn(&SCIPNativeSnapshot) -> Result<SCIPNativeControl> + Send + Sync>;

/// SCIP 原生回调（覆盖语义）/ SCIP native callback (override semantics).
pub type SCIPNativeCallback =
    Arc<dyn Fn(&SCIPNativeSnapshot) -> Result<SCIPNativeControl> + Send + Sync>;

/// Rust-style alias for [`SCIPStage`].
/// [`SCIPStage`] 的 Rust 风格别名。
pub type ScipStage = SCIPStage;

/// Rust-style alias for [`SCIPStageStatus`].
/// [`SCIPStageStatus`] 的 Rust 风格别名。
pub type ScipStageStatus = SCIPStageStatus;

/// Rust-style alias for [`SCIPStageCallback`].
/// [`SCIPStageCallback`] 的 Rust 风格别名。
pub type ScipStageCallback = SCIPStageCallback;

/// Rust-style alias for [`SCIPTelemetryStatus`].
/// [`SCIPTelemetryStatus`] 的 Rust 风格别名。
pub type ScipTelemetryStatus = SCIPTelemetryStatus;

/// Rust-style alias for [`SCIPTelemetryCallback`].
/// [`SCIPTelemetryCallback`] 的 Rust 风格别名。
pub type ScipTelemetryCallback = SCIPTelemetryCallback;

/// Rust-style alias for [`SCIPSnapshotControl`].
/// [`SCIPSnapshotControl`] 的 Rust 风格别名。
pub type ScipSnapshotControl = SCIPSnapshotControl;

/// Rust-style alias for [`SCIPSnapshotObserver`].
/// [`SCIPSnapshotObserver`] 的 Rust 风格别名。
pub type ScipSnapshotObserver = SCIPSnapshotObserver;

/// Rust-style alias for [`SCIPNativeSnapshot`].
/// [`SCIPNativeSnapshot`] 的 Rust 风格别名。
pub type ScipNativeSnapshot = SCIPNativeSnapshot;

/// Rust-style alias for [`SCIPNativeWhere`].
/// [`SCIPNativeWhere`] 的 Rust 风格别名。
pub type ScipNativeWhere = SCIPNativeWhere;

/// Rust-style alias for [`SCIPNativeControl`].
/// [`SCIPNativeControl`] 的 Rust 风格别名。
pub type ScipNativeControl = SCIPNativeControl;

/// Rust-style alias for [`SCIPNativeObserver`].
/// [`SCIPNativeObserver`] 的 Rust 风格别名。
pub type ScipNativeObserver = SCIPNativeObserver;

/// Rust-style alias for [`SCIPNativeCallback`].
/// [`SCIPNativeCallback`] 的 Rust 风格别名。
pub type ScipNativeCallback = SCIPNativeCallback;
