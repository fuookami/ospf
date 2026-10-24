//! 基于实际分配的限制条件与严重性。 / Assignment-based restriction conditions and severity.

mod condition;

use std::collections::{BTreeSet, HashSet};
use crate::framework::demo4::domain::task::model::{Aircraft, FlightTaskImpl, FlightTaskRecovery, RecoveryError};
pub use condition::{GeneralRestrictionCondition, GeneralRestrictionPredicate, RestrictionAssignment};

/// 限制严重性，与条件种类分开。 / Restriction severity, independent of condition kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RestrictionType {
    /// 弱限制，允许报告违反及配置费用。 / Weak restriction with reportable violations and optional costs.
    Weak,
    /// 可违反的强限制。 / Violable strong restriction.
    ViolableStrong,
    /// 未配置费用时为硬限制，配置费用时允许付费违反。 / Hard when unpriced, allowing penalized violations when priced.
    Strong,
}

/// 限制检查结果 / Restriction checking result (对齐 Kotlin RestrictionCheckingResult)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestrictionCheckingResult {
    /// 无关 / Not applicable
    NotMatter,
    /// 违规，包含原因 / Violation with reason
    Violate {
        /// 稳定限制标识。 / Stable restriction identity.
        reason: String,
    },
    /// 未违规 / No violation
    NotViolate,
    /// 可豁免违规，包含原因 / Waivable violation with reason
    ViolableViolate {
        /// 稳定限制标识。 / Stable restriction identity.
        reason: String,
    },
}

impl RestrictionCheckingResult {
    /// 是否违反，不包含不适用或未违反。 / Whether violated, excluding inapplicable or satisfied restrictions.
    pub fn violated(&self) -> bool { matches!(self, Self::Violate { .. } | Self::ViolableViolate { .. }) }
}

/// 飞机与有向航线的黑白名单。 / Aircraft blacklist or whitelist for a directed route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationRestrictionCategory {
    /// 集合内飞机禁止执行。 / Aircraft in the set are forbidden.
    BlackList,
    /// 仅集合内飞机可执行。 / Only aircraft in the set are permitted.
    WhiteList,
}

/// 业务条件，不混用严重性与规则种类。 / Business condition without conflating severity and rule kind.
#[derive(Debug, Clone)]
pub enum RestrictionRule {
    /// 有向航线与实际飞机的关系限制，仅对飞行活动适用。 / Directed-route/actual-aircraft relation restriction, applicable only to flight activities.
    Relation {
        /// 黑白名单类别。 / Blacklist or whitelist category.
        category: RelationRestrictionCategory,
        /// 出发机场标识。 / Departure airport identity.
        departure: String,
        /// 到达机场标识。 / Arrival airport identity.
        arrival: String,
        /// 飞机注册号集合。 / Aircraft registration set.
        aircraft: BTreeSet<String>,
    },
    /// 命中条件与飞机适用域按 AND 组合；全部缺省时不适用。 / Hit condition and aircraft scopes combined with AND; inapplicable when all are absent.
    General {
        /// 命中条件。 / Hit condition.
        condition: Option<GeneralRestrictionCondition>,
        /// 仅在集合内的实际飞机适用，None 不过滤。 / Apply only to actual aircraft in this set; None does not filter.
        enabled_aircraft: Option<BTreeSet<String>>,
        /// 集合内的实际飞机不适用，None 不过滤。 / Exempt actual aircraft in this set; None does not filter.
        disabled_aircraft: Option<BTreeSet<String>>,
    },
}

/// 带稳定身份、权重和可选单次费用的完整限制。 / Complete restriction with stable identity, weight and optional per-hit cost.
#[derive(Debug, Clone)]
pub struct Restriction {
    /// 限制标识 / Restriction identifier
    pub id: String,
    /// 严重性。 / Severity.
    pub restriction_type: RestrictionType,
    /// 每次违反的非负有限权重。 / Finite nonnegative weight per violation.
    pub weight: f64,
    /// 可选非负有限费用；弱/可违反限制缺省费用时只报告权重。 / Optional finite nonnegative cost; unpriced weak/violable restrictions report weights without invented costs.
    pub cost: Option<f64>,
    /// 关系或通用条件。 / Relation or general condition.
    pub rule: RestrictionRule,
}

pub(crate) struct RestrictionReferences<'a> {
    pub tasks: HashSet<&'a str>,
    pub aircraft: HashSet<&'a str>,
    pub airports: HashSet<&'a str>,
}

impl Restriction {
    /// 原任务检查，不虚构缺失分配。 / Check an original task without inventing missing assignments.
    pub fn check_task(&self, task: &FlightTaskImpl) -> RestrictionCheckingResult {
        self.check_assignment(&RestrictionAssignment::from_task(task))
    }

