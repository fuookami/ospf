//! 活动领域输入及完整候选。 / Activity domain inputs and complete choices.

use std::collections::{BTreeMap, BTreeSet};
use time::{Duration, OffsetDateTime};
use ospf_rust_quantities::{Quantity, unit::Unit};
use crate::domain::common::{ExecutorId, TaskId};
use crate::infrastructure::TimeRange;
use crate::GanttResult;

/// 时变资源容量覆盖，覆盖之间不允许重叠。 / Non-overlapping time-varying resource capacity override.
#[derive(Debug, Clone)]
pub struct ActivityCapacity<V> {
    /// 半开区间 / Half-open interval
    pub time: TimeRange,
    /// 区间容量 / Capacity within the interval
    pub capacity: Quantity<V, Unit>,
}

/// 多活动可同时占用的具名资源。 / Named resource shared concurrently by activities.
#[derive(Debug, Clone)]
pub struct ActivityResource<V> {
    /// 执行器身份 / Executor identity
    pub id: ExecutorId,
    /// 默认容量与单位 / Default capacity and unit
    pub capacity: Quantity<V, Unit>,
    /// 不可用区间，优先于容量覆盖 / Unavailability, taking precedence over capacity overrides
    pub unavailable_times: Vec<TimeRange>,
    /// 时变容量覆盖 / Time-varying capacity overrides
    pub capacities: Vec<ActivityCapacity<V>>,
}

/// 模式内部资源占用段，不自动暂停。 / Resource phase within a mode, without automatic preemption.
#[derive(Debug, Clone)]
pub struct ActivityResourceUse<V> {
    /// 资源身份 / Resource identity
    pub resource: ExecutorId,
    /// 非负占用量 / Non-negative usage
    pub quantity: Quantity<V, Unit>,
    /// 相对开始偏移 / Offset from activity start
    pub from: Duration,
    /// 相对结束偏移，None 为模式结束 / End offset; None denotes mode completion
    pub until: Option<Duration>,
}

/// 模式内部有符号物料事件。 / Signed material event within a mode.
#[derive(Debug, Clone)]
pub struct ActivityMaterialFlow<V> {
    /// 物料身份 / Material identity
    pub material: String,
    /// 正入库、负消耗 / Positive supply, negative consumption
    pub quantity: Quantity<V, Unit>,
    /// 相对起点偏移，含模式结束点 / Offset from start, including completion
    pub at: Duration,
}

/// 完整有限开始域的不可抢占模式。 / Non-preemptive mode with a complete finite start domain.
#[derive(Debug, Clone)]
pub struct ActivityMode<V> {
    /// 活动内唯一模式身份 / Unique mode identity within the activity
    pub id: String,
    /// 非负执行时长 / Non-negative duration
    pub duration: Duration,
    /// 完整允许开始域，不是搜索预算 / Complete allowed starts, not a search budget
    pub starts: Vec<OffsetDateTime>,
    /// 联合分阶段资源 / Joint phased resources
    pub resources: Vec<ActivityResourceUse<V>>,
    /// 有符号完整模式费用 / Signed complete mode cost
    pub cost: Quantity<V, Unit>,
    /// 内部库存事件 / Internal inventory events
    pub material_flows: Vec<ActivityMaterialFlow<V>>,
}

/// 待排活动，只有显式取消费才允许整单取消。 / Pending activity, cancellable only with an explicit cancellation cost.
#[derive(Debug, Clone)]
pub struct Activity<V> {
    /// 任务身份 / Task identity
    pub id: TaskId,
    /// 替代执行模式 / Alternative execution modes
    pub modes: Vec<ActivityMode<V>>,
    /// 非负整单取消费；None 表示必做 / Non-negative whole-order cancellation cost; None means mandatory
    pub cancellation_cost: Option<Quantity<V, Unit>>,
}

/// 完整候选稳定身份。 / Stable complete-choice identity.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActivityChoiceKey {
    /// 任务身份 / Task identity
    pub task: TaskId,
    /// 模式身份 / Mode identity
    pub mode: String,
    /// 允许开始时刻 / Allowed start time
    pub start: OffsetDateTime,
}

/// 绝对时间的资源占用。 / Absolute-time resource reservation.
#[derive(Debug, Clone)]
pub struct ActivityReservation<V> {
    /// 资源身份 / Resource identity
    pub resource: ExecutorId,
    /// 半开资源段 / Half-open resource phase
    pub time: TimeRange,
    /// 占用量 / Usage
    pub quantity: Quantity<V, Unit>,
}

/// 保留全部内部资源段的完整分配。 / Complete placement retaining all internal resource phases.
#[derive(Debug, Clone)]
pub struct ActivityPlacement<V> {
    /// 候选身份 / Choice identity
    pub key: ActivityChoiceKey,
    /// 实际或预计活动时间 / Actual or expected activity interval
    pub time: TimeRange,
    /// 内部资源段 / Internal resource phases
    pub reservations: Vec<ActivityReservation<V>>,
}

/// 依赖的时间端点。 / Temporal dependency endpoint.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityEndpoint {
    /// 开始 / Start
    Start,
    /// 完成 / Finish
    Finish,
}

