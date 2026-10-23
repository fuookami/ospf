//! 链接拆分与飞机闲置费用；定义可共享，注册态属于单个 context。 / Link-split and aircraft-idle costs; definitions are shareable, registration belongs to one context.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use ospf_rust_core::model::MetaModel;
use ospf_rust_core::model::flatten::{Linear, LinearMonomial};
use ospf_rust_core::model::mechanism::constraint::LinearInequality;
use ospf_rust_core::model::mechanism::constraint_group::ConstraintGroup;
use ospf_rust_core::model::object::SubObjective;
use ospf_rust_core::variable::{Binary, UContinuous, VariableRange};
use ospf_rust_framework_gantt_scheduling::{GanttError, GanttResult};
use ospf_rust_framework_gantt_scheduling::domain::task_compilation::adapter::next_gantt_symbol_id;
use crate::framework::demo4::domain::bunch_generation::recovery::RecoveryBunchCandidate;
use crate::framework::demo4::domain::rule::model::Link;

/// 只读运行费用定义，不包含变量、约束或模型身份。 / Read-only operating cost definition without variables, constraints or model identity.
#[derive(Debug, Clone, Default)]
pub struct RecoveryOperatingCosts {
    links: Vec<Link>,
    idle_costs: BTreeMap<String, f64>,
}

/// 一条软链接的物理损失及实际模型费用。 / Physical loss and actual modeled cost of one soft link.
#[derive(Debug, Clone, PartialEq)]
pub struct RecoveryLinkSplitResult {
    /// 原始前序任务。 / Original predecessor task.
    pub from_task: String,
    /// 原始后继任务。 / Original successor task.
    pub to_task: String,
    /// 物理损失，为 0、0.5 或 1。 / Physical loss, either 0, 0.5 or 1.
    pub loss: f64,
    /// 实际有界松弛，限时解可以大于最小损失。 / Actual bounded slack, possibly exceeding minimum loss in a time-limited incumbent.
    pub modeled_loss: f64,
    /// 实际松弛费用。 / Actual slack cost.
    pub penalty: f64,
}

/// 一架已配置飞机的实际闲置状态及费用。 / Actual idle state and cost of a configured aircraft.
#[derive(Debug, Clone, PartialEq)]
pub struct RecoveryAircraftIdleResult {
    /// 飞机注册号。 / Aircraft registration.
    pub aircraft: String,
    /// 未执行任何待恢复活动，选中空束仍算闲置。 / No recovery activities executed; selecting an empty bunch still counts as idle.
    pub idle: bool,
    /// 实际闲置费用。 / Actual idle cost.
    pub penalty: f64,
}

/// 从完整恢复记录与真实模型辅助变量复核的运行费用。 / Operating costs checked against full recoveries and actual model auxiliary variables.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RecoveryOperatingCostResult {
    /// 软链接结果，保持声明顺序。 / Soft-link results in declaration order.
    pub links: Vec<RecoveryLinkSplitResult>,
    /// 闲置结果，按飞机注册号排序。 / Idle results ordered by aircraft registration.
    pub aircraft: Vec<RecoveryAircraftIdleResult>,
}

impl RecoveryOperatingCosts {
    pub(crate) fn new(links: &[Link]) -> Self {
        Self { links: links.iter().filter(|link| link.split_cost.is_some()).cloned().collect(), idle_costs: BTreeMap::new() }
    }

    pub(crate) fn configure_idle(&mut self, costs: BTreeMap<String, f64>, aircraft: &HashSet<&str>) -> GanttResult<()> {
        if costs.iter().any(|(executor, cost)| !aircraft.contains(executor.as_str()) || !cost.is_finite() || *cost < 0.0) {
            return Err(invalid("unknown aircraft or invalid idle cost"));
        }
        self.idle_costs = costs;
        Ok(())
    }

