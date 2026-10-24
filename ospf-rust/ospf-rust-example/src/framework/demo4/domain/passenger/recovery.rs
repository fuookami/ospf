//! 有界旅客取消及舱位/航班改签；不可变行定义供主问题和定价共用。 / Bounded passenger cancellation and cabin/flight changes with immutable rows shared by master and pricing.

mod rebooking;

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::mechanism::constraint::LinearInequality;
use ospf_rust_core::model::mechanism::constraint_group::ConstraintGroup;
use ospf_rust_core::model::object::SubObjective;
use ospf_rust_core::variable::{UInteger, VariableRange};
use ospf_rust_framework_gantt_scheduling::{GanttError, GanttResult};
use ospf_rust_framework_gantt_scheduling::domain::task_compilation::adapter::next_gantt_symbol_id;
use crate::framework::demo4::domain::bunch_compilation::context::same_recovery;
use crate::framework::demo4::domain::bunch_compilation::rules::passenger_rows;
use crate::framework::demo4::domain::bunch_generation::recovery::RecoveryBunchCandidate;
use crate::framework::demo4::domain::task::model::{AircraftCapacity, FlightTaskImpl, FlightTaskRecovery, Route};
use self::rebooking::Connection;
use super::model::Passenger;

/// 每个旅客组航段的显式取消、舱位和航班改签政策。 / Explicit cancellation, cabin and flight-change policy for each passenger-group leg.
#[derive(Debug, Clone)]
pub struct RecoveryPassengerLegPolicy {
    /// 旅客组身份。 / Passenger-group identity.
    pub passenger: String,
    /// 原始航班身份。 / Original flight identity.
    pub flight: String,
    /// 每人取消费用；None 禁止取消。 / Per-person cancellation cost; None forbids cancellation.
    pub cancellation_cost: Option<f64>,
    /// 允许的目标舱位及每人改签费用；不得包含原舱位。 / Allowed destination cabins and per-person change costs, excluding the original cabin.
    pub class_change_costs: BTreeMap<String, f64>,
    /// 允许的同 OD 目标航班/舱位及每人总改签费用；空映射关闭航班改签。 / Allowed same-OD destination flight/cabin and total per-person rebooking cost; empty disables flight changes.
    pub flight_change_costs: BTreeMap<(String, String), f64>,
}

/// 完整旅客配置；每个航段必须恰好有一项政策。 / Complete passenger configuration requiring exactly one policy per leg.
#[derive(Debug, Clone)]
pub struct RecoveryPassengerConfiguration {
    /// 原始有序行程。 / Original ordered itineraries.
    pub passengers: Vec<Passenger>,
    /// 显式航段政策。 / Explicit leg policies.
    pub policies: Vec<RecoveryPassengerLegPolicy>,
}

/// 从实际整数决策及恢复活动独立复核的航段结果。 / Leg result independently checked against actual integer decisions and recovered activities.
#[derive(Debug, Clone, PartialEq)]
pub struct RecoveryPassengerLegResult {
    /// 原旅客组。 / Original passenger group.
    pub passenger: String,
    /// 原航班，实际改签去向另列。 / Original flight, with actual rebooking destinations reported separately.
    pub flight: String,
    /// 实际取消人数。 / Actual canceled count.
    pub canceled: u64,
    /// 各允许目标舱位的实际改签人数，包括零。 / Actual changed count for every allowed destination cabin, including zeros.
    pub class_changes: BTreeMap<String, u64>,
    /// 原航班各舱位实际运输人数，不包含改签去其他航班的人数。 / Actual counts by cabin on the original flight, excluding passengers rebooked to other flights.
    pub transported: BTreeMap<String, u64>,
    /// 各允许目标航班/舱位的实际改签人数，含零。 / Actual counts for every allowed destination flight/cabin, including zeros.
    pub flight_changes: BTreeMap<(String, String), u64>,
    /// 前航段到本航段的实际连接人数；只有参与航班改签的相邻段使用连接流。 / Actual incoming connection counts by previous/current flight; connection flow is used only for adjacent legs involving flight changes.
    pub connections: BTreeMap<(String, String), u64>,
    /// 实际取消费用。 / Actual cancellation penalty.
    pub cancellation_penalty: f64,
    /// 实际舱位改签费用。 / Actual cabin-change penalty.
    pub class_change_penalty: f64,
    /// 实际航班改签总费用，不重复计入舱位改签费用。 / Actual total flight-rebooking penalty, not counted again as a cabin-change penalty.
    pub flight_change_penalty: f64,
}