/// 独立于时间的存在性语义。 / Presence semantics independent of timing.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityDependencyPresence {
    /// 仅共同执行时约束 / Constrain only when both are present
    WhenBothPresent,
    /// 后继存在要求前驱存在 / Successor presence requires predecessor presence
    RequirePredecessor,
    /// 共同执行或共同取消 / Both present or both cancelled
    RequireBoth,
}

/// FS/SS/FF/SF 端点依赖，允许负间隔和同步环。 / FS/SS/FF/SF dependency permitting negative lags and synchronization cycles.
#[derive(Debug, Clone)]
pub struct ActivityDependency {
    /// 前驱任务 / Predecessor task
    pub predecessor: TaskId,
    /// 后继任务 / Successor task
    pub successor: TaskId,
    /// 前驱端点 / Predecessor endpoint
    pub from: ActivityEndpoint,
    /// 后继端点 / Successor endpoint
    pub to: ActivityEndpoint,
    /// 最小间隔 / Minimum lag
    pub minimum: Duration,
    /// 可选最大间隔 / Optional maximum lag
    pub maximum: Option<Duration>,
    /// 存在性规则 / Presence rule
    pub presence: ActivityDependencyPresence,
}

/// 基线选择与一次性变更费用。 / Baseline choice and one-off change costs.
#[derive(Debug, Clone)]
pub struct ActivityBaseline<V> {
    /// 原选择，非冻结时允许域外 / Original choice, allowed outside the domain when not frozen
    pub choice: ActivityChoiceKey,
    /// 必须保持原选择 / Whether the original choice must be retained
    pub frozen: bool,
    /// 非负开始变更费用 / Non-negative start-change cost
    pub start_change_cost: Quantity<V, Unit>,
    /// 非负模式变更费用 / Non-negative mode-change cost
    pub mode_change_cost: Quantity<V, Unit>,
}

/// 不重新选择和收费的执行事实类型。 / Execution fact kind, never reselected or recharged.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityExecutionStatus {
    /// 已完成 / Completed
    Completed,
    /// 执行中 / Running
    Running,
}

/// 固定事实，执行中只声明剩余资源段。 / Fixed fact; running facts declare only remaining resource phases.
#[derive(Debug, Clone)]
pub struct ActivityExecution<V> {
    /// 完整实际或预计分配 / Complete actual or expected placement
    pub placement: ActivityPlacement<V>,
    /// 执行状态 / Execution status
    pub status: ActivityExecutionStatus,
}

/// 独立具名库存，期初在起点事件之前。 / Named inventory whose opening precedes horizon-start events.
#[derive(Debug, Clone)]
pub struct ActivityMaterial<V> {
    /// 物料身份 / Material identity
    pub id: String,
    /// 期初量与统一单位 / Opening stock and canonical unit
    pub initial: Quantity<V, Unit>,
    /// 非负下界，None 为零 / Non-negative minimum; None means zero
    pub minimum: Option<Quantity<V, Unit>>,
    /// 可选库存上界 / Optional storage maximum
    pub maximum: Option<Quantity<V, Unit>>,
}

/// 与选择无关的未来物料事件。 / Fixed future material event independent of choices.
#[derive(Debug, Clone)]
pub struct ActivityMaterialEvent<V> {
    /// 物料身份 / Material identity
    pub material: String,
    /// 包含 horizon 两端的时间 / Time including both horizon endpoints
    pub time: OffsetDateTime,
    /// 有符号库存变化 / Signed inventory change
    pub quantity: Quantity<V, Unit>,
}

/// 具备显式单位的业务扩展行。 / Business extension row with an explicit unit.
#[derive(Debug, Clone)]
pub struct ActivityRow<V> {
    /// 稳定行身份 / Stable row identity
    pub id: String,
    /// 系数及边界单位 / Coefficient and bound unit
    pub unit: Unit,
    /// 候选系数，省略为零 / Choice coefficients, zero when omitted
    pub coefficients: BTreeMap<ActivityChoiceKey, Quantity<V, Unit>>,
    /// 显式取消系数 / Explicit cancellation coefficients
    pub cancellation_coefficients: BTreeMap<TaskId, Quantity<V, Unit>>,
    /// 可选下界 / Optional lower bound
    pub lower: Option<Quantity<V, Unit>>,
    /// 可选上界 / Optional upper bound
    pub upper: Option<Quantity<V, Unit>>,
}

/// 无模型注册态的领域输入；准备时转移所有权并冻结快照。
/// Domain input without model state; preparation takes ownership and freezes the snapshot.
#[derive(Debug, Clone)]
pub struct ActivitySchedulingProblem<V> {
    /// 有限优化区间 / Finite optimization horizon
    pub horizon: TimeRange,
    /// 待排活动 / Pending activities
    pub activities: Vec<Activity<V>>,
    /// 联合资源 / Joint resources
    pub resources: Vec<ActivityResource<V>>,
    /// 时间及存在性关系 / Temporal and presence relations
    pub dependencies: Vec<ActivityDependency>,
    /// 基线与冻结 / Baselines and freezing
    pub baselines: Vec<ActivityBaseline<V>>,
    /// 固定执行事实 / Fixed execution facts
    pub executions: Vec<ActivityExecution<V>>,
    /// 额外业务行 / Additional business rows
    pub rows: Vec<ActivityRow<V>>,
    /// 统一费用单位 / Canonical cost unit
    pub cost_unit: Unit,
    /// 物料与硬库存边界 / Materials and hard inventory bounds
    pub materials: Vec<ActivityMaterial<V>>,
    /// 固定未来库存事件 / Fixed future inventory events
    pub material_events: Vec<ActivityMaterialEvent<V>>,
}

