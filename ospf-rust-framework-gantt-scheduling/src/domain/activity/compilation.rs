//! 在注册前冻结并校验完整候选，不截取、不修改模型。 / Freeze and validate complete choices before registration, without truncation or model mutation.

use std::collections::{BTreeMap, BTreeSet};
use time::{Duration, OffsetDateTime};
use ospf_rust_quantities::{Quantity, unit::{Unit, UnitConversionValue}};
use ospf_rust_core::solver::value::SolveValue;
use crate::domain::common::TaskId;
use crate::domain::task::SchedulingSolverValueAdapter;
use crate::infrastructure::{TimeRange, distant_future, distant_past};
use crate::GanttResult;
use super::model::*;
use super::numeric::{ActivityValue, activity_value, input, normalize_activity_value, validate_activity_unit, zero};

/// 已准备的完整候选，保留领域数值与 solver 边界系数。 / Prepared complete choice retaining native values and solver coefficients.
#[derive(Debug, Clone)]
pub struct PreparedActivityChoice<V> {
    /// 完整分配 / Complete placement
    pub placement: ActivityPlacement<V>,
    /// 模式、基线变更及额外费用的缓存总和 / Cached sum of mode, baseline-change and extra costs
    pub cost: ActivityValue<V>,
    /// 已换算为绝对时间的库存事件 / Inventory events converted to absolute times
    pub material_events: Vec<ActivityMaterialEvent<V>>,
}

/// 不可变准备快照；无变量注册态，必做空域仍合法。 / Immutable preparation without variable state; mandatory empty domains remain valid.
#[derive(Debug, Clone)]
pub struct ActivityPreparation<V> {
    problem: ActivitySchedulingProblem<V>,
    choices: BTreeMap<ActivityChoiceKey, PreparedActivityChoice<V>>,
    cancellation_costs: BTreeMap<TaskId, ActivityValue<V>>,
    solver_model_only: bool,
    rows: Vec<super::pipeline::ActivityPreparedRow<V>>,
}

