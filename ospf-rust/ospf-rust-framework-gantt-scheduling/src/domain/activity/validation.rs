//! 直接复核业务选择、端点和占用，不依赖生成的矩阵行。 / Direct business-choice, endpoint and reservation validation independent of generated matrix rows.

use std::collections::{BTreeMap, BTreeSet};
use ospf_rust_quantities::{Quantity, unit::UnitConversionValue};
use ospf_rust_core::solver::value::SolveValue;
use crate::domain::common::TaskId;
use crate::{GanttError, GanttResult};
use super::compilation::ActivityPreparation;
use super::inventory::replay_activity_inventory;
use super::model::*;
use super::numeric::{sum_activity_values, zero};

/// 独立复核显式选择及取消，不使用 solver 容差豁免原域违反。
/// Independently validate explicit choices and cancellations, without solver-tolerance exemptions in the native domain.
pub fn validate_activity_selection<V>(
    prepared: &ActivityPreparation<V>,
    keys: &[ActivityChoiceKey],
    cancelled: &BTreeSet<TaskId>,
) -> GanttResult<ActivityScheduleValidation>
where
    V: SolveValue + UnitConversionValue,
{
    let problem = prepared.problem();
    let mut violations = Vec::new();
    let mut violate = |kind, subject: String, time, message: &str| {
        violations.push(ActivityViolation { kind, subject, time, message: message.into() });
    };
    let mut selected = BTreeSet::new();
    for key in keys {
        if !prepared.choices().contains_key(key) {
            violate(ActivityViolationKind::UnknownChoice, key.task.to_string(), Some(key.start), "unknown complete choice");
        } else if !selected.insert(key.clone()) {
            violate(ActivityViolationKind::DuplicateChoice, key.task.to_string(), Some(key.start), "duplicate complete choice");
        }
    }
    for task in cancelled {
        if !prepared.cancellation_costs().contains_key(task) {
            violate(ActivityViolationKind::Selection, task.to_string(), None, "task has no explicit cancellation option");
        }
    }
    for activity in &problem.activities {
        let count = selected.iter().filter(|key| key.task == activity.id).count() + usize::from(cancelled.contains(&activity.id));
        if count != 1 {
            violate(ActivityViolationKind::Selection, activity.id.to_string(), None, "exactly one execution or explicit cancellation is required");
        }
    }
    for baseline in &problem.baselines {
        if baseline.frozen && !selected.contains(&baseline.choice) {
            violate(ActivityViolationKind::Frozen, baseline.choice.task.to_string(), None, "frozen baseline changed or cancelled");
        }
    }
    let mut times = problem.executions.iter().map(|fact| (fact.placement.key.task.clone(), fact.placement.time)).collect::<BTreeMap<_, _>>();
    for key in &selected { times.insert(key.task.clone(), prepared.choices()[key].placement.time); }
    for dependency in &problem.dependencies {
        let left = times.get(&dependency.predecessor);
        let right = times.get(&dependency.successor);
        let presence_valid = match dependency.presence {
            ActivityDependencyPresence::WhenBothPresent => true,
            ActivityDependencyPresence::RequirePredecessor => right.is_none() || left.is_some(),
            ActivityDependencyPresence::RequireBoth => left.is_some() == right.is_some(),
        };
        if !presence_valid {
            violate(ActivityViolationKind::Dependency, format!("{}->{}", dependency.predecessor, dependency.successor), None, "dependency presence rule violated");
        }
        if let (Some(left), Some(right)) = (left, right) {
            let from = match dependency.from { ActivityEndpoint::Start => left.start, ActivityEndpoint::Finish => left.end };
            let to = match dependency.to { ActivityEndpoint::Start => right.start, ActivityEndpoint::Finish => right.end };
            let lag = to - from;
            if lag < dependency.minimum || dependency.maximum.is_some_and(|maximum| lag > maximum) {
                violate(ActivityViolationKind::Dependency, format!("{}->{}", dependency.predecessor, dependency.successor), Some(to), "dependency endpoint lag violated");
            }
        }
    }
    let reservations = selected.iter().flat_map(|key| &prepared.choices()[key].placement.reservations)
        .chain(problem.executions.iter().flat_map(|fact| &fact.placement.reservations)).collect::<Vec<_>>();
    for resource in &problem.resources {
        let mut events = BTreeSet::from([problem.horizon.start, problem.horizon.end]);
        for reservation in &reservations {
            if reservation.resource == resource.id {
                events.insert(reservation.time.start.clamp(problem.horizon.start, problem.horizon.end));
                events.insert(reservation.time.end.clamp(problem.horizon.start, problem.horizon.end));
            }
        }
        for time in resource.unavailable_times.iter().copied().chain(resource.capacities.iter().map(|capacity| capacity.time)) {
            events.insert(time.start.clamp(problem.horizon.start, problem.horizon.end));
            events.insert(time.end.clamp(problem.horizon.start, problem.horizon.end));
        }
        for at in events.into_iter().filter(|at| *at < problem.horizon.end) {
            let usage = sum_activity_values(reservations.iter().filter(|reservation| reservation.resource == resource.id
                && reservation.time.start <= at && at < reservation.time.end).map(|reservation| reservation.quantity.value.clone()))?;
            let mut capacity = resource.capacity.value.clone();
            for override_capacity in &resource.capacities {
                if override_capacity.time.start <= at && at < override_capacity.time.end { capacity = override_capacity.capacity.value.clone(); }
            }
            if resource.unavailable_times.iter().any(|time| time.start <= at && at < time.end) { capacity = zero()?; }
            if usage > capacity {
                violate(ActivityViolationKind::Resource, resource.id.to_string(), Some(at), "native-domain resource capacity exceeded");
            }
        }
    }
    for row in &problem.rows {
        let value = sum_activity_values(selected.iter().filter_map(|key| row.coefficients.get(key)).map(|quantity| quantity.value.clone())
            .chain(cancelled.iter().filter_map(|task| row.cancellation_coefficients.get(task)).map(|quantity| quantity.value.clone())))?;
        if row.lower.as_ref().is_some_and(|lower| value < lower.value) || row.upper.as_ref().is_some_and(|upper| value > upper.value) {
            violate(ActivityViolationKind::Row, row.id.clone(), None, "native-domain business row violated");
        }
    }
    let (_, inventory_violations) = replay_activity_inventory(prepared, &selected)?;
    violations.extend(inventory_violations);
    Ok(ActivityScheduleValidation { violations })
}