/// 只读完整候选额外费用策略；每候选准备时调用一次，不承诺回滚外部副作用。
/// Read-only complete-choice cost policy, called once per prepared choice; external side effects are not rolled back.
pub trait ActivityCostPolicy<V>: Send + Sync {
    /// 计算完整候选额外费用，允许有限负值。 / Compute extra complete-choice cost, permitting finite negative values.
    fn additional_cost(&self, placement: &ActivityPlacement<V>) -> GanttResult<Quantity<V, Unit>>;
}

impl<V, F> ActivityCostPolicy<V> for F
where
    F: Fn(&ActivityPlacement<V>) -> GanttResult<Quantity<V, Unit>> + Send + Sync,
{
    fn additional_cost(&self, placement: &ActivityPlacement<V>) -> GanttResult<Quantity<V, Unit>> {
        self(placement)
    }
}

/// 同点原子结算后的原域库存。 / Native inventory after atomic simultaneous settlement.
#[derive(Debug, Clone)]
pub struct ActivityInventoryLevel<V> {
    /// 事件时间 / Event time
    pub time: OffsetDateTime,
    /// 结算后库存 / Post-settlement stock
    pub quantity: Quantity<V, Unit>,
}

/// 不读取模型行的库存重放轨迹。 / Inventory replay trail independent of model rows.
#[derive(Debug, Clone)]
pub struct ActivityMaterialInventory<V> {
    /// 物料身份 / Material identity
    pub material: String,
    /// 起点事件之前的期初量 / Opening stock before horizon-start events
    pub initial: Quantity<V, Unit>,
    /// 包含 horizon 两端的全部实际事件后库存 / Post-event levels including both horizon endpoints
    pub levels: Vec<ActivityInventoryLevel<V>>,
}

/// 可结构化定位的业务违反种类。 / Structurally locatable business violation kind.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityViolationKind {
    /// 未知候选或取消身份 / Unknown choice or cancellation identity
    UnknownChoice,
    /// 候选重复或多选 / Duplicate or multiple choices
    DuplicateChoice,
    /// 必做缺失或非法取消 / Missing mandatory execution or illegal cancellation
    Selection,
    /// 冻结选择被改变 / Frozen choice changed
    Frozen,
    /// 依赖存在性或端点间隔 / Dependency presence or endpoint lag
    Dependency,
    /// 资源区间超载 / Resource interval overload
    Resource,
    /// 期初或时序库存越界 / Opening or event inventory violation
    Inventory,
    /// 业务扩展行违反 / Business extension row violation
    Row,
}

/// 独立复核违反项。 / Independent-validation violation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityViolation {
    /// 规则族 / Rule family
    pub kind: ActivityViolationKind,
    /// 原始业务身份 / Original business identity
    pub subject: String,
    /// 可选事件时点 / Optional event instant
    pub time: Option<OffsetDateTime>,
    /// 诊断详情 / Diagnostic detail
    pub message: String,
}

/// 不把业务不可行误作程序错误的复核结果。 / Validation result distinguishing business infeasibility from program errors.
#[derive(Debug, Clone)]
pub struct ActivityScheduleValidation {
    /// 全部结构化违反项 / All structured violations
    pub violations: Vec<ActivityViolation>,
}

impl ActivityScheduleValidation {
    /// 是否满足全部原域业务规则。 / Whether all native-domain business rules hold.
    pub fn feasible(&self) -> bool { self.violations.is_empty() }
}

/// 声明有限域内的完整结果；本类型不产生最优证书。 / Complete declared-finite-domain result; this type produces no optimality certificate.
#[derive(Debug, Clone)]
pub struct ActivitySchedule<V> {
    /// 待排活动的完整选中分配 / Complete selected pending placements
    pub placements: Vec<ActivityPlacement<V>>,
    /// 未重新选择或收费的执行事实 / Execution facts, neither reselected nor recharged
    pub executions: Vec<ActivityExecution<V>>,
    /// 显式整单取消集合 / Explicit whole-order cancellations
    pub cancelled: BTreeSet<TaskId>,
    /// 原 V 域领域费用，不是含下游扩展的后端目标 / Native-V domain cost, not the backend objective including downstream extensions
    pub cost: Quantity<V, Unit>,
    /// 完整声明候选域 / Complete declared candidate domain
    pub candidate_scope: BTreeSet<ActivityChoiceKey>,
    /// 独立原域库存轨迹 / Independent native-domain inventory trails
    pub material_inventory: Vec<ActivityMaterialInventory<V>>,
}