impl<V> ActivityPreparation<V>
where
    V: SolveValue + UnitConversionValue,
{
    /// 接管输入、校验全部声明并缓存额外费用，每完整候选恰调用一次策略。
    /// Take ownership, validate all declarations and cache extra costs, calling the policy once per complete choice.
    pub fn new(
        mut problem: ActivitySchedulingProblem<V>,
        adapter: &dyn SchedulingSolverValueAdapter<V>,
        additional_cost: Option<&dyn ActivityCostPolicy<V>>,
    ) -> GanttResult<Self> {
        interval(problem.horizon)?;
        if problem.horizon.start == problem.horizon.end {
            return Err(input("activity horizon must be nonempty"));
        }
        validate_activity_unit(&problem.cost_unit)?;
        let mut solver_model_only = std::any::TypeId::of::<V>() != std::any::TypeId::of::<f64>()
            || !adapter.preserves_f64_identity();
        let mut resources = BTreeMap::new();
        for resource in &mut problem.resources {
            unique(&mut resources, resource.id.clone(), resource.capacity.unit.clone(), "resource")?;
            let unit = resource.capacity.unit.clone();
            normalize(&mut resource.capacity, &unit, adapter, true)?;
            for time in &resource.unavailable_times {
                interval(*time)?;
                if time.start == time.end {
                    return Err(input("resource calendar intervals must be nonempty"));
                }
            }
            resource.capacities.sort_by_key(|capacity| capacity.time);
            let mut previous_end = None;
            for capacity in &mut resource.capacities {
                interval(capacity.time)?;
                if capacity.time.start == capacity.time.end {
                    return Err(input("resource capacity intervals must be nonempty"));
                }
                if previous_end.is_some_and(|end| end > capacity.time.start) {
                    return Err(input(format!("overlapping capacity overrides: {}", resource.id)));
                }
                previous_end = Some(capacity.time.end);
                solver_model_only |= capacity.capacity.unit != unit;
                normalize(&mut capacity.capacity, &unit, adapter, true)?;
            }
        }
        let mut materials = BTreeMap::new();
        for material in &mut problem.materials {
            unique(&mut materials, material.id.clone(), material.initial.unit.clone(), "material")?;
            let unit = material.initial.unit.clone();
            normalize(&mut material.initial, &unit, adapter, false)?;
            if let Some(minimum) = &mut material.minimum {
                solver_model_only |= minimum.unit != unit;
                normalize(minimum, &unit, adapter, true)?;
            }
            if let Some(maximum) = &mut material.maximum {
                solver_model_only |= maximum.unit != unit;
                normalize(maximum, &unit, adapter, true)?;
                let minimum = material.minimum.as_ref().map(|quantity| quantity.value.clone())
                    .unwrap_or(zero::<V>()?);
                if maximum.value < minimum {
                    return Err(input(format!("inverted inventory bounds: {}", material.id)));
                }
            }
        }
        for event in &mut problem.material_events {
            point_in_horizon(event.time, problem.horizon)?;
            let unit = materials.get(&event.material)
                .ok_or_else(|| input(format!("unknown material: {}", event.material)))?;
            solver_model_only |= event.quantity.unit != *unit;
            normalize(&mut event.quantity, unit, adapter, false)?;
        }
        let mut tasks = BTreeMap::new();
        let mut cancellation_costs = BTreeMap::new();
        for activity in &mut problem.activities {
            unique(&mut tasks, activity.id.clone(), false, "task")?;
            if let Some(cost) = &mut activity.cancellation_cost {
                solver_model_only |= cost.unit != problem.cost_unit;
                let cost = normalize(cost, &problem.cost_unit, adapter, true)?;
                cancellation_costs.insert(activity.id.clone(), cost);
            }
            let mut modes = BTreeMap::new();
            for mode in &mut activity.modes {
                unique(&mut modes, mode.id.clone(), (), "mode")?;
                nonnegative_duration(mode.duration)?;
                solver_model_only |= mode.cost.unit != problem.cost_unit;
                normalize(&mut mode.cost, &problem.cost_unit, adapter, false)?;
                mode.starts.sort();
                if mode.starts.windows(2).any(|pair| pair[0] == pair[1]) {
                    return Err(input(format!("duplicate mode start: {} / {}", activity.id, mode.id)));
                }
                for usage in &mut mode.resources {
                    nonnegative_duration(usage.from)?;
                    let until = usage.until.unwrap_or(mode.duration);
                    nonnegative_duration(until)?;
                    if usage.from > until || until > mode.duration {
                        return Err(input("resource offsets must lie within mode duration"));
                    }
                    let unit = resources.get(&usage.resource)
                        .ok_or_else(|| input(format!("unknown resource: {}", usage.resource)))?;
                    solver_model_only |= usage.quantity.unit != *unit;
                    normalize(&mut usage.quantity, unit, adapter, true)?;
                }
                for flow in &mut mode.material_flows {
                    nonnegative_duration(flow.at)?;
                    if flow.at > mode.duration {
                        return Err(input("material offset exceeds mode duration"));
                    }
                    let unit = materials.get(&flow.material)
                        .ok_or_else(|| input(format!("unknown material: {}", flow.material)))?;
                    solver_model_only |= flow.quantity.unit != *unit;
                    normalize(&mut flow.quantity, unit, adapter, false)?;
                }
            }
        }
        for fact in &mut problem.executions {
            let placement = &mut fact.placement;
            unique(&mut tasks, placement.key.task.clone(), true, "task/fact")?;
            nonempty(&placement.key.mode, "fact mode")?;
            finite_time(placement.key.start)?;
            interval(placement.time)?;
            if placement.key.start != placement.time.start {
                return Err(input("fact key start differs from placement start"));
            }
            match fact.status {
                ActivityExecutionStatus::Completed if placement.time.end > problem.horizon.start => {
                    return Err(input("completed fact ends after horizon start"));
                }
                ActivityExecutionStatus::Running if placement.time.start > problem.horizon.start
                    || placement.time.end <= problem.horizon.start => {
                    return Err(input("running fact must straddle horizon start"));
                }
                _ => {}
            }
            for reservation in &mut placement.reservations {
                interval(reservation.time)?;
                if reservation.time.start < placement.time.start || reservation.time.end > placement.time.end {
                    return Err(input("fact reservation lies outside its placement"));
                }
                if fact.status == ActivityExecutionStatus::Running && reservation.time.start < problem.horizon.start {
                    return Err(input("running reservations must explicitly describe remaining usage"));
                }
                let unit = resources.get(&reservation.resource)
                    .ok_or_else(|| input(format!("unknown resource: {}", reservation.resource)))?;
                solver_model_only |= reservation.quantity.unit != *unit;
                normalize(&mut reservation.quantity, unit, adapter, true)?;
            }
        }
        let mut baselines = BTreeMap::new();
        for baseline in &mut problem.baselines {
            if tasks.get(&baseline.choice.task) != Some(&false) {
                return Err(input("baseline must reference a pending activity"));
            }
            nonempty(&baseline.choice.mode, "baseline mode")?;
            finite_time(baseline.choice.start)?;
            solver_model_only |= baseline.start_change_cost.unit != problem.cost_unit
                || baseline.mode_change_cost.unit != problem.cost_unit;
            normalize(&mut baseline.start_change_cost, &problem.cost_unit, adapter, true)?;
            normalize(&mut baseline.mode_change_cost, &problem.cost_unit, adapter, true)?;
            unique(&mut baselines, baseline.choice.task.clone(), baseline.clone(), "baseline")?;
        }
        for dependency in &problem.dependencies {
            if !tasks.contains_key(&dependency.predecessor) || !tasks.contains_key(&dependency.successor) {
                return Err(input("dependency references an unknown task/fact"));
            }
            finite_duration(dependency.minimum)?;
            if let Some(maximum) = dependency.maximum {
                finite_duration(maximum)?;
                if maximum < dependency.minimum {
                    return Err(input("inverted dependency lag bounds"));
                }
            }
        }
        let mut choices = BTreeMap::new();
        for activity in &problem.activities {
            for mode in &activity.modes {
                for start in &mode.starts {
                    point_in_horizon(*start, problem.horizon)?;
                    let end = add_time(*start, mode.duration)?;
                    point_in_horizon(end, problem.horizon)?;
                    let key = ActivityChoiceKey { task: activity.id.clone(), mode: mode.id.clone(), start: *start };
                    let reservations = mode.resources.iter().map(|usage| {
                        Ok(ActivityReservation {
                            resource: usage.resource.clone(),
                            time: TimeRange::new(add_time(*start, usage.from)?, add_time(*start, usage.until.unwrap_or(mode.duration))?),
                            quantity: usage.quantity.clone(),
                        })
                    }).collect::<GanttResult<Vec<_>>>()?;
                    let placement = ActivityPlacement { key: key.clone(), time: TimeRange::new(*start, end), reservations };
                    let mut cost = mode.cost.value.clone();
                    if let Some(baseline) = baselines.get(&activity.id) {
                        if key.start != baseline.choice.start {
                            cost = checked_add(cost, baseline.start_change_cost.value.clone(), adapter)?;
                        }
                        if key.mode != baseline.choice.mode {
                            cost = checked_add(cost, baseline.mode_change_cost.value.clone(), adapter)?;
                        }
                    }
                    if let Some(policy) = additional_cost {
                        let extra = policy.additional_cost(&placement)?;
                        solver_model_only |= extra.unit != problem.cost_unit;
                        let extra = normalize_activity_value(&extra, &problem.cost_unit, adapter)?;
                        cost = checked_add(cost, extra.value, adapter)?;
                    }
                    let material_events = mode.material_flows.iter().map(|flow| {
                        Ok(ActivityMaterialEvent {
                            material: flow.material.clone(),
                            time: add_time(*start, flow.at)?,
                            quantity: flow.quantity.clone(),
                        })
                    }).collect::<GanttResult<Vec<_>>>()?;
                    choices.insert(key, PreparedActivityChoice { placement, cost: activity_value(cost, adapter)?, material_events });
                }
            }
        }
        for baseline in &problem.baselines {
            if baseline.frozen && !choices.contains_key(&baseline.choice) {
                return Err(input("frozen baseline is not a declared candidate"));
            }
        }
        let mut rows = BTreeSet::new();
        for row in &mut problem.rows {
            nonempty(&row.id, "row")?;
            if !rows.insert(row.id.clone()) {
                return Err(input("duplicate activity row"));
            }
            validate_activity_unit(&row.unit)?;
            if row.lower.is_none() && row.upper.is_none() {
                return Err(input("activity row requires at least one bound"));
            }
            for bound in [&mut row.lower, &mut row.upper].into_iter().flatten() {
                solver_model_only |= bound.unit != row.unit;
                normalize(bound, &row.unit, adapter, false)?;
            }
            if let (Some(lower), Some(upper)) = (&row.lower, &row.upper) {
                if lower.value > upper.value {
                    return Err(input("inverted activity row bounds"));
                }
                activity_value(upper.value.clone() - lower.value.clone(), adapter)?;
            }
            for (key, quantity) in &mut row.coefficients {
                if !choices.contains_key(key) {
                    return Err(input("activity row references an unknown candidate"));
                }
                solver_model_only |= quantity.unit != row.unit;
                normalize(quantity, &row.unit, adapter, false)?;
            }
            for (task, quantity) in &mut row.cancellation_coefficients {
                if !cancellation_costs.contains_key(task) {
                    return Err(input("activity row references a task without explicit cancellation"));
                }
                solver_model_only |= quantity.unit != row.unit;
                normalize(quantity, &row.unit, adapter, false)?;
            }
        }
        let mut prepared = Self { problem, choices, cancellation_costs, solver_model_only, rows: Vec::new() };
        prepared.rows = super::pipeline::prepare_rows(&prepared, adapter)?;
        // 恒等声明不是数值等价证明；仍检查本次真正使用的每个系数。
        // An identity declaration is not numeric equivalence; check every coefficient actually used in this preparation.
        use ospf_rust_core::solver::value::SolveValueConversionPolicy;
        for value in prepared.rows.iter().flat_map(|row| row.terms.values().chain(row.lower.iter()).chain(row.upper.iter()))
            .chain(prepared.choices.values().map(|choice| &choice.cost)).chain(prepared.cancellation_costs.values())
        {
            let native = value.value.to_f64_with_policy(SolveValueConversionPolicy::AllowRounding)
                .map_err(|error| super::numeric::conversion(error.to_string()))?;
            prepared.solver_model_only |= native != value.solver;
        }
        Ok(prepared)
    }

    /// 归一化不可变输入；不暴露可修改的注册态。 / Normalized immutable input without mutable registration state.
    pub fn problem(&self) -> &ActivitySchedulingProblem<V> {
        &self.problem
    }

    /// 确定性顺序的全部完整候选。 / All complete choices in deterministic order.
    pub fn choices(&self) -> &BTreeMap<ActivityChoiceKey, PreparedActivityChoice<V>> {
        &self.choices
    }

    /// 仅含明确允许取消的活动及其费用。 / Only explicitly cancellable activities and their costs.
    pub fn cancellation_costs(&self) -> &BTreeMap<TaskId, ActivityValue<V>> {
        &self.cancellation_costs
    }

    /// 数值证据仅属于 SolverModel，不能直接提升为原域证明。
    /// Numeric evidence belongs only to SolverModel and cannot be promoted to a native-domain proof.
    pub fn solver_model_only(&self) -> bool {
        self.solver_model_only
    }

    /// 已完成全部可失败计算的受管行。 / Managed rows with all fallible calculations already completed.
    pub fn rows(&self) -> &[super::pipeline::ActivityPreparedRow<V>] { &self.rows }
}

