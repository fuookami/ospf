//! 注册前准备各约束族的双数值行，不接触 MetaModel。 / Prepare dual-numeric constraint families before registration without touching MetaModel.

use std::collections::{BTreeMap, BTreeSet};
use time::OffsetDateTime;
use ospf_rust_quantities::unit::UnitConversionValue;
use ospf_rust_core::solver::value::SolveValue;
use crate::domain::common::TaskId;
use crate::domain::task::SchedulingSolverValueAdapter;
use crate::infrastructure::TimeRange;
use crate::GanttResult;
use super::compilation::ActivityPreparation;
use super::model::*;
use super::numeric::{ActivityValue, activity_value, scalar, sum_activity_values, zero};

/// 活动域二元决策的稳定身份。 / Stable identity of an activity-domain binary decision.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActivityDecision {
    /// 完整执行候选 / Complete execution choice
    Execute(ActivityChoiceKey),
    /// 显式整单取消 / Explicit whole-order cancellation
    Cancel(TaskId),
}

/// 注册前准备的原域与 solver 行；不用于独立业务 validator。
/// Prepared native/solver row, never used as the independent business validator.
#[derive(Debug, Clone)]
pub struct ActivityPreparedRow<V> {
    /// 诊断身份，不作为未经清理的模型名称 / Diagnostic identity, not an unsanitized model name
    pub label: String,
    /// 完整有符号系数 / Complete signed coefficients
    pub terms: BTreeMap<ActivityDecision, ActivityValue<V>>,
    /// 已扣除固定事实与常数的下界 / Lower RHS after fixed facts and constants
    pub lower: Option<ActivityValue<V>>,
    /// 已扣除固定事实与常数的上界 / Upper RHS after fixed facts and constants
    pub upper: Option<ActivityValue<V>>,
}

struct Rows<'a, V: SolveValue> {
    adapter: &'a dyn SchedulingSolverValueAdapter<V>,
    rows: Vec<ActivityPreparedRow<V>>,
}

impl<V: SolveValue + UnitConversionValue> Rows<'_, V> {
    fn push(
        &mut self,
        label: String,
        terms: Vec<(ActivityDecision, V)>,
        constant: V,
        bounds: (Option<V>, Option<V>),
    ) -> GanttResult<()> {
        let mut grouped = BTreeMap::<ActivityDecision, Vec<V>>::new();
        for (decision, value) in terms { grouped.entry(decision).or_default().push(value); }
        let terms = grouped.into_iter().map(|(decision, values)| {
            Ok((decision, activity_value(sum_activity_values(values)?, self.adapter)?))
        }).collect::<GanttResult<_>>()?;
        let rhs = |bound: V| activity_value(bound - constant.clone(), self.adapter);
        self.rows.push(ActivityPreparedRow {
            label, terms, lower: bounds.0.map(&rhs).transpose()?, upper: bounds.1.map(rhs).transpose()?,
        });
        Ok(())
    }
}

