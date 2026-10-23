//! 消费实际恢复事件与完整停场间隔的有限机场流控。 / Finite airport flow controls consuming actual recovery events and complete stay intervals.

use std::collections::BTreeSet;
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use crate::framework::demo4::domain::task::model::{FlightTaskBunch, FlightTaskRecovery, FlightType, RecoveryError};

/// 流量控制场景 / Flow control scene (对齐 Kotlin FlowControlScene)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlowControlScene {
    /// 出场控制 / Departure control
    Departure,
    /// 进场控制 / Arrival control
    Arrival,
    /// 出进场控制 / Departure and arrival control
    DepartureArrival,
    /// 停场控制 / Stay control
    Stay,
}

impl FlowControlScene {
    /// 按实际事件判断；Stay 要求完整停场间隔被窗口包含，不表示任意重叠占位。 / Evaluate actual events; Stay requires full interval containment, not arbitrary overlapping occupancy.
    pub fn evaluate(
        &self,
        previous: Option<&FlightTaskRecovery>,
        activity: Option<&FlightTaskRecovery>,
        evaluation: &FlowControlEvaluation<'_>,
    ) -> bool {
        let accepts = |activity: &FlightTaskRecovery| evaluation.condition.is_none_or(|condition| condition.matches(activity));
        let departure = |activity: &FlightTaskRecovery| activity.route().dep.icao.0 == evaluation.airport
            && evaluation.slot.contains_event(activity.time().0);
        let arrival = |activity: &FlightTaskRecovery| activity.route().arr.icao.0 == evaluation.airport
            && evaluation.slot.contains_event(activity.time().1);
        match self {
            Self::Departure => activity.is_some_and(|activity| accepts(activity) && departure(activity)),
            Self::Arrival => activity.is_some_and(|activity| accepts(activity) && arrival(activity)),
            Self::DepartureArrival => activity.is_some_and(|activity| accepts(activity) && (departure(activity) || arrival(activity))),
            Self::Stay => match (previous, activity) {
                (Some(previous), Some(activity)) => accepts(activity) && previous.route().arr.icao.0 == evaluation.airport
                    && evaluation.slot.contains_interval((previous.time().1, if activity.is_flight() { activity.time().0 } else { activity.time().1 })),
                (Some(previous), None) => accepts(previous) && arrival(previous),
                (None, Some(activity)) => accepts(activity) && departure(activity),
                (None, None) => false,
            },
        }
    }
}

/// 自定义只读事件条件，相同记录必须产生确定结果。 / Custom read-only event condition, deterministic for the same record.
pub trait FlowControlPredicate: std::fmt::Debug + Send + Sync {
    /// 当前完整实际恢复是否匹配。 / Whether the complete actual recovery matches.
    fn matches(&self, activity: &FlightTaskRecovery) -> bool;
}

/// 航班类别、飞机子型号及自定义条件的合取。 / Conjunction of flight category, aircraft minor type and custom conditions.
#[derive(Debug, Clone, Default)]
pub struct FlowControlCondition {
    /// 空集合表示不按国内/区域/国际类型过滤。 / Empty means no domestic/regional/international filter.
    pub flight_types: BTreeSet<FlightType>,
    /// 空集合表示不按实际飞机子型号过滤。 / Empty means no actual aircraft minor-type filter.
    pub aircraft_minor_types: BTreeSet<String>,
    /// 可选无模型状态的扩展条件。 / Optional model-state-free extension condition.
    pub predicate: Option<Arc<dyn FlowControlPredicate>>,
}

impl FlowControlCondition {
    /// 有条件过滤只针对飞行活动，类型从实际航线推导。 / Conditional filtering applies to flight activities, deriving type from the actual route.
    pub fn matches(&self, activity: &FlightTaskRecovery) -> bool {
        activity.is_flight()
            && (self.flight_types.is_empty() || self.flight_types.contains(&FlightType::from_airport_types(
                activity.route().dep.airport_type, activity.route().arr.airport_type)))
            && (self.aircraft_minor_types.is_empty() || self.aircraft_minor_types.contains(&activity.aircraft().minor_type.code.0))
            && self.predicate.as_ref().is_none_or(|predicate| predicate.matches(activity))
    }
}

/// 独立容量窗；相邻事件窗左闭右开，最后一窗包含末端。 / Independent capacity window; adjoining event windows are half-open except the inclusive final endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlowControlSlot {
    /// 实际开始时刻。 / Actual start instant.
    pub start: OffsetDateTime,
    /// 实际结束时刻。 / Actual end instant.
    pub end: OffsetDateTime,
    /// 是否包含结束事件，防止相邻窗重复计数。 / Whether to include end events, preventing duplicate adjoining-window counts.
    pub include_end: bool,
}

impl FlowControlSlot {
    /// 检查事件归属，非法反向窗口不匹配。 / Check event ownership, not matching reversed windows.
    pub fn contains_event(&self, time: OffsetDateTime) -> bool {
        self.start <= time && (time < self.end || (self.include_end && time == self.end))
    }