#[derive(Debug, Clone)]
struct Decision { bound: f64, cost: f64 }

#[derive(Debug, Clone)]
struct Leg {
    passenger: String,
    flight: String,
    route: Route,
    class: String,
    amount: u64,
    cancel: usize,
    changes: BTreeMap<String, usize>,
    flights: BTreeMap<(String, String), usize>,
    previous: Option<usize>,
}

#[derive(Debug, Clone)]
enum Usage {
    None,
    Capacity { flight: String, class: String },
    Transfer { pair: Box<(FlightTaskRecovery, FlightTaskRecovery)>, amount: f64 },
    Enabled { activity: Box<FlightTaskRecovery>, amount: f64 },
    Allowed { activities: Vec<FlightTaskRecovery>, amount: f64 },
}

#[derive(Debug, Clone)]
struct Row { name: String, usage: Usage, terms: Vec<(usize, f64)>, bound: f64 }

impl Row {
    fn coefficient(&self, candidate: &RecoveryBunchCandidate) -> f64 {
        match &self.usage {
            Usage::None => 0.0,
            Usage::Capacity { flight, class } => candidate.bunch.recovery(flight).map_or(0.0, |activity| seats(activity, class)),
            Usage::Transfer { pair, amount } => -amount * candidate.bunch.activities().iter().map(|activity| {
                f64::from(same_recovery(activity, &pair.0)) + f64::from(same_recovery(activity, &pair.1))
            }).sum::<f64>(),
            Usage::Enabled { activity, amount } => amount * f64::from(candidate.bunch.activities().iter().any(|selected| same_recovery(selected, activity))),
            Usage::Allowed { activities, amount } => amount * f64::from(candidate.bunch.activities().iter()
                .any(|selected| activities.iter().any(|allowed| same_recovery(selected, allowed)))),
        }
    }
}

/// 不可变旅客恢复定义，不持有模型或变量 token。 / Immutable passenger recovery definition without model or variable tokens.
#[derive(Debug, Clone)]
pub struct RecoveryPassengerRecovery {
    legs: Vec<Leg>,
    decisions: Vec<Decision>,
    rows: Vec<Row>,
    connections: Vec<Connection>,
}