pub(super) fn prepare_rows<V>(
    prepared: &ActivityPreparation<V>,
    adapter: &dyn SchedulingSolverValueAdapter<V>,
) -> GanttResult<Vec<ActivityPreparedRow<V>>>
where
    V: SolveValue + UnitConversionValue,
{
    let problem = prepared.problem();
    let one = scalar::<V>(1.0)?;
    let mut rows = Rows { adapter, rows: Vec::new() };
    for activity in &problem.activities {
        let mut terms = prepared.choices().keys().filter(|key| key.task == activity.id)
            .map(|key| (ActivityDecision::Execute(key.clone()), one.clone())).collect::<Vec<_>>();
        if activity.cancellation_cost.is_some() { terms.push((ActivityDecision::Cancel(activity.id.clone()), one.clone())); }
        rows.push(format!("selection/{}", activity.id), terms, zero()?, (Some(one.clone()), Some(one.clone())))?;
    }
    for baseline in &problem.baselines {
        if baseline.frozen {
            rows.push(format!("frozen/{}", baseline.choice.task), vec![(ActivityDecision::Execute(baseline.choice.clone()), one.clone())],
                zero()?, (Some(one.clone()), Some(one.clone())))?;
        }
    }
    for (ordinal, dependency) in problem.dependencies.iter().enumerate() {
        let predecessor = variants(prepared, &dependency.predecessor);
        let successor = variants(prepared, &dependency.successor);
        if dependency.presence != ActivityDependencyPresence::WhenBothPresent {
            let mut terms = Vec::new();
            let mut constants = Vec::new();
            for (decision, _) in &successor {
                if let Some(decision) = decision { terms.push((decision.clone(), one.clone())); } else { constants.push(one.clone()); }
            }
            for (decision, _) in &predecessor {
                let negative = scalar::<V>(-1.0)?;
                if let Some(decision) = decision { terms.push((decision.clone(), negative)); } else { constants.push(negative); }
            }
            let lower = (dependency.presence == ActivityDependencyPresence::RequireBoth).then(zero).transpose()?;
            rows.push(format!("presence/{ordinal}"), terms, sum_activity_values(constants)?, (lower, Some(zero()?)))?;
        }
        for (left, left_time) in &predecessor {
            for (right, right_time) in &successor {
                let lag = endpoint(*right_time, dependency.to) - endpoint(*left_time, dependency.from);
                if lag < dependency.minimum || dependency.maximum.is_some_and(|maximum| lag > maximum) {
                    let mut terms = Vec::new();
                    let mut constants = Vec::new();
                    for decision in [left, right] {
                        if let Some(decision) = decision { terms.push((decision.clone(), one.clone())); }
                        else { constants.push(one.clone()); }
                    }
                    rows.push(format!("timing/{ordinal}"), terms, sum_activity_values(constants)?, (None, Some(one.clone())))?;
                }
            }
        }
    }
    for resource in &problem.resources {
        let mut events = BTreeSet::from([problem.horizon.start, problem.horizon.end]);
        for time in resource.unavailable_times.iter().copied().chain(resource.capacities.iter().map(|capacity| capacity.time))
            .chain(prepared.choices().values().flat_map(|choice| &choice.placement.reservations)
                .chain(problem.executions.iter().flat_map(|fact| &fact.placement.reservations))
                .filter(|reservation| reservation.resource == resource.id).map(|reservation| reservation.time))
        {
            events.insert(time.start.clamp(problem.horizon.start, problem.horizon.end));
            events.insert(time.end.clamp(problem.horizon.start, problem.horizon.end));
        }
        let events = events.into_iter().collect::<Vec<_>>();
        for span in events.windows(2) {
            let at = span[0];
            let mut terms = Vec::new();
            for (key, choice) in prepared.choices() {
                let usage = sum_activity_values(choice.placement.reservations.iter()
                    .filter(|reservation| reservation.resource == resource.id && contains(reservation.time, at))
                    .map(|reservation| reservation.quantity.value.clone()))?;
                terms.push((ActivityDecision::Execute(key.clone()), usage));
            }
            let fixed = sum_activity_values(problem.executions.iter().flat_map(|fact| &fact.placement.reservations)
                .filter(|reservation| reservation.resource == resource.id && contains(reservation.time, at))
                .map(|reservation| reservation.quantity.value.clone()))?;
            let capacity = if resource.unavailable_times.iter().any(|time| contains(*time, at)) { zero()? }
                else { resource.capacities.iter().find(|capacity| contains(capacity.time, at))
                    .map(|capacity| capacity.capacity.value.clone()).unwrap_or_else(|| resource.capacity.value.clone()) };
            rows.push(format!("resource/{}/{at}", resource.id), terms, fixed, (None, Some(capacity)))?;
        }
    }
    for row in &problem.rows {
        let terms = row.coefficients.iter().map(|(key, quantity)| (ActivityDecision::Execute(key.clone()), quantity.value.clone()))
            .chain(row.cancellation_coefficients.iter().map(|(task, quantity)| (ActivityDecision::Cancel(task.clone()), quantity.value.clone()))).collect();
        rows.push(format!("extension/{}", row.id), terms, zero()?,
            (row.lower.as_ref().map(|quantity| quantity.value.clone()), row.upper.as_ref().map(|quantity| quantity.value.clone())))?;
    }
    for material in &problem.materials {
        let minimum = material.minimum.as_ref().map(|quantity| quantity.value.clone()).unwrap_or(zero()?);
        let maximum = material.maximum.as_ref().map(|quantity| quantity.value.clone());
        rows.push(format!("opening/{}", material.id), vec![], material.initial.value.clone(), (Some(minimum.clone()), maximum.clone()))?;
        let events = BTreeSet::from([problem.horizon.start, problem.horizon.end]).into_iter()
            .chain(problem.material_events.iter().filter(|event| event.material == material.id).map(|event| event.time))
            .chain(prepared.choices().values().flat_map(|choice| &choice.material_events)
                .filter(|event| event.material == material.id).map(|event| event.time)).collect::<BTreeSet<_>>();
        for at in events {
            let mut terms = Vec::new();
            for (key, choice) in prepared.choices() {
                let quantity = sum_activity_values(choice.material_events.iter()
                    .filter(|event| event.material == material.id && event.time <= at).map(|event| event.quantity.value.clone()))?;
                terms.push((ActivityDecision::Execute(key.clone()), quantity));
            }
            let constant = sum_activity_values(std::iter::once(material.initial.value.clone())
                .chain(problem.material_events.iter().filter(|event| event.material == material.id && event.time <= at)
                    .map(|event| event.quantity.value.clone())))?;
            rows.push(format!("inventory/{}/{at}", material.id), terms, constant, (Some(minimum.clone()), maximum.clone()))?;
        }
    }
    Ok(rows.rows)
}

fn variants<V: SolveValue + UnitConversionValue>(prepared: &ActivityPreparation<V>, task: &TaskId) -> Vec<(Option<ActivityDecision>, TimeRange)> {
    prepared.choices().iter().filter(|(key, _)| key.task == *task)
        .map(|(key, choice)| (Some(ActivityDecision::Execute(key.clone())), choice.placement.time))
        .chain(prepared.problem().executions.iter().filter(|fact| fact.placement.key.task == *task)
            .map(|fact| (None, fact.placement.time))).collect()
}

pub(super) fn endpoint(time: TimeRange, endpoint: ActivityEndpoint) -> OffsetDateTime {
    match endpoint { ActivityEndpoint::Start => time.start, ActivityEndpoint::Finish => time.end }
}

fn contains(time: TimeRange, at: OffsetDateTime) -> bool { time.start <= at && at < time.end }