    /// 检查完整间隔包含；正长度间隔可终止于窗末端，零长度按事件归属。 / Check full containment; positive intervals may end at the window boundary, zero lengths follow event ownership.
    pub fn contains_interval(&self, time: (OffsetDateTime, OffsetDateTime)) -> bool {
        time.0 <= time.1 && self.contains_event(time.0) && time.1 <= self.end
    }
}

/// 单次实际事件或停场检查的统一口径。 / Unified semantics for one actual event or stay check.
#[derive(Debug, Clone, Copy)]
pub struct FlowControlEvaluation<'a> {
    /// 机场业务标识。 / Airport business identity.
    pub airport: &'a str,
    /// 实际容量窗。 / Actual capacity window.
    pub slot: &'a FlowControlSlot,
    /// 可选当前活动过滤条件。 / Optional current-activity filter.
    pub condition: Option<&'a FlowControlCondition>,
}

/// 每个容量窗的数量及固定外部使用量。 / Per-window capacity and fixed external usage.
#[derive(Debug, Clone)]
pub struct FlowControlCapacity {
    /// 每窗最大事件/完整停场数量。 / Maximum event/full-stay count per window.
    pub max_capacity: u64,
    /// 每窗固定外部已用数量，不重复注册为待恢复活动。 / Fixed external usage in each window, not re-registered as recovery activities.
    pub current_usage: u64,
    /// 正容量窗长度，最后一窗可截短。 / Positive capacity-window duration, with a possibly truncated final window.
    pub interval: Duration,
}

/// 流量控制 / Flow control (对齐 Kotlin FlowControl)
#[derive(Debug, Clone)]
pub struct FlowControl {
    /// 流控标识 / Flow control identifier
    pub id: String,
    /// 受控机场业务标识。 / Controlled airport business identity.
    pub airport: String,
    /// 有限流控时间范围。 / Finite flow-control time range.
    pub time_range: (OffsetDateTime, OffsetDateTime),
    /// 流控场景。 / Flow-control scene.
    pub scene: FlowControlScene,
    /// 可选飞行条件；None 允许地面活动的显式事件。 / Optional flight filter; None permits explicit ground-activity events.
    pub condition: Option<FlowControlCondition>,
    /// 流控容量 / Flow control capacity
    pub capacity: FlowControlCapacity,
    /// 每个超额事件的有限非负费用，None 表示硬上界。 / Finite nonnegative cost per excess event, with None denoting a hard upper bound.
    pub over_cost: Option<f64>,
}

impl FlowControl {
    /// 获取剩余可用容量 / Get remaining available capacity
    pub fn available(&self) -> u64 {
        self.capacity
            .max_capacity
            .saturating_sub(self.capacity.current_usage)
    }

    /// 在修改模型前生成有限容量窗，拒绝非正间隔及非法时间/费用。 / Prepare finite capacity windows before model mutation, rejecting nonpositive intervals and invalid times/costs.
    pub fn slots(&self) -> Result<Vec<FlowControlSlot>, RecoveryError> {
        if self.id.is_empty() || self.airport.is_empty() || self.time_range.0 >= self.time_range.1 || self.capacity.interval <= Duration::ZERO
            || self.capacity.max_capacity > (1_u64 << 53) || self.capacity.current_usage > (1_u64 << 53)
            || self.over_cost.is_some_and(|cost| !cost.is_finite() || cost < 0.0)
        { return Err(RecoveryError::InvalidRule("invalid flow-control identity, window, capacity or cost".into())); }
        let mut slots = Vec::new();
        let duration = (self.time_range.1 - self.time_range.0).whole_nanoseconds();
        let interval = self.capacity.interval.whole_nanoseconds();
        let count = usize::try_from((duration - 1) / interval + 1)
            .map_err(|_| RecoveryError::InvalidRule("flow-control slot count overflow".into()))?;
        slots.try_reserve_exact(count).map_err(|_| RecoveryError::InvalidRule("flow-control slot allocation failed".into()))?;
        let mut start = self.time_range.0;
        while start < self.time_range.1 {
            let duration = self.capacity.interval.min(self.time_range.1 - start);
            let end = start.checked_add(duration).ok_or_else(|| RecoveryError::InvalidRule("flow-control window overflow".into()))?;
            slots.push(FlowControlSlot { start, end, include_end: end == self.time_range.1 });
            start = end;
        }
        Ok(slots)
    }

    /// 检查束内每个活动及其实际前序，不凭整束端点推断内部事件；空束不虚构无限停场。 / Inspect every activity and its actual predecessor, never inferring internal events from bunch endpoints or inventing infinite idle stays.
    pub fn usage_in_slot(
        &self,
        bunch: &FlightTaskBunch,
        previous: Option<&FlightTaskRecovery>,
        slot: &FlowControlSlot,
    ) -> u64 {
        let evaluation = FlowControlEvaluation { airport: &self.airport, slot, condition: self.condition.as_ref() };
        let mut predecessor = previous;
        let mut usage = 0;
        for activity in bunch.activities() {
            usage += u64::from(self.scene.evaluate(predecessor, Some(activity), &evaluation));
            predecessor = Some(activity);
        }
        usage
    }
}