impl RecoveryPassengerRecovery {
    /// 校验全部原始引用、人数和费用，再准备动态容量与转机行。 / Validate all original references, counts and costs before preparing dynamic capacity and transfer rows.
    pub fn new(
        configuration: RecoveryPassengerConfiguration,
        tasks: &[FlightTaskImpl],
        variants: &[FlightTaskRecovery],
    ) -> GanttResult<Self> {
        let task_ids = tasks.iter().map(|task| task.plan.id.as_str()).collect::<HashSet<_>>();
        if task_ids.len() != tasks.len() || task_ids.contains("")
            || variants.iter().any(|activity| !task_ids.contains(activity.original().plan.id.as_str()))
        { return Err(invalid("unknown or duplicate passenger task definition")); }
        passenger_rows(&task_ids, variants, &configuration.passengers)?;
        let mut policies = BTreeMap::new();
        for policy in &configuration.policies {
            if policies.insert((policy.passenger.as_str(), policy.flight.as_str()), policy).is_some() {
                return Err(invalid("duplicate passenger leg policy"));
            }
        }
        let task_map = tasks.iter().map(|task| (task.plan.id.as_str(), task)).collect::<BTreeMap<_, _>>();
        let mut result = Self { legs: vec![], decisions: vec![], rows: vec![], connections: vec![] };
        for passenger in &configuration.passengers {
            if passenger.amount > (1_u64 << 53) { return Err(invalid("passenger count exceeds exact solver range")); }
            let mut previous = None;
            for (flight, class) in &passenger.flights {
                let policy = policies.remove(&(passenger.id.as_str(), flight.as_str())).ok_or_else(|| invalid("missing passenger leg policy"))?;
                let mut add = |cost: f64, bound: u64| -> GanttResult<usize> {
                    if !cost.is_finite() || cost < 0.0 || !(cost * bound as f64).is_finite() { return Err(invalid("invalid passenger cost")); }
                    let index = result.decisions.len();
                    result.decisions.push(Decision { bound: bound as f64, cost });
                    Ok(index)
                };
                let cancel = add(policy.cancellation_cost.unwrap_or(0.0), if policy.cancellation_cost.is_some() { passenger.amount } else { 0 })?;
                let mut changes = BTreeMap::new();
                for (target, cost) in &policy.class_change_costs {
                    if target.is_empty() || target == &class.0 { return Err(invalid("invalid destination cabin")); }
                    changes.insert(target.clone(), add(*cost, passenger.amount)?);
                }
                let mut flights = BTreeMap::new();
                for ((target, cabin), cost) in &policy.flight_change_costs {
                    let original = task_map[flight.as_str()];
                    let target_task = task_map.get(target.as_str()).ok_or_else(|| invalid("unknown rebooking flight"))?;
                    if target == flight || cabin.is_empty() || !original.task_type.category.is_flight_type()
                        || !target_task.task_type.category.is_flight_type()
                        || original.plan.dep != target_task.plan.dep || original.plan.arr != target_task.plan.arr
                    { return Err(invalid("rebooking requires another flight with the same original OD and a nonempty cabin")); }
                    flights.insert((target.clone(), cabin.clone()), add(*cost, passenger.amount)?);
                }
                let index = result.legs.len();
                result.legs.push(Leg { passenger: passenger.id.clone(), flight: flight.clone(), class: class.0.clone(),
                    route: Route { dep: task_map[flight.as_str()].plan.dep.clone(), arr: task_map[flight.as_str()].plan.arr.clone() },
                    amount: passenger.amount, cancel, changes, flights, previous });
                previous = Some(index);
            }
        }
        if !policies.is_empty() { return Err(invalid("unknown passenger leg policy")); }
        if !result.decisions.iter().map(|decision| decision.cost * decision.bound).sum::<f64>().is_finite() {
            return Err(invalid("nonfinite aggregate passenger cost envelope"));
        }
        let mut capacities = BTreeMap::<(String, String), (f64, Vec<(usize, f64)>)>::new();
        for (index, leg) in result.legs.iter().enumerate() {
            let mut terms = vec![(leg.cancel, -1.0)];
            terms.extend(leg.changes.values().map(|variable| (*variable, -1.0)));
            terms.extend(leg.flights.values().map(|variable| (*variable, -1.0)));
            result.rows.push(Row { name: format!("recovery_passenger_disposition_{index}"), usage: Usage::None, terms, bound: -(leg.amount as f64) });
            let original = capacities.entry((leg.flight.clone(), leg.class.clone())).or_default();
            original.0 += leg.amount as f64;
            if original.0 > (1_u64 << 53) as f64 { return Err(invalid("aggregate passenger count exceeds exact solver range")); }
            // 取消只减少原舱位，改签同时减少原舱位并增加目标舱位。 / Cancellation reduces only the original cabin; a change debits the original and credits its destination.
            original.1.push((leg.cancel, 1.0));
            original.1.extend(leg.changes.values().map(|variable| (*variable, 1.0)));
            original.1.extend(leg.flights.values().map(|variable| (*variable, 1.0)));
            for (class, variable) in &leg.changes {
                capacities.entry((leg.flight.clone(), class.clone())).or_default().1.push((*variable, -1.0));
            }
            for ((flight, class), variable) in &leg.flights {
                capacities.entry((flight.clone(), class.clone())).or_default().1.push((*variable, -1.0));
            }
            if let Some(previous) = leg.previous {
                let previous = &result.legs[previous];
                result.rows.push(Row { name: format!("recovery_passenger_propagation_{index}"), usage: Usage::None,
                    terms: vec![(leg.cancel, 1.0), (previous.cancel, -1.0)], bound: 0.0 });
                if previous.flights.is_empty() && leg.flights.is_empty() {
                    for (left_index, left) in variants.iter().enumerate().filter(|(_, activity)| activity.original().plan.id == previous.flight) {
                        for (right_index, right) in variants.iter().enumerate().filter(|(_, activity)| activity.original().plan.id == leg.flight) {
                            if !compatible(left, right) {
                                result.rows.push(Row { name: format!("recovery_passenger_recovery_transfer_{index}_{left_index}_{right_index}"),
                                    usage: Usage::Transfer { pair: Box::new((left.clone(), right.clone())), amount: leg.amount as f64 },
                                    terms: vec![(leg.cancel, 1.0)], bound: -(leg.amount as f64) });
                            }
                        }
                    }
                }
            }
        }
        for (index, ((flight, class), (bound, terms))) in capacities.into_iter().enumerate() {
            result.rows.push(Row { name: format!("recovery_passenger_recovery_capacity_{index}"), usage: Usage::Capacity { flight, class }, terms, bound });
        }
        result.prepare_connections(variants)?;
        Ok(result)
    }

