//! 有限机场流控的共享列系数、容量行及生产提取。 / Shared column coefficients, capacity rows and production extraction for finite airport flow controls.

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
use crate::framework::demo4::domain::bunch_generation::problem::RecoveryAircraftDomain;
use crate::framework::demo4::domain::bunch_generation::recovery::RecoveryBunchCandidate;
use crate::framework::demo4::domain::rule::model::{FlowControl, FlowControlSlot};
use crate::framework::demo4::domain::task::model::{FlightTaskImpl, FlightTaskRecovery};

#[derive(Debug, Clone)]
struct CapacitySlot {
    control: usize,
    ordinal: usize,
    time: FlowControlSlot,
    maximum_overage: u64,
}

/// 不可变的有限容量窗与实际已执行前序，不包含模型注册态。 / Immutable finite capacity windows and actual executed predecessors without model registration state.
#[derive(Debug, Clone, Default)]
pub struct RecoveryFlowControls {
    controls: Vec<FlowControl>,
    slots: Vec<CapacitySlot>,
    predecessors: BTreeMap<String, Option<FlightTaskRecovery>>,
}

/// 当前调度的容量使用及实际超额变量，不从任务 ID 猜测事件。 / Capacity usage and actual overage variables of the current schedule, without guessing events from task IDs.
#[derive(Debug, Clone, PartialEq)]
pub struct RecoveryFlowControlResult {
    /// 原流控身份。 / Original control identity.
    pub control: String,
    /// 在该规则中的容量窗序号。 / Capacity-window ordinal within the rule.
    pub slot_index: usize,
    /// 实际容量窗及端点归属。 / Actual capacity window and endpoint ownership.
    pub time: FlowControlSlot,
    /// 固定外部已使用量。 / Fixed external usage.
    pub initial_usage: u64,
    /// 所选完整恢复束贡献的使用量。 / Usage contributed by selected complete recovery bunches.
    pub recovery_usage: u64,
    /// 外部与恢复使用量之和。 / Sum of external and recovery usage.
    pub usage: u64,
    /// 声明的每窗容量。 / Declared per-window capacity.
    pub capacity: u64,
    /// 从实际调度重算的物理超额。 / Physical excess recomputed from the actual schedule.
    pub overage: u64,
    /// 真实求解器超额变量，硬容量没有该变量。 / Actual solver overage variable, absent for hard capacities.
    pub modeled_overage: Option<u64>,
    /// 按真实超额变量计算的费用，允许限时解有非最小松弛。 / Cost of actual modeled overage, permitting nonminimal slack in a time-limited incumbent.
    pub penalty: f64,
}

impl RecoveryFlowControls {
    pub(crate) fn new(
        controls: Vec<FlowControl>,
        tasks: &[FlightTaskImpl],
        domains: &[RecoveryAircraftDomain],
    ) -> GanttResult<Self> {
        let maximum_usage = u64::try_from(tasks.len()).map_err(invalid)?;
        if maximum_usage > (1_u64 << 53) { return Err(invalid("recovery event count exceeds exact solver integers")); }
        let mut airports = tasks.iter().flat_map(|task| [task.dep().icao.0.as_str(), task.arr().icao.0.as_str()]).collect::<HashSet<_>>();
        for domain in domains {
            airports.insert(domain.usability.location.icao.0.as_str());
            for recovery in domain.previous.iter().chain(domain.candidates.iter().map(|candidate| &candidate.recovery)) {
                airports.extend([recovery.route().dep.icao.0.as_str(), recovery.route().arr.icao.0.as_str()]);
            }
        }
        let minor_types = domains.iter().map(|domain| domain.aircraft.minor_type.code.0.as_str()).collect::<HashSet<_>>();
        let predecessors = domains.iter().map(|domain| (domain.aircraft.reg_no.0.clone(), domain.previous.clone())).collect::<BTreeMap<_, _>>();
        if predecessors.len() != domains.len() { return Err(invalid("duplicate flow-control aircraft")); }
        let mut ids = HashSet::new();
        let mut slots = Vec::new();
        let mut maximum_cost = 0.0;
        for (index, control) in controls.iter().enumerate() {
            if !ids.insert(&control.id) || !airports.contains(control.airport.as_str())
                || control.condition.as_ref().is_some_and(|condition| condition.aircraft_minor_types.iter().any(|minor| !minor_types.contains(minor.as_str())))
            { return Err(invalid("unknown or repeated flow-control reference")); }
            let times = control.slots().map_err(invalid)?;
            let maximum_total = maximum_usage.checked_add(control.capacity.current_usage).filter(|value| *value <= (1_u64 << 53))
                .ok_or_else(|| invalid("flow-control total usage exceeds exact solver integers"))?;
            let maximum_overage = maximum_total.saturating_sub(control.capacity.max_capacity);
            if control.over_cost.is_some_and(|cost| !(cost * maximum_overage as f64).is_finite()) {
                return Err(invalid("non-finite bounded flow-control cost"));
            }
            maximum_cost += control.over_cost.unwrap_or(0.0) * maximum_overage as f64 * times.len() as f64;
            if !maximum_cost.is_finite() { return Err(invalid("non-finite total bounded flow-control cost")); }
            slots.extend(times.into_iter().enumerate().map(|(ordinal, time)| CapacitySlot { control: index, ordinal, time, maximum_overage }));
        }
        Ok(Self { controls, slots, predecessors })
    }