fn nonempty(value: &str, kind: &str) -> GanttResult<()> {
    if value.trim().is_empty() {
        return Err(input(format!("empty {kind} identity")));
    }
    Ok(())
}

fn unique<K: Ord + ToString, T>(
    map: &mut BTreeMap<K, T>,
    key: K,
    value: T,
    kind: &str,
) -> GanttResult<()> {
    nonempty(&key.to_string(), kind)?;
    if map.insert(key, value).is_some() {
        return Err(input(format!("duplicate {kind} identity")));
    }
    Ok(())
}

fn normalize<V>(
    quantity: &mut Quantity<V, Unit>,
    unit: &Unit,
    adapter: &dyn SchedulingSolverValueAdapter<V>,
    nonnegative: bool,
) -> GanttResult<ActivityValue<V>>
where
    V: SolveValue + UnitConversionValue,
{
    let value = normalize_activity_value(quantity, unit, adapter)?;
    if nonnegative && value.value < zero::<V>()? {
        return Err(input("activity quantity must be non-negative"));
    }
    *quantity = Quantity::new(value.value.clone(), unit.clone());
    Ok(value)
}

pub(crate) fn checked_add<V>(
    left: V,
    right: V,
    adapter: &dyn SchedulingSolverValueAdapter<V>,
) -> GanttResult<V>
where
    V: SolveValue + UnitConversionValue,
{
    Ok(activity_value(left + right, adapter)?.value)
}