    /// 全部声明行，包括定价系数为零的辅助行。 / All declared rows, including auxiliary rows with zero pricing coefficients.
    pub fn row_names(&self) -> impl Iterator<Item = String> + '_ { self.rows.iter().map(|row| row.name.clone()) }

    /// 完整候选的同源列系数，显式保留零。 / Shared full-candidate column coefficients retaining explicit zeros.
    pub fn column_coefficients(&self, candidate: &RecoveryBunchCandidate) -> BTreeMap<String, f64> {
        self.rows.iter().map(|row| (row.name.clone(), row.coefficient(candidate))).collect()
    }

    fn evaluate(&self, selected: &[RecoveryBunchCandidate], values: &[f64]) -> GanttResult<Vec<RecoveryPassengerLegResult>> {
        if values.len() != self.decisions.len() { return Err(invalid("missing passenger decision")); }
        let values = values.iter().zip(&self.decisions).map(|(value, decision)| {
            if !value.is_finite() || value.round() < 0.0 || value.round() > decision.bound || (value - value.round()).abs() > 1e-6 {
                Err(invalid("nonfinite, fractional or out-of-range passenger decision"))
            } else { Ok(value.round() as u64) }
        }).collect::<GanttResult<Vec<_>>>()?;
        let mut activities = BTreeMap::new();
        let mut aircraft = HashSet::new();
        for candidate in selected {
            if !aircraft.insert(&candidate.bunch.aircraft().reg_no.0) { return Err(invalid("multiple selected bunches per aircraft")); }
            for activity in candidate.bunch.activities() {
                if activities.insert(activity.original().plan.id.as_str(), activity).is_some() { return Err(invalid("duplicate passenger flight")); }
            }
        }
        let mut demand = BTreeMap::<(String, String), u64>::new();
        let mut results = Vec::new();
        for leg in &self.legs {
            let canceled = values[leg.cancel];
            let changed = leg.changes.values().try_fold(0_u64, |sum, variable| sum.checked_add(values[*variable]).ok_or_else(|| invalid("passenger change overflow")))?;
            let rebooked = leg.flights.values().try_fold(0_u64, |sum, variable| sum.checked_add(values[*variable]).ok_or_else(|| invalid("passenger rebooking overflow")))?;
            let original = leg.amount.checked_sub(canceled).and_then(|remaining| remaining.checked_sub(changed))
                .and_then(|remaining| remaining.checked_sub(rebooked)).ok_or_else(|| invalid("passenger over-disposition"))?;
            let class_changes = leg.changes.iter().map(|(class, variable)| (class.clone(), values[*variable])).collect::<BTreeMap<_, _>>();
            let mut transported = class_changes.clone();
            transported.insert(leg.class.clone(), original);
            let flight_changes = leg.flights.iter().map(|(key, variable)| (key.clone(), values[*variable])).collect::<BTreeMap<_, _>>();
            for (class, amount) in &transported {
                let total = demand.entry((leg.flight.clone(), class.clone())).or_default();
                *total = total.checked_add(*amount).ok_or_else(|| invalid("passenger seat demand overflow"))?;
            }
            for ((flight, class), amount) in &flight_changes {
                let total = demand.entry((flight.clone(), class.clone())).or_default();
                *total = total.checked_add(*amount).ok_or_else(|| invalid("rebooked seat demand overflow"))?;
            }
            if let Some(previous) = leg.previous {
                let previous = &self.legs[previous];
                if values[previous.cancel] > canceled { return Err(invalid("passenger cancellation does not propagate")); }
                if canceled < leg.amount && previous.flights.is_empty() && leg.flights.is_empty() {
                    let left = activities.get(previous.flight.as_str()).ok_or_else(|| invalid("missing incoming passenger flight"))?;
                    let right = activities.get(leg.flight.as_str()).ok_or_else(|| invalid("missing outgoing passenger flight"))?;
                    if !compatible(left, right) { return Err(invalid("infeasible recovered passenger connection")); }
                }
            }
            let cancellation_penalty = canceled as f64 * self.decisions[leg.cancel].cost;
            let class_change_penalty = leg.changes.values().map(|variable| values[*variable] as f64 * self.decisions[*variable].cost).sum::<f64>();
            let flight_change_penalty = leg.flights.values().map(|variable| values[*variable] as f64 * self.decisions[*variable].cost).sum::<f64>();
            if !(cancellation_penalty + class_change_penalty + flight_change_penalty).is_finite() { return Err(invalid("nonfinite passenger penalty")); }
            results.push(RecoveryPassengerLegResult { passenger: leg.passenger.clone(), flight: leg.flight.clone(), canceled,
                class_changes, transported, flight_changes, connections: BTreeMap::new(), cancellation_penalty, class_change_penalty, flight_change_penalty });
        }
        self.check_connections(&activities, &values, &mut results)?;
        for ((flight, class), amount) in demand {
            let capacity = activities.get(flight.as_str()).map_or(0.0, |activity| seats(activity, &class));
            if amount > capacity as u64 { return Err(invalid("passenger demand exceeds actual selected cabin capacity")); }
        }
        if !results.iter().map(|result| result.cancellation_penalty + result.class_change_penalty + result.flight_change_penalty).sum::<f64>().is_finite() {
            return Err(invalid("nonfinite aggregate passenger penalty"));
        }
        Ok(results)
    }

