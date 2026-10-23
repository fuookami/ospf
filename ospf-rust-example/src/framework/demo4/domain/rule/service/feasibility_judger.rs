//! 可行性判断器 / Feasibility judger

use super::super::model::{Link, LinkType, Restriction, RestrictionCheckingResult};
use std::collections::HashMap;
use time::Duration;
use super::transition::{DefaultRecoveryTransitionPolicy, RecoveryTransitionPolicy};
use crate::framework::demo4::domain::passenger::model::Passenger;
use crate::framework::demo4::domain::task::model::{
    AircraftCapacity, AircraftUsability, FlightCycle, FlightHour, FlightTaskBunch,
    FlightTaskRecovery, FlightTaskStatus, RecoveryError,
};

/// 单束恢复规则；前序记录与本束必须消费同一时间及航线口径。 / Bunch recovery rules sharing the same time and route semantics as the predecessor.
pub struct BunchRecoveryRules<'a> {
    /// 飞机初始可用性 / Initial aircraft usability.
    pub usability: &'a AircraftUsability,
    /// 已执行前序活动，不重复计入本束飞行额度 / Executed predecessor, excluded from this bunch's flight allowance consumption.
    pub previous: Option<&'a FlightTaskRecovery>,
    /// 硬相邻或付费拆分链接 / Hard-adjacency or penalized-split links.
    pub links: &'a [Link],
}

/// 可行性判断器 / Feasibility judger
/// 对齐 Kotlin FeasibilityJudger
pub struct FeasibilityJudger;

impl FeasibilityJudger {
    /// 校验实际恢复记录的飞机可用性、前序、连接、状态及飞行额度。 / Validate actual recovery usability, predecessor, connections, statuses and flight allowances.
    pub fn judge_bunch(
        &self,
        bunch: &FlightTaskBunch,
        rules: &BunchRecoveryRules<'_>,
    ) -> Result<FeasibilityResult, RecoveryError> {
        self.judge_bunch_with_policy(bunch, rules, &DefaultRecoveryTransitionPolicy)
    }