impl<V: SolveValue + UnitConversionValue> ActivityPreparation<V> {
    /// 未选择的可取消活动视为显式取消；必做缺失仍报告违反。
    /// Unselected cancellable activities are treated as cancellations; missing mandatory choices remain violations.
    pub fn validate(&self, keys: &[ActivityChoiceKey]) -> GanttResult<ActivityScheduleValidation> {
        let tasks = keys.iter().map(|key| &key.task).collect::<BTreeSet<_>>();
        let cancelled = self.cancellation_costs().keys().filter(|task| !tasks.contains(task)).cloned().collect();
        validate_activity_selection(self, keys, &cancelled)
    }

    /// 只从已通过独立原域复核的显式选择组装结果；不生成任何求解证明。
    /// Assemble results only from independently validated native choices, without generating solve proofs.
    pub fn schedule(
        &self,
        keys: &[ActivityChoiceKey],
        cancelled: &BTreeSet<TaskId>,
    ) -> GanttResult<ActivitySchedule<V>> {
        let validation = validate_activity_selection(self, keys, cancelled)?;
        if !validation.feasible() {
            return Err(GanttError::Calculation { message: format!("activity incumbent fails independent validation: {:?}", validation.violations) });
        }
        let selected = keys.iter().cloned().collect::<BTreeSet<_>>();
        let (material_inventory, _) = replay_activity_inventory(self, &selected)?;
        let cost = sum_activity_values(selected.iter().map(|key| self.choices()[key].cost.value.clone())
            .chain(cancelled.iter().map(|task| self.cancellation_costs()[task].value.clone())))?;
        Ok(ActivitySchedule {
            placements: selected.iter().map(|key| self.choices()[key].placement.clone()).collect(),
            executions: self.problem().executions.clone(), cancelled: cancelled.clone(),
            cost: Quantity::new(cost, self.problem().cost_unit.clone()), candidate_scope: self.choices().keys().cloned().collect(), material_inventory,
        })
    }
}