    pub(crate) fn validate_results(&self, selected: &[RecoveryBunchCandidate], results: &[RecoveryPassengerLegResult]) -> GanttResult<()> {
        if results.len() != self.legs.len() { return Err(invalid("missing passenger extraction")); }
        let mut values = vec![0.0; self.decisions.len()];
        for (leg, result) in self.legs.iter().zip(results) {
            values[leg.cancel] = result.canceled as f64;
            if result.class_changes.len() != leg.changes.len() { return Err(invalid("unexpected passenger cabin extraction")); }
            for (class, variable) in &leg.changes {
                values[*variable] = *result.class_changes.get(class).ok_or_else(|| invalid("missing passenger cabin extraction"))? as f64;
            }
            if result.flight_changes.len() != leg.flights.len() { return Err(invalid("unexpected passenger flight extraction")); }
            for (key, variable) in &leg.flights {
                values[*variable] = *result.flight_changes.get(key).ok_or_else(|| invalid("missing passenger flight extraction"))? as f64;
            }
        }
        self.replay_connections(selected, results, &mut values)?;
        if self.evaluate(selected, &values)? != results { return Err(invalid("passenger extraction differs from actual recoveries")); }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RecoveryPassengerComponent {
    pub definition: Option<RecoveryPassengerRecovery>,
    group: Option<Arc<ConstraintGroup>>,
    variables: Vec<usize>,
    objective_name: String,
}

impl RecoveryPassengerComponent {
    pub fn refresh(&mut self, columns: &[(usize, RecoveryBunchCandidate)], model: &mut MetaModel<f64>) -> GanttResult<()> {
        let Some(definition) = &self.definition else { return Ok(()); };
        if definition.rows.is_empty() { return Ok(()); }
        if self.group.is_none() {
            let id = next_gantt_symbol_id();
            self.group = Some(model.create_constraint_group(id, "recovery_passenger_recovery").map_err(invalid)?);
            self.objective_name = format!("recovery_passenger_recovery_cost_{id}");
            for (index, decision) in definition.decisions.iter().enumerate() {
                self.variables.push(model.register_auto_variable_with_range::<UInteger>(&format!("recovery_passenger_decision_{index}"),
                    VariableRange::bounded(0.0, decision.bound)).map_err(invalid)?);
            }
        }
        let group = self.group.clone().ok_or_else(|| invalid("missing passenger recovery group"))?;
        model.remove_constraints_by_group_id(group.id);
        for row in &definition.rows {
            let mut terms = row.terms.iter().map(|(index, coefficient)| LinearMonomial::new(*coefficient, self.variables[*index])).collect::<Vec<_>>();
            terms.extend(columns.iter().map(|(variable, candidate)| LinearMonomial::new(row.coefficient(candidate), *variable)));
            model.add_inequality_with_metadata(LinearInequality::greater_equal(Linear::new(terms, 0.0), row.bound),
                &row.name, Some(group.clone()), false, 0, None).map_err(invalid)?;
        }
        let objective = definition.decisions.iter().zip(&self.variables).map(|(decision, variable)| LinearMonomial::new(decision.cost, *variable)).collect();
        model.replace_sub_objective(&self.objective_name, SubObjective::minimize(Linear::new(objective, 0.0), &self.objective_name));
        Ok(())
    }

    pub fn extract(&self, selected: &[RecoveryBunchCandidate], values: &[f64]) -> GanttResult<Vec<RecoveryPassengerLegResult>> {
        let Some(definition) = &self.definition else { return Ok(vec![]); };
        let values = self.variables.iter().map(|variable| values.get(*variable).copied().ok_or_else(|| invalid("missing passenger incumbent"))).collect::<GanttResult<Vec<_>>>()?;
        definition.evaluate(selected, &values)
    }
}

fn compatible(left: &FlightTaskRecovery, right: &FlightTaskRecovery) -> bool {
    left.route().arr == right.route().dep && right.time().0 - left.time().1 >= left.route().arr.passenger_transfer_time
}

fn seats(activity: &FlightTaskRecovery, class: &str) -> f64 {
    match &activity.aircraft().capacity {
        AircraftCapacity::Passenger { capacities } => capacities.iter().filter(|(key, _)| key.0 == class).map(|(_, value)| *value as f64).sum(),
        AircraftCapacity::Cargo { .. } => 0.0,
    }
}

fn invalid(error: impl std::fmt::Display) -> GanttError {
    GanttError::Calculation { message: format!("非法旅客恢复输入或结果 / invalid passenger recovery input or result: {error}") }
}