fn finite_time(time: OffsetDateTime) -> GanttResult<()> {
    if time <= distant_past() || time >= distant_future() {
        return Err(input("activity time cannot be an infinite sentinel"));
    }
    Ok(())
}

fn finite_duration(duration: Duration) -> GanttResult<()> {
    if duration == Duration::MIN || duration == Duration::MAX {
        return Err(input("activity duration cannot be an infinite sentinel"));
    }
    Ok(())
}

fn nonnegative_duration(duration: Duration) -> GanttResult<()> {
    finite_duration(duration)?;
    if duration < Duration::ZERO {
        return Err(input("activity duration must be non-negative"));
    }
    Ok(())
}

fn interval(time: TimeRange) -> GanttResult<()> {
    finite_time(time.start)?;
    finite_time(time.end)?;
    if time.start > time.end {
        return Err(input("inverted activity time interval"));
    }
    Ok(())
}

fn point_in_horizon(time: OffsetDateTime, horizon: TimeRange) -> GanttResult<()> {
    finite_time(time)?;
    if time < horizon.start || time > horizon.end {
        return Err(input("declared activity candidate/event lies outside the horizon"));
    }
    Ok(())
}

fn add_time(time: OffsetDateTime, duration: Duration) -> GanttResult<OffsetDateTime> {
    let result = time.checked_add(duration).ok_or_else(|| input("activity date arithmetic overflow"))?;
    finite_time(result)?;
    Ok(result)
}