    /// 指定飞机检查，不改写原任务。 / Check an explicitly assigned aircraft without rewriting the original task.
    pub fn check_aircraft(&self, task: &FlightTaskImpl, aircraft: &Aircraft) -> RestrictionCheckingResult {
        self.check_assignment(&RestrictionAssignment { aircraft: Some(aircraft), ..RestrictionAssignment::from_task(task) })
    }

    /// 检查实际飞机、航线和时间，禁止回退到计划值。 / Check actual aircraft, route and times without falling back to planned values.
    pub fn check(&self, recovery: &FlightTaskRecovery) -> RestrictionCheckingResult {
        self.check_assignment(&RestrictionAssignment::from_recovery(recovery))
    }

    /// 统一检查入口，自定义分配也消费相同口径。 / Unified check entry with the same semantics for custom assignment views.
    pub fn check_assignment(&self, assignment: &RestrictionAssignment<'_>) -> RestrictionCheckingResult {
        let hit = match &self.rule {
            RestrictionRule::Relation { category, departure, arrival, aircraft } => {
                if !assignment.task.task_type.category.is_flight_type() || &assignment.departure.icao.0 != departure || &assignment.arrival.icao.0 != arrival {
                    return RestrictionCheckingResult::NotMatter;
                }
                let Some(executor) = assignment.aircraft else { return RestrictionCheckingResult::NotMatter; };
                let contains = aircraft.contains(&executor.reg_no.0);
                let violated = match category { RelationRestrictionCategory::BlackList => contains, RelationRestrictionCategory::WhiteList => !contains };
                if !violated { return RestrictionCheckingResult::NotViolate; }
                true
            }
            RestrictionRule::General { condition, enabled_aircraft, disabled_aircraft } => {
                (condition.is_some() || enabled_aircraft.is_some() || disabled_aircraft.is_some())
                    && condition.as_ref().is_none_or(|condition| condition.matches(assignment))
                    && enabled_aircraft.as_ref().is_none_or(|aircraft| assignment.aircraft.is_some_and(|executor| aircraft.contains(&executor.reg_no.0)))
                    && disabled_aircraft.as_ref().is_none_or(|aircraft| assignment.aircraft.is_none_or(|executor| !aircraft.contains(&executor.reg_no.0)))
            }
        };
        if !hit { RestrictionCheckingResult::NotMatter }
        else if self.is_hard() { RestrictionCheckingResult::Violate { reason: self.id.clone() } }
        else { RestrictionCheckingResult::ViolableViolate { reason: self.id.clone() } }
    }

    /// 仅 Strong 且没有费用时禁止违反，与快照严重性合同一致。 / Only unpriced Strong restrictions forbid violations, matching snapshot severity semantics.
    pub fn is_hard(&self) -> bool { self.restriction_type == RestrictionType::Strong && self.cost.is_none() }

    /// 飞机是否被显式引用，不以此替代实际条件判断。 / Whether the aircraft is explicitly referenced, not a substitute for checking actual conditions.
    pub fn related(&self, aircraft: &Aircraft) -> bool {
        match &self.rule {
            RestrictionRule::Relation { aircraft: executors, .. } => executors.contains(&aircraft.reg_no.0),
            RestrictionRule::General { enabled_aircraft, disabled_aircraft, .. } => enabled_aircraft.as_ref().is_some_and(|executors| executors.contains(&aircraft.reg_no.0))
                || disabled_aircraft.as_ref().is_some_and(|executors| executors.contains(&aircraft.reg_no.0)),
        }
    }

    pub(crate) fn validate(&self, references: &RestrictionReferences<'_>) -> Result<(), RecoveryError> {
        if self.id.is_empty() || !self.weight.is_finite() || self.weight < 0.0
            || self.cost.is_some_and(|cost| !cost.is_finite() || cost < 0.0 || !(cost * self.weight).is_finite())
        { return Err(RecoveryError::InvalidRule("invalid restriction identity, weight or cost".into())); }
        match &self.rule {
            RestrictionRule::Relation { departure, arrival, aircraft, .. } => {
                if !references.airports.contains(departure.as_str()) || !references.airports.contains(arrival.as_str())
                    || aircraft.iter().any(|executor| !references.aircraft.contains(executor.as_str()))
                { return Err(RecoveryError::InvalidRule("unknown relation restriction reference".into())); }
            }
            RestrictionRule::General { condition, enabled_aircraft, disabled_aircraft } => {
                if let Some(condition) = condition { condition.validate(references)?; }
                if enabled_aircraft.iter().chain(disabled_aircraft).flatten().any(|executor| !references.aircraft.contains(executor.as_str())) {
                    return Err(RecoveryError::InvalidRule("unknown general restriction aircraft".into()));
                }
            }
        }
        Ok(())
    }
}