    /// 生成、回放和提取共用实际记录的转移策略；不能覆盖时空连续、硬链接或飞行额度。 / Share actual-record transition policy across generation, replay and extraction without overriding continuity, hard links or allowances.
    pub fn judge_bunch_with_policy(
        &self,
        bunch: &FlightTaskBunch,
        rules: &BunchRecoveryRules<'_>,
        policy: &dyn RecoveryTransitionPolicy,
    ) -> Result<FeasibilityResult, RecoveryError> {
        let mut violations = Vec::new();
        if bunch.aircraft().minor_type.max_fly_time.is_some_and(|maximum| maximum < Duration::ZERO)
            || bunch.aircraft().minor_type.connection_time.values().any(|duration| *duration < Duration::ZERO)
        {
            return Err(RecoveryError::InvalidRule("negative aircraft duration limit".into()));
        }
        if let Some(previous) = rules.previous
            && !previous.aircraft().same_configuration(bunch.aircraft())
        {
            return Err(RecoveryError::AircraftMismatch(previous.original().plan.id.clone()));
        }
        if rules.previous.is_some_and(|previous| bunch.contains(&previous.original().plan.id)) {
            violations.push("executed predecessor is repeated in recovery activities".into());
        }
        for link in rules.links {
            if link.min_connection_time() < Duration::ZERO || link.split_cost.is_some_and(|cost| !cost.is_finite() || cost < 0.0) {
                return Err(RecoveryError::InvalidRule("invalid link duration or split cost".into()));
            }
            let from = bunch.recovery(&link.from_task).or_else(|| rules.previous
                .filter(|previous| previous.original().plan.id == link.from_task));
            let to = bunch.recovery(&link.to_task);
            if from.is_some() || to.is_some() {
                let follows_previous = rules.previous.is_some_and(|previous| previous.original().plan.id == link.from_task)
                    && bunch.activities().first().is_some_and(|activity| activity.original().plan.id == link.to_task);
                if !bunch.consecutive(&link.from_task, &link.to_task) && !follows_previous {
                    if link.split_cost.is_none() {
                        violations.push(format!("link {} -> {} is not consecutive", link.from_task, link.to_task));
                    }
                } else if let (Some(from), Some(to)) = (from, to)
                    && to.time().0 - from.time().1 < link.min_connection_time()
                {
                    violations.push(format!("link {} -> {} connection time", link.from_task, link.to_task));
                }
            }
        }
        let location = rules.previous.map(|previous| &previous.route().arr).unwrap_or(&rules.usability.location);
        if let Some(first) = bunch.activities().first()
            && (&first.route().dep != location || first.time().0 < rules.usability.enabled_time)
        {
            violations.push("aircraft initial usability".into());
        }
        let mut previous = rules.previous;
        for activity in bunch.activities() {
            let original = activity.original();
            if activity.is_flight() && original.aircraft().is_some_and(|planned| !matches!(
                (&planned.capacity, &activity.aircraft().capacity),
                (AircraftCapacity::Passenger { .. }, AircraftCapacity::Passenger { .. })
                    | (AircraftCapacity::Cargo { .. }, AircraftCapacity::Cargo { .. })
            )) {
                violations.push(format!("task {} aircraft capacity category", original.plan.id));
            }
            if activity.is_flight() && activity.aircraft().minor_type.max_fly_time
                .is_some_and(|maximum| activity.time().1 - activity.time().0 > maximum)
            {
                violations.push(format!("task {} maximum flight duration", original.plan.id));
            }
            let status = &original.plan.flight_task_status;
            let original_time = original.plan.time.or(original.plan.scheduled_time);
            if let Some(time) = original_time
                && ((status.contains(&FlightTaskStatus::NotAdvance) && activity.time().0 < time.0)
                    || (status.contains(&FlightTaskStatus::NotDelay) && activity.time().0 > time.0))
            {
                violations.push(format!("task {} time change", original.plan.id));
            }
            if let Some(planned) = original.plan.aircraft.as_ref()
                && ((status.contains(&FlightTaskStatus::NotAircraftChange) && planned != activity.aircraft())
                    || (status.contains(&FlightTaskStatus::NotAircraftTypeChange) && planned.type_code() != activity.aircraft().type_code())
                    || (status.contains(&FlightTaskStatus::NotAircraftMinorTypeChange) && planned.minor_type != activity.aircraft().minor_type))
            {
                violations.push(format!("task {} aircraft change", original.plan.id));
            }
            if status.contains(&FlightTaskStatus::NotTerminalChange)
                && (original.plan.dep != activity.route().dep || original.plan.arr != activity.route().arr)
            {
                violations.push(format!("task {} route change", original.plan.id));
            }
            if !policy.allows(bunch.aircraft(), previous, activity)? {
                violations.push(format!("task {} transition policy", original.plan.id));
            }
            if let Some(prev) = previous {
                let mut connection = policy.connection_time(bunch.aircraft(), prev, activity)?;
                if connection < Duration::ZERO {
                    return Err(RecoveryError::InvalidRule("negative transition connection time".into()));
                }
                if rules.links.iter().any(|link| {
                    link.from_task == prev.original().plan.id && link.to_task == original.plan.id
                        && matches!(link.link_type, LinkType::ConnectionTimeIgnoring)
                }) {
                    connection = Duration::ZERO;
                }
                if prev.route().arr != activity.route().dep || activity.time().0 - prev.time().1 < connection {
                    violations.push(format!("task {} predecessor connection", original.plan.id));
                }
            }
            previous = Some(activity);
        }
        for period in &rules.usability.flight_cycle_periods {
            if period.remaining_flight_hour.as_ref().is_some_and(|value| value.hours < Duration::ZERO) {
                return Err(RecoveryError::InvalidRule("negative flight allowance".into()));
            }
            // 起飞在到期前的飞行完整计入，不把跨到期航段截断。 / Count whole flights departing before expiry, without truncating crossing flights.
            let flights: Vec<_> = bunch.activities().iter()
                .filter(|activity| activity.is_flight() && activity.time().0 < period.expiration_time).collect();
            let hour = FlightHour::new(flights.iter().map(|activity| activity.time().1 - activity.time().0)
                .fold(Duration::ZERO, |sum, duration| sum + duration));
            let cycle = FlightCycle::new(flights.len() as u64);
            if !period.enabled_flight_hour(&hour) || !period.enabled_flight_cycle(&cycle) {
                violations.push("aircraft flight allowance".into());
            }
        }
        Ok(FeasibilityResult { feasible: violations.is_empty(), violations })
    }