    /// 全部费用行的完整列系数，零项显式保留。 / Complete column coefficients for all cost rows, retaining explicit zeros.
    pub fn column_coefficients(&self, candidate: &RecoveryBunchCandidate) -> BTreeMap<String, f64> {
        let mut rows = self.links.iter().enumerate().map(|(index, link)| {
            (link_row(index), f64::from(candidate.bunch.consecutive(&link.from_task, &link.to_task)))
        }).collect::<BTreeMap<_, _>>();
        rows.extend(self.idle_costs.keys().map(|aircraft| {
            (idle_row(aircraft), f64::from(&candidate.bunch.aircraft().reg_no.0 == aircraft && !candidate.bunch.activities().is_empty()))
        }));
        rows
    }

    pub(crate) fn row_names(&self) -> impl Iterator<Item = String> + '_ {
        (0..self.links.len()).map(link_row).chain(self.idle_costs.keys().map(|aircraft| idle_row(aircraft)))
    }

    fn evaluate(
        &self,
        selected: &[RecoveryBunchCandidate],
        canceled: &HashSet<&str>,
        modeled: &[f64],
        idle: &BTreeMap<String, f64>,
    ) -> GanttResult<RecoveryOperatingCostResult> {
        if modeled.len() != self.links.len() || idle.len() != self.idle_costs.len() {
            return Err(invalid("missing operating-cost mapping"));
        }
        let links = self.links.iter().zip(modeled).map(|(link, value)| {
            let kept = selected.iter().any(|candidate| candidate.bunch.consecutive(&link.from_task, &link.to_task));
            let loss = (1.0 - f64::from(kept) - 0.5 * f64::from(canceled.contains(link.from_task.as_str()))
                - 0.5 * f64::from(canceled.contains(link.to_task.as_str()))).max(0.0);
            if !value.is_finite() || *value < -1e-6 || *value > 1.0 + 1e-6 || *value + 1e-6 < loss {
                return Err(invalid("invalid or insufficient link-split incumbent"));
            }
            let modeled_loss = value.clamp(0.0, 1.0);
            let penalty = modeled_loss * link.split_cost.ok_or_else(|| invalid("missing split cost"))?;
            if !penalty.is_finite() { return Err(invalid("non-finite link-split penalty")); }
            Ok(RecoveryLinkSplitResult { from_task: link.from_task.clone(), to_task: link.to_task.clone(), loss, modeled_loss, penalty })
        }).collect::<GanttResult<Vec<_>>>()?;
        let aircraft = self.idle_costs.iter().map(|(aircraft, cost)| {
            let expected = !selected.iter().any(|candidate| &candidate.bunch.aircraft().reg_no.0 == aircraft && !candidate.bunch.activities().is_empty());
            let value = idle.get(aircraft).ok_or_else(|| invalid("missing idle incumbent"))?;
            if !value.is_finite() || (*value - f64::from(expected)).abs() > 1e-6 { return Err(invalid("invalid aircraft-idle incumbent")); }
            Ok(RecoveryAircraftIdleResult { aircraft: aircraft.clone(), idle: expected, penalty: f64::from(expected) * cost })
        }).collect::<GanttResult<Vec<_>>>()?;
        Ok(RecoveryOperatingCostResult { links, aircraft })
    }

    pub(crate) fn validate_results(
        &self,
        selected: &[RecoveryBunchCandidate],
        canceled: &HashSet<&str>,
        result: &RecoveryOperatingCostResult,
    ) -> GanttResult<()> {
        let modeled = result.links.iter().map(|link| link.modeled_loss).collect::<Vec<_>>();
        let idle = result.aircraft.iter().map(|aircraft| (aircraft.aircraft.clone(), f64::from(aircraft.idle))).collect();
        if self.evaluate(selected, canceled, &modeled, &idle)? != *result { return Err(invalid("operating-cost result differs from recoveries")); }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct RecoveryOperatingCostComponent {
    pub definition: Option<RecoveryOperatingCosts>,
    group: Option<Arc<ConstraintGroup>>,
    slacks: Vec<usize>,
    idle: BTreeMap<String, usize>,
    objective_name: String,
}

impl RecoveryOperatingCostComponent {
    pub fn refresh(
        &mut self,
        columns: &[(usize, RecoveryBunchCandidate)],
        cancellations: &BTreeMap<String, usize>,
        model: &mut MetaModel<f64>,
    ) -> GanttResult<()> {
        let Some(definition) = &self.definition else { return Ok(()); };
        if definition.links.is_empty() && definition.idle_costs.is_empty() { return Ok(()); }
        if self.group.is_none() {
            let id = next_gantt_symbol_id();
            self.group = Some(model.create_constraint_group(id, "recovery_operating_costs").map_err(invalid)?);
            self.objective_name = format!("recovery_operating_costs_{id}");
            for index in 0..definition.links.len() {
                self.slacks.push(model.register_auto_variable_with_range::<UContinuous>(&format!("recovery_link_split_{index}"),
                    VariableRange::bounded(0.0, 1.0)).map_err(invalid)?);
            }
            for aircraft in definition.idle_costs.keys() {
                let variable = model.register_auto_variable_with_range::<Binary>(&format!("recovery_aircraft_idle_{aircraft}"),
                    VariableRange::bounded(0.0, 1.0)).map_err(invalid)?;
                self.idle.insert(aircraft.clone(), variable);
            }
        }
        let group = self.group.clone().ok_or_else(|| invalid("missing operating-cost group"))?;
        model.remove_constraints_by_group_id(group.id);
        let coefficients = columns.iter().map(|(variable, candidate)| (*variable, definition.column_coefficients(candidate))).collect::<Vec<_>>();
        let mut objective = Vec::new();
        for (index, link) in definition.links.iter().enumerate() {
            let name = link_row(index);
            let mut terms = coefficients.iter().map(|(variable, rows)| LinearMonomial::new(rows[&name], *variable)).collect::<Vec<_>>();
            for task in [&link.from_task, &link.to_task] {
                terms.push(LinearMonomial::new(0.5, *cancellations.get(task).ok_or_else(|| invalid("missing linked cancellation variable"))?));
            }
            terms.push(LinearMonomial::new(1.0, self.slacks[index]));
            model.add_inequality_with_metadata(LinearInequality::greater_equal(Linear::new(terms, 0.0), 1.0),
                &name, Some(group.clone()), false, 0, None).map_err(invalid)?;
            objective.push(LinearMonomial::new(link.split_cost.ok_or_else(|| invalid("missing split cost"))?, self.slacks[index]));
        }
        for (aircraft, cost) in &definition.idle_costs {
            let name = idle_row(aircraft);
            let mut terms = coefficients.iter().map(|(variable, rows)| LinearMonomial::new(rows[&name], *variable)).collect::<Vec<_>>();
            terms.push(LinearMonomial::new(1.0, self.idle[aircraft]));
            model.add_inequality_with_metadata(LinearInequality::equal(Linear::new(terms, 0.0), 1.0),
                &name, Some(group.clone()), false, 0, None).map_err(invalid)?;
            objective.push(LinearMonomial::new(*cost, self.idle[aircraft]));
        }
        model.replace_sub_objective(&self.objective_name, SubObjective::minimize(Linear::new(objective, 0.0), &self.objective_name));
        Ok(())
    }

    pub fn extract(
        &self,
        selected: &[RecoveryBunchCandidate],
        canceled: &HashSet<&str>,
        values: &[f64],
    ) -> GanttResult<RecoveryOperatingCostResult> {
        let Some(definition) = &self.definition else { return Ok(Default::default()); };
        let read = |variable: usize| values.get(variable).copied().ok_or_else(|| invalid("missing operating-cost incumbent"));
        let modeled = self.slacks.iter().map(|variable| read(*variable)).collect::<GanttResult<Vec<_>>>()?;
        let idle = self.idle.iter().map(|(aircraft, variable)| Ok((aircraft.clone(), read(*variable)?))).collect::<GanttResult<BTreeMap<_, _>>>()?;
        definition.evaluate(selected, canceled, &modeled, &idle)
    }
}

fn link_row(index: usize) -> String { format!("recovery_link_split_{index}") }

fn idle_row(aircraft: &str) -> String { format!("recovery_aircraft_idle_{aircraft}") }

fn invalid(error: impl std::fmt::Display) -> GanttError {
    GanttError::Calculation { message: format!("非法恢复运行费用输入或结果 / invalid recovery operating-cost input or result: {error}") }
}