    /// 只读流控业务目录，注册状态由每个 context 独占。 / Read-only control directory with registration state exclusively owned by each context.
    pub fn controls(&self) -> &[FlowControl] { &self.controls }

    /// 与主问题完全同源的列系数，含显式零和内部机场事件。 / Column coefficients shared exactly with the master, including explicit zeros and internal-airport events.
    pub fn column_coefficients(&self, candidate: &RecoveryBunchCandidate) -> GanttResult<BTreeMap<String, f64>> {
        if self.slots.is_empty() { return Ok(BTreeMap::new()); }
        let previous = self.predecessors.get(&candidate.bunch.aircraft().reg_no.0).ok_or_else(|| invalid("unknown flow-control executor"))?;
        Ok(self.slots.iter().enumerate().map(|(index, slot)| {
            (row_name(index), self.controls[slot.control].usage_in_slot(&candidate.bunch, previous.as_ref(), &slot.time) as f64)
        }).collect())
    }

    pub(crate) fn row_names(&self) -> impl Iterator<Item = String> + '_ { (0..self.slots.len()).map(row_name) }

    fn evaluate(&self, selected: &[RecoveryBunchCandidate], modeled: &[Option<f64>]) -> GanttResult<Vec<RecoveryFlowControlResult>> {
        if modeled.len() != self.slots.len() { return Err(invalid("missing flow-control overage mapping")); }
        if self.slots.is_empty() { return Ok(vec![]); }
        let mut executors = HashSet::new();
        let mut tasks = HashSet::new();
        for candidate in selected {
            if !executors.insert(&candidate.bunch.aircraft().reg_no.0) || candidate.bunch.task_ids().any(|task| !tasks.insert(task)) {
                return Err(invalid("duplicate flow-control executor or selected task"));
            }
        }
        let coefficients = selected.iter().map(|candidate| self.column_coefficients(candidate)).collect::<GanttResult<Vec<_>>>()?;
        self.slots.iter().zip(modeled).enumerate().map(|(index, (slot, value))| {
            let control = &self.controls[slot.control];
            let mut recovery_usage = 0_u64;
            for rows in &coefficients {
                recovery_usage = recovery_usage.checked_add(rows[&row_name(index)] as u64).ok_or_else(|| invalid("flow-control recovery usage overflow"))?;
            }
            let usage = recovery_usage.checked_add(control.capacity.current_usage).filter(|value| *value <= (1_u64 << 53))
                .ok_or_else(|| invalid("flow-control total usage overflow"))?;
            let overage = usage.saturating_sub(control.capacity.max_capacity);
            let (modeled_overage, penalty) = match (control.over_cost, value) {
                (None, None) if overage == 0 => (None, 0.0),
                (Some(cost), Some(value)) if value.is_finite() && value.round() >= 0.0 && value.round() <= slot.maximum_overage as f64
                    && (*value - value.round()).abs() <= 1e-6 && *value + 1e-6 >= overage as f64 =>
                {
                    let penalty = cost * value.round();
                    if !penalty.is_finite() { return Err(invalid("non-finite extracted flow-control cost")); }
                    (Some(value.round() as u64), penalty)
                }
                _ => return Err(invalid("hard flow-control violation or illegal overage incumbent")),
            };
            Ok(RecoveryFlowControlResult { control: control.id.clone(), slot_index: slot.ordinal, time: slot.time,
                initial_usage: control.capacity.current_usage, recovery_usage, usage, capacity: control.capacity.max_capacity,
                overage, modeled_overage, penalty })
        }).collect()
    }