    /// 独立复核硬旅客行程，允许跨飞机中转；容量按任务及舱位聚合。 / Independently validate hard passenger itineraries, allowing cross-aircraft transfers and aggregating seats by task and class.
    pub fn judge_passengers(
        &self,
        bunches: &[FlightTaskBunch],
        passengers: &[Passenger],
    ) -> Result<FeasibilityResult, RecoveryError> {
        let mut activities = HashMap::new();
        for activity in bunches.iter().flat_map(|bunch| bunch.activities()) {
            let id = activity.original().plan.id.as_str();
            if activities.insert(id, activity).is_some() {
                return Err(RecoveryError::DuplicateTask(id.into()));
            }
        }
        let mut violations = Vec::new();
        let mut seats = HashMap::<(&str, &str), u64>::new();
        let mut ids = std::collections::HashSet::new();
        for passenger in passengers {
            if !ids.insert(&passenger.id) || passenger.flights.is_empty() {
                return Err(RecoveryError::InvalidRule(format!("invalid passenger {}", passenger.id)));
            }
            let mut previous: Option<&FlightTaskRecovery> = None;
            let mut visited = std::collections::HashSet::new();
            for (id, class) in &passenger.flights {
                if !visited.insert(id) {
                    return Err(RecoveryError::InvalidRule(format!("repeated passenger flight {id}")));
                }
                let Some(activity) = activities.get(id.as_str()).copied() else {
                    violations.push(format!("passenger {} missing flight {id}", passenger.id));
                    previous = None;
                    continue;
                };
                if let Some(prev) = previous {
                    let transfer = prev.route().arr.passenger_transfer_time;
                    if transfer < Duration::ZERO {
                        return Err(RecoveryError::InvalidRule("negative passenger transfer time".into()));
                    }
                    if prev.route().arr != activity.route().dep || activity.time().0 - prev.time().1 < transfer {
                        violations.push(format!("passenger {} transfer", passenger.id));
                    }
                }
                let used = seats.entry((id.as_str(), class.0.as_str())).or_default();
                *used = used.checked_add(passenger.amount)
                    .ok_or_else(|| RecoveryError::InvalidRule("passenger amount overflow".into()))?;
                previous = Some(activity);
            }
        }
        for ((id, class), amount) in seats {
            let activity = activities[id];
            let capacity = match &activity.aircraft().capacity {
                AircraftCapacity::Passenger { capacities } => capacities.iter()
                    .filter(|(key, _)| key.0 == class).try_fold(0_u64, |sum, (_, value)| sum.checked_add(*value))
                    .ok_or_else(|| RecoveryError::InvalidRule("passenger capacity overflow".into()))?,
                AircraftCapacity::Cargo { .. } => 0,
            };
            if amount > capacity {
                violations.push(format!("flight {id} class {class} passenger capacity"));
            }
        }
        Ok(FeasibilityResult { feasible: violations.is_empty(), violations })
    }

    /// 使用完整实际记录检查硬限制，不把允许付费的违反当作不可行。 / Check hard restrictions against full actual records without treating permitted penalized violations as infeasibility.
    pub fn judge_restrictions(&self, bunch: &FlightTaskBunch, restrictions: &[Restriction]) -> FeasibilityResult {
        let violations = bunch.activities().iter().flat_map(|activity| restrictions.iter().filter_map(move |restriction| {
            if let RestrictionCheckingResult::Violate { reason } = restriction.check(activity) {
                Some(format!("{}: {reason}", activity.original().plan.id))
            } else { None }
        })).collect::<Vec<_>>();
        FeasibilityResult { feasible: violations.is_empty(), violations }
    }
}

/// 可行性结果 / Feasibility result
#[derive(Debug, Clone)]
pub struct FeasibilityResult {
    /// 是否可行 / Whether feasible
    pub feasible: bool,
    /// 违规原因列表 / Violation reason list
    pub violations: Vec<String>,
}