    pub(crate) fn validate_results(&self, selected: &[RecoveryBunchCandidate], results: &[RecoveryFlowControlResult]) -> GanttResult<()> {
        let modeled = results.iter().map(|result| result.modeled_overage.map(|value| value as f64)).collect::<Vec<_>>();
        if self.evaluate(selected, &modeled)? != results { return Err(invalid("flow-control results differ from full actual events")); }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct RecoveryFlowControlComponent {
    pub definition: Option<RecoveryFlowControls>,
    group: Option<Arc<ConstraintGroup>>,
    overages: BTreeMap<usize, usize>,
    objective_name: String,
}

impl RecoveryFlowControlComponent {
    pub fn refresh(&mut self, columns: &[(usize, RecoveryBunchCandidate)], model: &mut MetaModel<f64>) -> GanttResult<()> {
        let Some(definition) = &self.definition else { return Ok(()); };
        if definition.slots.is_empty() { return Ok(()); }
        if self.group.is_none() {
            let id = next_gantt_symbol_id();
            self.group = Some(model.create_constraint_group(id, "recovery_flow_controls").map_err(invalid)?);
            self.objective_name = format!("recovery_flow_control_cost_{id}");
            for (index, slot) in definition.slots.iter().enumerate() {
                if definition.controls[slot.control].over_cost.is_some() {
                    let variable = model.register_auto_variable_with_range::<UInteger>(&format!("recovery_flow_overage_{index}"),
                        VariableRange::bounded(0.0, slot.maximum_overage as f64)).map_err(invalid)?;
                    self.overages.insert(index, variable);
                }
            }
        }
        let group = self.group.clone().ok_or_else(|| invalid("missing flow-control group"))?;
        let coefficients = columns.iter().map(|(variable, candidate)| Ok((*variable, definition.column_coefficients(candidate)?))).collect::<GanttResult<Vec<_>>>()?;
        model.remove_constraints_by_group_id(group.id);
        let mut objective = Vec::new();
        for (index, slot) in definition.slots.iter().enumerate() {
            let control = &definition.controls[slot.control];
            let name = row_name(index);
            let mut terms = coefficients.iter().map(|(variable, rows)| LinearMonomial::new(rows[&name], *variable)).collect::<Vec<_>>();
            if let Some(variable) = self.overages.get(&index) {
                terms.push(LinearMonomial::new(-1.0, *variable));
                objective.push(LinearMonomial::new(control.over_cost.ok_or_else(|| invalid("unmapped flow-control cost"))?, *variable));
            }
            model.add_inequality_with_metadata(LinearInequality::less_equal(Linear::new(terms, control.capacity.current_usage as f64), control.capacity.max_capacity as f64),
                &name, Some(group.clone()), false, 0, None).map_err(invalid)?;
        }
        model.replace_sub_objective(&self.objective_name, SubObjective::minimize(Linear::new(objective, 0.0), &self.objective_name));
        Ok(())
    }

    pub fn extract(&self, selected: &[RecoveryBunchCandidate], values: &[f64]) -> GanttResult<Vec<RecoveryFlowControlResult>> {
        let Some(definition) = &self.definition else { return Ok(vec![]); };
        let modeled = definition.slots.iter().enumerate().map(|(index, slot)| {
            if definition.controls[slot.control].over_cost.is_some() {
                self.overages.get(&index).and_then(|variable| values.get(*variable)).copied().map(Some).ok_or_else(|| invalid("missing flow-control overage incumbent"))
            } else { Ok(None) }
        }).collect::<GanttResult<Vec<_>>>()?;
        definition.evaluate(selected, &modeled)
    }
}

fn row_name(index: usize) -> String { format!("recovery_flow_control_{index}") }

fn invalid(error: impl std::fmt::Display) -> GanttError {
    GanttError::Calculation { message: format!("非法恢复流控输入或结果 / invalid recovery flow-control input or result: {error}") }
}
